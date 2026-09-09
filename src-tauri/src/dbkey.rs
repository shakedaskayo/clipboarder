//! Where the SQLCipher database key comes from.
//!
//! Resolution order, first hit wins:
//!
//! 1. `CLIPBOARDER_DB_KEY` — 64 hex chars. Lets the integration tests, CI, and
//!    headless `clipboarder serve` deployments supply a key without touching a
//!    keychain that may not exist or may be locked.
//! 2. The macOS Keychain, under service `com.clipboarder.app`. This is the
//!    real path for the desktop app: the key is protected by the login
//!    keychain rather than sitting next to the data it protects.
//! 3. A `db.key` file in the app data directory, mode 0600. The fallback for
//!    non-macOS builds and for a Keychain that refuses (locked, no GUI
//!    session, sandboxed CI runner). Weaker — an attacker who can read the
//!    database can usually read the key beside it — but it keeps the database
//!    encrypted against backups, cloud sync, and disk images, which is where
//!    clipboard history most often escapes.
//!
//! The key is generated once, on first use, from the OS CSPRNG.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rand::RngCore;

const SERVICE: &str = "com.clipboarder.app";
const ACCOUNT: &str = "database-key";
const ENV_VAR: &str = "CLIPBOARDER_DB_KEY";
const KEY_FILE: &str = "db.key";

/// 32 bytes rendered as 64 lowercase hex chars.
fn generate() -> String {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

fn valid(key: &str) -> bool {
    key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit())
}

/// Fetch the database key, creating and persisting one on first run.
///
/// `data_dir` is the app data directory — only consulted for the file
/// fallback.
pub fn get_or_create(data_dir: &Path) -> Result<String> {
    if let Ok(from_env) = std::env::var(ENV_VAR) {
        let key = from_env.trim().to_ascii_lowercase();
        if !valid(&key) {
            return Err(anyhow!(
                "{ENV_VAR} must be 64 hex characters (32 bytes); got {} chars",
                key.len()
            ));
        }
        return Ok(key);
    }

    #[cfg(target_os = "macos")]
    if let Some(key) = keychain::get_or_create() {
        return Ok(key);
    }

    from_file(data_dir)
}

fn from_file(data_dir: &Path) -> Result<String> {
    let path = data_dir.join(KEY_FILE);
    if path.exists() {
        let key = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?
            .trim()
            .to_ascii_lowercase();
        if valid(&key) {
            return Ok(key);
        }
        return Err(anyhow!(
            "{} does not contain a 64-hex-character key; refusing to guess, \
             move it aside to start a fresh encrypted database",
            path.display()
        ));
    }

    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("create {}", data_dir.display()))?;
    let key = generate();
    std::fs::write(&path, &key).with_context(|| format!("write {}", path.display()))?;
    restrict(&path)?;
    Ok(key)
}

/// Narrow a file to owner-read/write. A key or a database that any process
/// running as another local user can open is not protected by being encrypted.
pub fn restrict(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("chmod 600 {}", path.display()))?;
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(target_os = "macos")]
mod keychain {
    use super::{generate, valid, ACCOUNT, SERVICE};
    use security_framework::passwords::{get_generic_password, set_generic_password};

    /// Read the key from the login keychain, creating it on first run.
    ///
    /// Returns `None` on any Keychain failure so the caller can fall back to
    /// the key file. A locked keychain or a headless session is a normal
    /// condition for `clipboarder serve`, not an error worth aborting on.
    pub fn get_or_create() -> Option<String> {
        if let Ok(raw) = get_generic_password(SERVICE, ACCOUNT) {
            let key = String::from_utf8(raw).ok()?.trim().to_ascii_lowercase();
            if valid(&key) {
                return Some(key);
            }
            // Present but malformed. Leave it alone rather than overwriting
            // what might be a key we simply failed to parse — falling through
            // to the file path would silently orphan an encrypted database.
            eprintln!("clipboarder: keychain entry for {SERVICE}/{ACCOUNT} is malformed");
            return None;
        }
        let key = generate();
        match set_generic_password(SERVICE, ACCOUNT, key.as_bytes()) {
            Ok(()) => Some(key),
            Err(err) => {
                eprintln!("clipboarder: could not store database key in keychain: {err}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_valid_and_distinct() {
        let _env = crate::testenv::lock();
        let a = generate();
        let b = generate();
        assert!(valid(&a), "generated key should be 64 hex chars: {a}");
        assert_ne!(a, b, "two generated keys must not collide");
    }

    #[test]
    fn rejects_malformed_keys() {
        let _env = crate::testenv::lock();
        assert!(!valid(""));
        assert!(!valid("short"));
        assert!(!valid(&"z".repeat(64)));
        assert!(valid(&"a".repeat(64)));
    }

    #[test]
    fn env_var_wins_and_is_normalized() {
        let _env = crate::testenv::lock();
        let dir = tempdir();
        let key = "A".repeat(64);
        std::env::set_var(ENV_VAR, &key);
        let got = get_or_create(&dir).expect("env key accepted");
        std::env::remove_var(ENV_VAR);
        assert_eq!(got, "a".repeat(64), "env key should be lowercased");
    }

    #[test]
    fn env_var_must_be_hex() {
        let _env = crate::testenv::lock();
        let dir = tempdir();
        std::env::set_var(ENV_VAR, "not-a-key");
        let got = get_or_create(&dir);
        std::env::remove_var(ENV_VAR);
        assert!(got.is_err(), "a malformed env key must be rejected, not ignored");
    }

    #[test]
    fn file_fallback_round_trips_and_is_0600() {
        let _env = crate::testenv::lock();
        let dir = tempdir();
        let first = from_file(&dir).expect("first call creates a key");
        let second = from_file(&dir).expect("second call reads it back");
        assert_eq!(first, second, "key must be stable across calls");
        assert!(valid(&first));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(KEY_FILE)).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "key file must not be group/world readable");
        }
    }

    fn tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("clipboarder-dbkey-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
