//! User-facing settings, persisted as JSON in the app data dir.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Accelerator string accepted by tauri-plugin-global-shortcut,
    /// e.g. "CommandOrControl+Shift+V".
    pub hotkey: String,
    /// Start clipboarder on user login.
    pub launch_at_login: bool,
    /// Max items kept in history (pinned items are always preserved).
    pub max_items: u32,
    /// Auto-delete non-pinned items older than this many days. 0 = never.
    pub auto_clear_days: u32,
    /// Bundle identifiers of apps whose clipboard activity we should ignore.
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "CommandOrControl+Shift+V".into(),
            launch_at_login: false,
            max_items: 500,
            auto_clear_days: 0,
            excluded_apps: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(path: &Path) -> Result<Self> {
        let current = if path.exists() {
            let raw = std::fs::read_to_string(path).context("read settings.json")?;
            serde_json::from_str::<Settings>(&raw).unwrap_or_default()
        } else {
            Settings::default()
        };
        Ok(Self {
            path: path.to_path_buf(),
            current: Mutex::new(current),
        })
    }

    pub fn get(&self) -> Settings {
        self.current.lock().clone()
    }

    pub fn save(&self, new: Settings) -> Result<Settings> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(&new)?;
        std::fs::write(&self.path, json).context("write settings.json")?;
        *self.current.lock() = new.clone();
        Ok(new)
    }
}
