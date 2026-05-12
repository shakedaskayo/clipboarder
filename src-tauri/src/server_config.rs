//! TOML-backed server config: bind address, data dir, token→namespace map.
//!
//! Default location: `~/Library/Application Support/com.clipboarder.app/server.toml`
//! (override with `--config <path>` on `clipboarder serve` / `admin`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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
    pub token: String,
    pub namespace: String,
    #[serde(default)]
    pub label: Option<String>,
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("parse {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
        // Lock down perms so the file isn't world-readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(())
    }

    pub fn lookup_token(&self, token: &str) -> Option<&TokenEntry> {
        self.tokens.iter().find(|t| t.token == token)
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
