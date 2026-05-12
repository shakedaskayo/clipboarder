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

## First launch — guided permission grant

On first launch clipboarder shows an in-app banner asking for **Accessibility permission**. This is what lets it synthesize `⌘V` into your previously-focused app after you pick an item:

1. Click **Open Settings** in the banner → macOS jumps straight to the right pane
2. Toggle `clipboarder` on
3. Switch back to clipboarder — the banner auto-detects within ~2 seconds and turns green

You only do this once. Without it, *Copy to clipboard* still works, but the auto-paste step won't fire (you'd have to press `⌘V` yourself).

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
