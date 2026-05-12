//! On-demand extraction and caching of macOS app icons keyed by bundle id.

use std::path::PathBuf;

use parking_lot::Mutex;

#[derive(Debug)]
pub struct AppIconCache {
    pub dir: PathBuf,
    /// In-memory map of "bundle_id -> Some(path) | None (negative cache)".
    seen: Mutex<std::collections::HashMap<String, Option<String>>>,
}

impl AppIconCache {
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        Self {
            dir,
            seen: Mutex::new(Default::default()),
        }
    }

    /// Returns the on-disk path to a PNG for this bundle id, generating it if
    /// missing. Returns None when the system can't resolve the bundle.
    pub fn get_or_extract(&self, bundle_id: &str) -> Option<String> {
        if bundle_id.is_empty() { return None; }

        if let Some(cached) = self.seen.lock().get(bundle_id).cloned() {
            return cached;
        }

        let safe = sanitize(bundle_id);
        let path = self.dir.join(format!("{safe}.png"));
        if path.exists() {
            let s = path.to_string_lossy().into_owned();
            self.seen.lock().insert(bundle_id.into(), Some(s.clone()));
            return Some(s);
        }

        #[cfg(target_os = "macos")]
        let png = crate::macos::extract_app_icon_png(bundle_id, 64);
        #[cfg(not(target_os = "macos"))]
        let png: Option<Vec<u8>> = None;

        let result = match png {
            Some(bytes) => match std::fs::write(&path, &bytes) {
                Ok(_) => Some(path.to_string_lossy().into_owned()),
                Err(_) => None,
            },
            None => None,
        };
        self.seen.lock().insert(bundle_id.into(), result.clone());
        result
    }
}

fn sanitize(bundle_id: &str) -> String {
    bundle_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
