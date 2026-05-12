# Launch at login

Toggle on to start clipboarder automatically when you log in.

## How it works

clipboarder uses [`tauri-plugin-autostart`](https://v2.tauri.app/plugin/autostart/), which on macOS creates a LaunchAgent at:

```
~/Library/LaunchAgents/com.clipboarder.app.plist
```

`launchctl` loads the plist on login, runs the clipboarder binary, and macOS treats it like any other login item.

## Verifying

Check the file exists:

```bash
ls -l ~/Library/LaunchAgents/com.clipboarder.app.plist
```

You can also see it in **System Settings → General → Login Items** under *Open at Login*.

## Disabling

Toggle off in Settings, or delete the plist manually:

```bash
launchctl unload ~/Library/LaunchAgents/com.clipboarder.app.plist
rm ~/Library/LaunchAgents/com.clipboarder.app.plist
```

## Hidden start

clipboarder always starts with the overlay hidden when launched at login. Press the hotkey or click the tray icon to bring it up.
