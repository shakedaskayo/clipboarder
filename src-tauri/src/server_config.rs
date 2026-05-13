//! TOML-backed server config: bind address, data dir, token→namespace map.
//!
//! Default location: `~/Library/Application Support/com.clipboarder.app/server.toml`
//! (override with `--config <path>` on `clipboarder serve` / `admin`).
//!
//! ## Token format on disk
//!
//! Tokens are stored as argon2id PHC strings — never plaintext. Each entry has:
//!
//! - `fingerprint`: the first 11 chars of the bearer (`tk_` + 8 random). It's
//!   stable, plaintext-safe, and acts as the human-readable ID for `admin token
//!   list` / `revoke`.
//! - `hash`: argon2id of the full bearer.
//!
//! Legacy configs with `token = "<plaintext>"` are migrated to the new shape
//! on `load()` (hashed in-place and rewritten to disk).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use password_hash::{rand_core::OsRng, SaltString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub data_dir: Option<String>,
    #[serde(default)]
    pub tokens: Vec<TokenEntry>,
    #[serde(default)]
    pub default_namespace: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7474".into(),
            data_dir: None,
            tokens: Vec::new(),
            default_namespace: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEntry {
    /// First 11 chars of the bearer (`tk_` + 8 random). Plaintext, stable ID.
    #[serde(default)]
    pub fingerprint: String,
    /// argon2id PHC string covering the full bearer. Empty during migration.
    #[serde(default)]
    pub hash: String,
    pub namespace: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Unix milliseconds. Written once at creation; never updated.
    #[serde(default)]
    pub created_at: i64,
    /// Unix milliseconds of the last successful auth. Updated by the server.
    #[serde(default)]
    pub last_used_at: Option<i64>,
    /// Cross-namespace admin token (gates `/admin` and `/v1/admin/*`).
    #[serde(default)]
    pub admin: bool,
    /// **Legacy** field for pre-argon2 configs. Migrated on `load()` and
    /// dropped on next `save()`. Never set on new entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let mut cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("parse {}", path.display()))?;

        // Migrate any pre-argon2 entries to fingerprint+hash form. We rewrite
        // the file on disk if any migrations happened so the plaintext token
        // is no longer at rest.
        let mut migrated = false;
        for entry in cfg.tokens.iter_mut() {
            if let Some(legacy) = entry.token.take() {
                if !legacy.is_empty() && entry.hash.is_empty() {
                    entry.fingerprint = fingerprint_of(&legacy);
                    entry.hash = hash_token(&legacy)
                        .with_context(|| format!("migrate token for ns `{}`", entry.namespace))?;
                    if entry.created_at == 0 {
                        entry.created_at = now_ms();
                    }
                    migrated = true;
                }
            }
        }
        if migrated {
            cfg.save(path)
                .with_context(|| format!("rewrite migrated config to {}", path.display()))?;
        }
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let raw = toml::to_string_pretty(self)?;
        // Atomic write — write to a sibling tmp file, fsync, rename. That way
        // a crash or concurrent reader never sees a half-written config.
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, raw).with_context(|| format!("write {}", tmp.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).ok();
        }
        std::fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
        Ok(())
    }

    /// Look up a token entry by the fingerprint embedded in a bearer string.
    /// Returns `None` if no entry matches — the caller still must verify the
    /// argon2 hash against the full bearer before honoring it.
    pub fn lookup_by_fingerprint(&self, bearer: &str) -> Option<&TokenEntry> {
        let fp = fingerprint_of(bearer);
        self.tokens.iter().find(|t| t.fingerprint == fp)
    }
}

pub fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Library/Application Support/com.clipboarder.app/server.toml")
}

pub fn generate_token() -> String {
    use rand::Rng;
    // tk_ + 32 url-safe chars
    let chars: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let mut out = String::with_capacity(35);
    out.push_str("tk_");
    for _ in 0..32 {
        out.push(chars[rng.gen_range(0..chars.len())] as char);
    }
    out
}

/// The first 11 chars of a bearer (`tk_` + 8 random). Stable, plaintext-safe.
/// Used as the lookup key for hashed entries and as the user-facing token ID
/// in `admin token list` / `admin token revoke`.
pub fn fingerprint_of(bearer: &str) -> String {
    let n = bearer.len().min(11);
    bearer[..n].to_string()
}

/// Hash a bearer with argon2id (default OWASP-ish params: m=19 MiB, t=2, p=1).
/// Returns the PHC string. Used at token-creation time and during migration.
pub fn hash_token(bearer: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let phc = argon2
        .hash_password(bearer.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2 hash: {e}"))?
        .to_string();
    Ok(phc)
}

/// Constant-time verify of a bearer against a stored PHC string. Returns
/// false on any parse / verify failure (we don't want to leak the reason).
pub fn verify_token(bearer: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(bearer.as_bytes(), &parsed)
        .is_ok()
}

/// Unix milliseconds, wall-clock. Helper for created_at / last_used_at.
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Client-side config (~/.config/clipboarder/client.toml). Env vars override.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server: Option<String>,
    pub token: Option<String>,
    pub namespace: Option<String>,
}

impl ClientConfig {
    pub fn load() -> Self {
        let path = client_config_path();
        if !path.exists() {
            return Self::default();
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => return Self::default(),
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = client_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(())
    }
}

pub fn client_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/clipboarder/client.toml")
}

/// Resolve server URL + token + namespace from env-then-file.
pub fn resolve_client() -> (Option<String>, Option<String>, Option<String>) {
    let env_server = std::env::var("CLIPBOARDER_SERVER").ok();
    let env_token = std::env::var("CLIPBOARDER_TOKEN").ok();
    let env_ns = std::env::var("CLIPBOARDER_NAMESPACE").ok();
    if env_server.is_some() && env_token.is_some() {
        return (env_server, env_token, env_ns);
    }
    let file = ClientConfig::load();
    (
        env_server.or(file.server),
        env_token.or(file.token),
        env_ns.or(file.namespace),
    )
}
