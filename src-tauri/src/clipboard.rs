//! Clipboard watcher: observes NSPasteboard changes and persists new items.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use clipboard_rs::{
    common::RustImage, Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher,
    ClipboardWatcherContext, ContentFormat, RustImageData,
};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::classify::{self, Kind};
use crate::settings::SettingsStore;
use crate::storage::{NewItem, Storage, DEFAULT_NAMESPACE};

pub fn start_watcher(
    app: AppHandle,
    storage: Arc<Mutex<Storage>>,
    settings: Arc<SettingsStore>,
    images_dir: PathBuf,
) {
    thread::Builder::new()
        .name("clipboarder-watcher".into())
        .spawn(move || {
            let handler = Handler {
                app,
                storage,
                settings,
                images_dir,
                last_hash: Mutex::new(String::new()),
            };
            let mut watcher: ClipboardWatcherContext<Handler> =
                ClipboardWatcherContext::new().expect("init clipboard watcher");
            let _shutdown = watcher.add_handler(handler).get_shutdown_channel();
            watcher.start_watch();
        })
        .expect("spawn clipboard watcher thread");
}

struct Handler {
    app: AppHandle,
    storage: Arc<Mutex<Storage>>,
    settings: Arc<SettingsStore>,
    images_dir: PathBuf,
    last_hash: Mutex<String>,
}

impl Handler {
    fn process(&self) -> Result<()> {
        // Respect the pasteboard's own "don't record me" marker first. This is
        // the only signal that is actually attached to the copy itself —
        // is_excluded() below infers the source from whichever app happens to
        // be frontmost, which races anything that writes the pasteboard from
        // the background.
        if let Some(marker) = self.privacy_marker() {
            eprintln!("clipboarder: skipping copy marked {marker}");
            return Ok(());
        }

        // Respect privacy exclusions: skip capture if the frontmost app is on
        // the list (e.g. 1Password). On non-macOS this is a no-op.
        if self.is_excluded() { return Ok(()); }

        let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("clipboard ctx: {e}"))?;

        // Prefer text. If absent, try image. If absent, try files.
        if ctx.has(ContentFormat::Text) {
            if let Ok(text) = ctx.get_text() {
                if text.is_empty() { return Ok(()); }
                // Never let a credential reach the database in the first
                // place. Redaction at read time (the CLI's --no-secrets) only
                // hides it from agents; the plaintext still sits on disk for
                // as long as the row survives.
                if !self.settings.get().store_secrets {
                    if let Some(kind) = crate::secrets::detect(&text) {
                        eprintln!(
                            "clipboarder: not storing clipboard content that looks like a {}",
                            kind.label()
                        );
                        return Ok(());
                    }
                }
                let hash = hash_str(&text);
                if self.is_dup(&hash) { return Ok(()); }
                let cls = classify::classify_text(&text);
                let (app_name, app_id) = self.frontmost_app_info();
                let mut db = self.storage.lock();
                db.upsert(&NewItem {
                    kind: cls.kind,
                    content: &text,
                    preview: &cls.preview,
                    meta: cls.meta.as_deref(),
                    source_app: app_name.as_deref(),
                    source_app_id: app_id.as_deref(),
                    image_path: None,
                    content_hash: &hash,
                    size: text.len() as i64,
                }, DEFAULT_NAMESPACE)?;
                drop(db);
                *self.last_hash.lock() = hash;
                self.enforce_limits();
                let _ = self.app.emit("clipboard:new", ());
            }
            return Ok(());
        }

        if ctx.has(ContentFormat::Image) {
            if let Ok(img) = ctx.get_image() {
                self.save_image(img)?;
                self.enforce_limits();
                let _ = self.app.emit("clipboard:new", ());
            }
            return Ok(());
        }

