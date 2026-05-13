use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::paste;
use crate::settings::Settings;
use crate::storage::{ClipItem, DEFAULT_NAMESPACE};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct QueryArgs {
    pub query: String,
    pub kind: String,
    pub limit: i64,
}

type CmdResult<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String { format!("{e:#}") }

#[tauri::command]
pub fn search_items(state: State<AppState>, args: QueryArgs) -> CmdResult<Vec<ClipItem>> {
    let db = state.db.lock();
    db.search(&args.query, &args.kind, args.limit.max(1), DEFAULT_NAMESPACE)
        .map_err(err)
}

#[tauri::command]
pub fn paste_item(app: AppHandle, state: State<AppState>, id: i64) -> CmdResult<()> {
    eprintln!("[paste] paste_item called id={id}");
    let item = {
        let mut db = state.db.lock();
        let it = db.get(id, DEFAULT_NAMESPACE).map_err(err)?;
        if it.is_some() { db.bump(id, DEFAULT_NAMESPACE).map_err(err)?; }
        it
    };
    let Some(item) = item else {
        eprintln!("[paste] item not found id={id}");
        return Err("item not found".into());
    };
    eprintln!("[paste] item kind={} content_len={}", item.kind, item.content.len());

    #[cfg(target_os = "macos")]
    {
        let pre = crate::macos::frontmost_bundle_id().unwrap_or_else(|| "<none>".into());
        eprintln!("[paste] frontmost BEFORE hide: {pre}");
    }

    paste::copy_to_clipboard(&item).map_err(err)?;
    eprintln!("[paste] clipboard written");

    #[cfg(target_os = "macos")]
    let prev_pid: Option<i32> = *state.prev_frontmost_pid.lock();
    #[cfg(target_os = "macos")]
    eprintln!("[paste] saved prev pid: {prev_pid:?}");

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let app_for_main = app.clone();
    let dispatch_res = app.run_on_main_thread(move || {
        eprintln!("[paste] (main) running hide");
        if let Some(win) = app_for_main.get_webview_window("main") {
            let _ = win.hide();
        }
        #[cfg(target_os = "macos")]
        crate::macos::hide_app();
        eprintln!("[paste] (main) hide done");
        let _ = tx.send(());
    });
    eprintln!("[paste] dispatch result: {:?}", dispatch_res.is_ok());
    let wait_res = rx.recv_timeout(std::time::Duration::from_millis(500));
    eprintln!("[paste] main-thread sync recv: {wait_res:?}");

    // Explicitly bring the user's previous app back to the foreground.
    // [NSApp hide:] on macOS 15 doesn't reliably hand focus back when our
    // window is floating + hidesOnDeactivate, so we activate the saved PID
    // directly. This is what makes the synthesized ⌘V land in their text
    // field instead of clipboarder's invisible webview.
    #[cfg(target_os = "macos")]
    if let Some(pid) = prev_pid {
        let ok = crate::macos::activate_app_by_pid(pid);
        eprintln!("[paste] activate_app_by_pid({pid}) -> {ok}");
    } else {
        eprintln!("[paste] no prev pid captured — relying on [NSApp hide:]");
    }

    #[cfg(target_os = "macos")]
    {
        let post = crate::macos::frontmost_bundle_id().unwrap_or_else(|| "<none>".into());
        eprintln!("[paste] frontmost AFTER hide+activate: {post}");
    }

    let synth_res = paste::simulate_paste();
    eprintln!("[paste] simulate_paste returned: {synth_res:?}");
    synth_res.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn copy_item(state: State<AppState>, id: i64) -> CmdResult<()> {
    let item = {
        let mut db = state.db.lock();
        let it = db.get(id, DEFAULT_NAMESPACE).map_err(err)?;
        if it.is_some() { db.bump(id, DEFAULT_NAMESPACE).map_err(err)?; }
        it
    };
    let Some(item) = item else { return Err("item not found".into()); };
    paste::copy_to_clipboard(&item).map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn toggle_pin(state: State<AppState>, id: i64) -> CmdResult<bool> {
    let mut db = state.db.lock();
    db.toggle_pin(id, DEFAULT_NAMESPACE).map_err(err)
}

#[tauri::command]
pub fn delete_item(state: State<AppState>, id: i64) -> CmdResult<()> {
    let img = {
        let mut db = state.db.lock();
        db.delete(id, DEFAULT_NAMESPACE).map_err(err)?
    };
    if let Some(path) = img { let _ = std::fs::remove_file(path); }
    Ok(())
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) -> CmdResult<()> {
    let imgs = {
        let mut db = state.db.lock();
        db.clear(DEFAULT_NAMESPACE).map_err(err)?
    };
    for p in imgs { let _ = std::fs::remove_file(p); }
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> CmdResult<()> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> CmdResult<Settings> {
    Ok(state.settings.get())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> CmdResult<Settings> {
    let prev = state.settings.get();
    let saved = state.settings.save(settings.clone()).map_err(err)?;

    // Re-register hotkey if it changed.
    if prev.hotkey != saved.hotkey {
        if let Err(e) = crate::reapply_hotkey(&app, &prev.hotkey, &saved.hotkey) {
            return Err(format!("hotkey: {e}"));
        }
    }

    // Apply autostart change.
    #[cfg(desktop)]
    if prev.launch_at_login != saved.launch_at_login {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let result = if saved.launch_at_login {
            manager.enable()
        } else {
            manager.disable()
        };
        if let Err(e) = result {
            eprintln!("[clipboarder] autostart toggle failed: {e}");
        }
    }

    Ok(saved)
}

#[tauri::command]
pub fn get_app_icon(state: State<AppState>, bundle_id: String) -> CmdResult<Option<String>> {
    Ok(state.app_icons.get_or_extract(&bundle_id))
}

#[tauri::command]
pub fn initial_view() -> CmdResult<String> {
    Ok(std::env::var("CLIPBOARDER_INITIAL_VIEW").unwrap_or_else(|_| "search".into()))
}

#[tauri::command]
pub fn initial_filter() -> CmdResult<String> {
    Ok(std::env::var("CLIPBOARDER_INITIAL_FILTER").unwrap_or_else(|_| "all".into()))
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<String>)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn accessibility_trusted() -> CmdResult<bool> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::macos::is_accessibility_trusted())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

#[tauri::command]
pub fn open_accessibility_settings(app: AppHandle) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
                .to_string(),
            None::<String>,
        )
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn fetch_url_metadata(
    state: State<'_, AppState>,
    url: String,
    refresh: Option<bool>,
) -> CmdResult<crate::url_meta::UrlMetadata> {
    if refresh != Some(true) {
        if let Some(meta) = state.url_meta.cached(&url) {
            if meta.error.is_none() {
                return Ok(meta);
            }
        }
    }
    state.url_meta.fetch(&url).await.map_err(err)
}

#[tauri::command]
pub fn fetch_file_bytes(path: String) -> CmdResult<Vec<u8>> {
    const MAX: u64 = 50 * 1024 * 1024; // 50 MB safety cap
    let meta = std::fs::metadata(&path).map_err(|e| format!("stat: {e}"))?;
    if meta.len() > MAX {
        return Err(format!("file too large ({} MB)", meta.len() / (1024 * 1024)));
    }
    std::fs::read(&path).map_err(|e| format!("read: {e}"))
}

#[tauri::command]
pub fn frontmost_app_info() -> CmdResult<Option<(String, String)>> {
    #[cfg(target_os = "macos")]
    {
        let bid = crate::macos::frontmost_bundle_id();
        let name = crate::macos::frontmost_app_name();
        match (bid, name) {
            (Some(b), Some(n)) => Ok(Some((b, n))),
            (Some(b), None) => Ok(Some((b.clone(), b))),
            _ => Ok(None),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}
