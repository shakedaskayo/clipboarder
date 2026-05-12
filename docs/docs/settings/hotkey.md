# Hotkey

The global shortcut that toggles the clipboarder overlay. Default: `⌘⇧V`.

## Recording a new hotkey

1. Open Settings → **Hotkey**
2. Click **Record**
3. Press your desired combination (e.g. `⌘⌥V`)
4. Release the non-modifier key — clipboarder saves the binding immediately

Press `Esc` (without modifiers) to cancel mid-recording.

## Accelerator format

clipboarder uses Tauri's accelerator string. Modifiers:

- `CommandOrControl` — `⌘` on macOS
- `Meta` / `Super` — `⌘`
- `Control` — `⌃`
- `Alt` / `Option` — `⌥`
- `Shift` — `⇧`

Keys:

- Letters use `KeyA`, `KeyB`, … (the recorder picks these automatically)
- Digits use `Digit0`, …, `Digit9`
- Function keys: `F1`, …, `F12`
- Special: `Space`, `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `ArrowUp/Down/Left/Right`

Examples:

```
CommandOrControl+Shift+V          ⌘⇧V
CommandOrControl+Alt+Space        ⌘⌥Space
Control+Shift+Period              ⌃⇧.
```

## Why must the hotkey include a modifier?

Bare keys would intercept every keystroke from every app. clipboarder enforces at least one of `⌘`, `⌥`, or `⌃` plus a non-modifier key.

## What if registration fails?

If another app has grabbed the same combination at the OS level, clipboarder logs an error and the old hotkey stays active. Pick a different combo and try again.
