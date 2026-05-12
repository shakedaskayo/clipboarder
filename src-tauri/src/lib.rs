mod app_icons;
mod classify;
mod clipboard;
mod commands;
mod paste;
mod settings;
mod storage;
mod url_meta;

#[cfg(target_os = "macos")]
mod macos;

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::app_icons::AppIconCache;
use crate::settings::SettingsStore;
use crate::url_meta::UrlMetaCache;

pub struct AppState {
    pub db: Arc<Mutex<storage::Storage>>,
    pub settings: Arc<SettingsStore>,
    pub app_icons: Arc<AppIconCache>,
    pub url_meta: Arc<UrlMetaCache>,
    pub images_dir: std::path::PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Resolve app data dir
            let data_dir = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&data_dir).ok();
            let images_dir = data_dir.join("images");
            std::fs::create_dir_all(&images_dir).ok();
            let db_path = data_dir.join("clipboarder.sqlite");
            let settings_path = data_dir.join("settings.json");

            let storage = storage::Storage::open(&db_path).expect("open db");
            let storage = Arc::new(Mutex::new(storage));

            let settings_store = Arc::new(
                SettingsStore::load(&settings_path).expect("load settings"),
            );

            let app_icons = Arc::new(AppIconCache::new(data_dir.join("app_icons")));
            let url_meta = Arc::new(UrlMetaCache::new(data_dir.join("url_meta")));

            // Apply persisted retention settings at startup.
            {
                let s = settings_store.get();
                let mut db = storage.lock();
                if s.max_items > 0 {
                    if let Ok(paths) = db.enforce_limit(s.max_items) {
                        for p in paths { let _ = std::fs::remove_file(p); }
                    }
                }
                if s.auto_clear_days > 0 {
                    if let Ok(paths) = db.prune_older_than(s.auto_clear_days) {
                        for p in paths { let _ = std::fs::remove_file(p); }
                    }
                }
            }

            app.manage(AppState {
                db: storage.clone(),
                settings: settings_store.clone(),
                app_icons: app_icons.clone(),
                url_meta: url_meta.clone(),
                images_dir: images_dir.clone(),
            });

            // Start clipboard watcher
            clipboard::start_watcher(
                app_handle.clone(),
                storage.clone(),
                settings_store.clone(),
                images_dir.clone(),
            );

            // Register hotkey from settings
            let hotkey = settings_store.get().hotkey;
            register_hotkey(&app_handle, &hotkey)?;

            // System tray
            build_tray(&app_handle)?;

            // macOS panel-style window
            let win = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            macos::configure_window(&win);

            let hide_handle = app_handle.clone();
            let no_auto_hide = std::env::var("CLIPBOARDER_NO_AUTO_HIDE").is_ok();
            win.on_window_event(move |event| match event {
                WindowEvent::Focused(false) => {
                    if no_auto_hide { return; }
                    if let Some(w) = hide_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Some(w) = hide_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
                _ => {}
            });

            let _ = win.show();
            let _ = win.set_focus();
            let _ = app_handle.emit("window:shown", ());

            // Open straight into Settings when launched for screenshots.
            if std::env::var("CLIPBOARDER_INITIAL_VIEW").as_deref() == Ok("settings") {
                let handle = app_handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    let _ = handle.emit("nav:settings", ());
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_items,
            commands::paste_item,
            commands::copy_item,
            commands::toggle_pin,
            commands::delete_item,
            commands::clear_history,
            commands::hide_window,
            commands::get_settings,
            commands::save_settings,
            commands::frontmost_app_info,
            commands::get_app_icon,
            commands::fetch_file_bytes,
            commands::fetch_url_metadata,
            commands::open_url,
            commands::initial_view,
            commands::initial_filter,
            commands::accessibility_trusted,
            commands::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running clipboarder");
}

fn register_hotkey(app: &AppHandle, accel: &str) -> Result<()> {
    let shortcut = Shortcut::from_str(accel)
        .map_err(|e| anyhow!("invalid accelerator '{accel}': {e:?}"))?;
    let handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _scut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_window(&handle);
            }
        })
        .map_err(|e| anyhow!("register {accel}: {e:?}"))?;
    eprintln!("[clipboarder] registered hotkey: {accel}");
    Ok(())
}

pub(crate) fn reapply_hotkey(app: &AppHandle, prev: &str, next: &str) -> Result<()> {
    // Unregister the old shortcut (best-effort).
    if let Ok(s) = Shortcut::from_str(prev) {
        let _ = app.global_shortcut().unregister(s);
    }
    register_hotkey(app, next)
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Show clipboarder", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let clear = MenuItem::with_id(app, "clear", "Clear history", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit clipboarder", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &clear, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| Image::new_owned(vec![0; 4], 1, 1));

    let app_for_events = app.clone();
    TrayIconBuilder::with_id("clipboarder-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("clipboarder")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => toggle_window(app),
            "settings" => {
                show_window(app);
                let _ = app.emit("nav:settings", ());
            }
            "quit" => {
                app.exit(0);
            }
            "clear" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let imgs = state.db.lock().clear().unwrap_or_default();
                    for p in imgs {
                        let _ = std::fs::remove_file(p);
                    }
                    let _ = app.emit("clipboard:new", ());
                }
            }
            _ => {}
        })
        .build(&app_for_events)?;
    Ok(())
}

fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => {
                let _ = win.hide();
            }
            _ => {
                let _ = win.center();
                let _ = win.show();
                let _ = win.set_focus();
                let _ = app.emit("window:shown", ());
            }
        }
    }
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit("window:shown", ());
    }
}
