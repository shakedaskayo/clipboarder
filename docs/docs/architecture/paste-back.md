# Paste-back

The trickiest part of any clipboard manager: select an item in the overlay, and have it pasted into whichever app you were just using.

## The dance

When you press Enter on a row, clipboarder runs the `paste_item` IPC command:

```rust
#[tauri::command]
pub fn paste_item(app, state, id) -> Result<()> {
    // 1. Look up the item, bump its last_used_at
    let item = state.db.get(id)?;

    // 2. Write its content to NSPasteboard (text, image bytes, or files)
    paste::copy_to_clipboard(&item)?;

    // 3. Hide the overlay window
    win.hide()?;

    // 4. Hide the app itself — this matters
    macos::hide_app();   // [NSApp hide:nil]

    // 5. Wait ~60 ms for AppKit to surface the previous app, then synthesize ⌘V
    paste::simulate_paste()?;
    Ok(())
}
```

## Why `[NSApp hide:]` is required

The naïve sequence is: write to pasteboard → hide window → CGEventPost ⌘V. That breaks: with just `win.hide()`, clipboarder is still the *active* application, even though no windows are visible. The synthesized `⌘V` is delivered to clipboarder's (now-invisible) window and bounces off.

`NSApplication.hide:` does two things in one synchronous call:

1. Hides all of the app's windows
2. **Deactivates the app**, surfacing the next-most-recently-active app

After it returns, the previously-focused Safari/VS Code/Slack window is now key. `CGEventPost` lands there.

## Why the 60 ms sleep

AppKit needs a moment to finish the activation handoff. 60 ms is the empirically-determined minimum on macOS 11–14. Below that, the `⌘V` occasionally lands while clipboarder is still active.

## Synthesizing `⌘V`

```rust
let src = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)?;
const KEY_V: CGKeyCode = 9;
let down = CGEvent::new_keyboard_event(src.clone(), KEY_V, true)?;
down.set_flags(CGEventFlags::CGEventFlagCommand);
down.post(CGEventTapLocation::HID);
let up = CGEvent::new_keyboard_event(src, KEY_V, false)?;
up.set_flags(CGEventFlags::CGEventFlagCommand);
up.post(CGEventTapLocation::HID);
```

Posting at `CGEventTapLocation::HID` injects into the system event stream at the lowest level (above the keyboard driver, below every app). All apps see it just like a real key press.

## Accessibility permission

Posting synthesized keyboard events requires the **Accessibility** privilege in *System Settings → Privacy & Security → Accessibility*. macOS does not allow this to be requested programmatically — the user has to grant it manually.

If the permission is missing, `CGEventPost` silently does nothing. The clipboard is updated (since `copy_to_clipboard` doesn't require any permission), but the receiving app doesn't see the `⌘V`. clipboarder shows the user a hint on first launch and in the docs.

## What about images and files?

Same flow. `copy_to_clipboard`:

- For `Kind::Image`: reads the stored PNG, hands it to `NSPasteboard` via `clipboard-rs::Clipboard::set_image`.
- For `Kind::File`: writes the file URL list (the raw newline-separated paths) as text — most apps accept that.
- For everything else: `set_text`.

The simulated `⌘V` then triggers the receiving app's native paste handler, which interprets the pasteboard contents however it normally would.
