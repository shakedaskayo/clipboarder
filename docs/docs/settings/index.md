# Settings

Open the Settings panel three ways:

- **Gear icon** in the bottom-right of the overlay footer
- `⌘,` keyboard shortcut (macOS convention)
- Tray menu → **Settings…**

All changes save instantly to `~/Library/Application Support/com.clipboarder.app/settings.json`.

## Sections

- [Hotkey](hotkey.md) — record a custom global shortcut
- [Launch at login](launch-at-login.md) — start clipboarder when you log in
- [History limits](history.md) — cap retention by count and age
- [Privacy & exclusions](privacy.md) — keep sensitive apps' clipboard out of history

## Settings file format

```json
{
  "hotkey": "CommandOrControl+Shift+V",
  "launch_at_login": true,
  "max_items": 500,
  "auto_clear_days": 30,
  "excluded_apps": ["com.1password.1password"]
}
```

Edit by hand if you want — clipboarder reloads the file on every save command. Just keep the JSON valid.
