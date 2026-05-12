# Installation

clipboarder supports macOS 11+ on Apple Silicon and Intel.

## One-liner (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
```

What it does:

1. Detects your CPU architecture (`arm64` / `x86_64`)
2. Looks up the latest release on GitHub
3. Downloads the matching `.dmg`
4. Mounts it, copies `clipboarder.app` to `/Applications`, unmounts
5. Strips macOS's quarantine attribute so you don't get the "downloaded from internet" prompt
6. Launches clipboarder

### Installer flags

| Flag | Description |
|------|-------------|
| `--version v0.1.0` | Install a specific release tag instead of `latest` |
| `--dir ~/Applications` | Install somewhere other than `/Applications` |
| `--keep-dmg` | Don't delete the `.dmg` after copying |
| `--no-launch` | Don't auto-open clipboarder after install |
| `--help` | Show usage |

Example — install a pinned version to `~/Applications`:

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh \
  | bash -s -- --version v0.1.0 --dir ~/Applications
```

## Manual `.dmg`

Download the latest `.dmg` from the [Releases page](https://github.com/shakedaskayo/clipboarder/releases/latest), open it, and drag `clipboarder` into `Applications`.

| Architecture | File |
|--------------|------|
| Apple Silicon (M1/M2/M3/M4) | `clipboarder_<version>_aarch64.dmg` |
| Intel | `clipboarder_<version>_x64.dmg` |

## From source

You'll need [Rust](https://rustup.rs) stable and Node 20+:

```bash
git clone https://github.com/shakedaskayo/clipboarder.git
cd clipboarder
make dmg
open src-tauri/target/release/bundle/dmg/
```

See [Contributing](../contributing/index.md) for the full development setup.

## First launch

On first launch macOS will request **Accessibility permission**. This is needed so clipboarder can synthesize `⌘V` into the previously-focused app after you select an item.

To grant:

1. Open **System Settings** → **Privacy & Security** → **Accessibility**
2. Toggle `clipboarder` on (or click `+` and add it from `/Applications`)

Until you grant it, *Copy to clipboard* still works — but paste-back won't fire automatically and you'll have to press `⌘V` yourself.

## Uninstall

```bash
rm -rf /Applications/clipboarder.app
rm -rf ~/Library/Application\ Support/com.clipboarder.app
```

If you enabled **Launch at login**, also delete:

```bash
rm -f ~/Library/LaunchAgents/com.clipboarder.app.plist
```

→ [Quickstart](quickstart.md)
