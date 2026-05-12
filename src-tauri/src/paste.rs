//! Paste-back: write content to the clipboard, restore previous-frontmost app,
//! then synthesize Cmd+V so the focused app receives a paste.

use std::thread;
use std::time::Duration;

use anyhow::Result;
use clipboard_rs::{common::RustImage, Clipboard, ClipboardContext, RustImageData};

use crate::storage::ClipItem;

pub fn copy_to_clipboard(item: &ClipItem) -> Result<()> {
    let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("clipboard ctx: {e}"))?;
    if item.kind == "image" {
        if let Some(path) = &item.image_path {
            let bytes = std::fs::read(path)?;
            let img = RustImageData::from_bytes(&bytes)
                .map_err(|e| anyhow::anyhow!("decode png: {e}"))?;
            ctx.set_image(img)
                .map_err(|e| anyhow::anyhow!("set image: {e}"))?;
            return Ok(());
        }
    }
    ctx.set_text(item.content.clone())
        .map_err(|e| anyhow::anyhow!("set text: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn simulate_paste() -> Result<()> {
    use core_graphics::event::{
        CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode,
    };
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // Give macOS time to (a) hide our window, (b) deactivate our app, and
    // (c) hand keyboard focus to the previously-frontmost app before the
    // synthesized ⌘V hits the event tap. 60 ms was the empirical minimum on
    // M-series; 150 ms is generously safe on Intel + slower machines while
    // still feeling instant.
    thread::sleep(Duration::from_millis(150));

    let src = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| anyhow::anyhow!("CGEventSource create"))?;

    const KEY_V: CGKeyCode = 9;

    let down = CGEvent::new_keyboard_event(src.clone(), KEY_V, true)
        .map_err(|_| anyhow::anyhow!("create down event"))?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);

    let up = CGEvent::new_keyboard_event(src, KEY_V, false)
        .map_err(|_| anyhow::anyhow!("create up event"))?;
    up.set_flags(CGEventFlags::CGEventFlagCommand);
    up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn simulate_paste() -> Result<()> {
    Ok(())
}