        if ctx.has(ContentFormat::Files) {
            if let Ok(files) = ctx.get_files() {
                if files.is_empty() { return Ok(()); }
                let joined = files.join("\n");
                let hash = hash_str(&joined);
                if self.is_dup(&hash) { return Ok(()); }
                let preview = if files.len() == 1 {
                    files[0].clone()
                } else {
                    format!("{} files · {}", files.len(), files[0])
                };
                let (app_name, app_id) = self.frontmost_app_info();
                // If we got exactly one file, refine the kind based on extension
                // (e.g. .pdf -> Kind::Pdf for a richer preview).
                let kind = if files.len() == 1 {
                    classify::kind_for_file(&files[0])
                } else {
                    Kind::File
                };
                let mut db = self.storage.lock();
                db.upsert(&NewItem {
                    kind,
                    content: &joined,
                    preview: &preview,
                    meta: None,
                    source_app: app_name.as_deref(),
                    source_app_id: app_id.as_deref(),
                    image_path: None,
                    content_hash: &hash,
                    size: joined.len() as i64,
                }, DEFAULT_NAMESPACE)?;
                drop(db);
                *self.last_hash.lock() = hash;
                self.enforce_limits();
                let _ = self.app.emit("clipboard:new", ());
            }
        }
        Ok(())
    }

    fn save_image(&self, img: RustImageData) -> Result<()> {
        // Encode as PNG bytes for hashing + write to disk.
        let png = img
            .to_png()
            .map_err(|e| anyhow::anyhow!("encode png: {e}"))?;
        let bytes = png.get_bytes();
        let hash = hash_bytes(bytes);
        if self.is_dup(&hash) { return Ok(()); }

        std::fs::create_dir_all(&self.images_dir).ok();
        let filename = format!("{}.png", &hash[..16]);
        let path = self.images_dir.join(&filename);
        std::fs::write(&path, bytes)?;

        let (w, h) = (img.get_size().0, img.get_size().1);
        let preview = format!("{} × {} image", w, h);
        let (app_name, app_id) = self.frontmost_app_info();

        let mut db = self.storage.lock();
        db.upsert(&NewItem {
            kind: Kind::Image,
            content: "",
            preview: &preview,
            meta: Some(&format!("{}x{}", w, h)),
            source_app: app_name.as_deref(),
            source_app_id: app_id.as_deref(),
            image_path: path.to_str(),
            content_hash: &hash,
            size: bytes.len() as i64,
        }, DEFAULT_NAMESPACE)?;
        drop(db);
        *self.last_hash.lock() = hash;
        Ok(())
    }

    fn is_dup(&self, hash: &str) -> bool {
        *self.last_hash.lock() == hash
    }

    #[cfg(target_os = "macos")]
    fn privacy_marker(&self) -> Option<&'static str> {
        crate::macos::pasteboard_privacy_marker()
    }

    #[cfg(not(target_os = "macos"))]
    fn privacy_marker(&self) -> Option<&'static str> { None }

    #[cfg(target_os = "macos")]
    fn is_excluded(&self) -> bool {
        let s = self.settings.get();
        let Some(bid) = crate::macos::frontmost_bundle_id() else { return false; };
        if s.use_default_exclusions
            && crate::settings::DEFAULT_EXCLUDED_APPS
                .iter()
                .any(|e| bid.eq_ignore_ascii_case(e))
        {
            return true;
        }
        s.excluded_apps.iter().any(|e| e.eq_ignore_ascii_case(&bid))
    }

    #[cfg(not(target_os = "macos"))]
    fn is_excluded(&self) -> bool { false }

    #[cfg(target_os = "macos")]
    fn frontmost_app_info(&self) -> (Option<String>, Option<String>) {
        (
            crate::macos::frontmost_app_name(),
            crate::macos::frontmost_bundle_id(),
        )
    }

    #[cfg(not(target_os = "macos"))]
    fn frontmost_app_info(&self) -> (Option<String>, Option<String>) { (None, None) }

    fn enforce_limits(&self) {
        let s = self.settings.get();
        if s.max_items == 0 && s.auto_clear_days == 0 { return; }
        let mut db = self.storage.lock();
        let mut stale = Vec::new();
        if s.max_items > 0 {
            if let Ok(paths) = db.enforce_limit(s.max_items, DEFAULT_NAMESPACE) {
                stale.extend(paths);
            }
        }
        // Age-based retention used to run only in `run()` at app launch, so on
        // a machine that stays up for weeks it never fired at all. Enforce it
        // on the same path as the count limit.
        if s.auto_clear_days > 0 {
            if let Ok(paths) = db.prune_older_than(s.auto_clear_days, DEFAULT_NAMESPACE) {
                stale.extend(paths);
            }
        }
        drop(db);
        for p in stale { let _ = std::fs::remove_file(p); }
    }
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) {
        if let Err(err) = self.process() {
            eprintln!("clipboarder watcher error: {err:?}");
        }
    }
}

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex(&h.finalize())
}

fn hash_bytes(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
