# Privacy & exclusions

clipboarder watches the system pasteboard. That's privileged. The **Privacy** section in Settings is how you tell it to *not* record clipboard activity from specific apps.

## Why it matters

Password managers (1Password, Bitwarden, Dashlane) put secrets on the clipboard when you copy. If clipboarder captures them, they'd live in your history. **You don't want that.**

## How it works

For each clipboard event, clipboarder asks macOS *"which app is frontmost right now?"* (via `NSWorkspace.frontmostApplication.bundleIdentifier`). If that bundle id is in the excluded list, the event is dropped — never read, never written to the database.

## Adding apps

In **Settings → Privacy**:

1. **Add frontmost app** — adds whatever app is currently in front. Handy: open the app you want to exclude, then hit this button.
2. **Manual entry** — type a bundle id directly. Bundle ids look like `com.1password.1password`.

Recommended exclusions:

```
com.1password.1password         # 1Password 8
com.agilebits.onepassword7      # 1Password 7
com.bitwarden.desktop           # Bitwarden
com.dashlane.dashlanephonefinal # Dashlane
com.lastpass.LastPassMacDesktop # LastPass
```

## How to find a bundle id

Open the app, then in a terminal:

```bash
osascript -e 'tell application "System Events" to bundle identifier of (first application process whose frontmost is true)'
```

Or from a `.app`:

```bash
mdls -name kMDItemCFBundleIdentifier /Applications/Some\ App.app
```

## What clipboarder still does

When an app is excluded, clipboarder:

- Does **not** read the pasteboard during that app's clipboard events
- Does **not** store anything tied to that event
- Does **not** see the bytes — privacy is enforced before the watcher reads

It does still know **which app** was active (because that's how we decide whether to skip), but nothing else.

## Source-of-truth

The exclusion list is stored in `settings.json`:

```json
{
  "excluded_apps": [
    "com.1password.1password",
    "com.bitwarden.desktop"
  ]
}
```
