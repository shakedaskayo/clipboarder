# Contributing to clipboarder

Thanks for considering a contribution! This guide will get you set up to build clipboarder locally.

## Prerequisites

- **macOS 11+** (clipboarder is macOS-only — it relies on NSPasteboard/NSWorkspace/CGEvent)
- **Rust** stable, latest — install via [rustup.rs](https://rustup.rs)
- **Node.js 20+** — for the frontend toolchain
- **Python 3.10+** — only if you want to build the docs site locally

## Quick start

```bash
git clone https://github.com/shakedaskayo/clipboarder.git
cd clipboarder

# Install JS deps
npm install

# Run in dev mode (Vite + tauri dev with HMR)
make dev
```

The first build takes a few minutes (Rust dep graph), subsequent builds are incremental and fast.

## Building releases

```bash
make build        # full release build: target/release/clipboarder + bundle/macos/clipboarder.app + bundle/dmg/*.dmg
make dmg          # same, exits after the DMG is produced
```

The bundled outputs land in `src-tauri/target/release/bundle/`.

## Running tests + checks

```bash
make test         # cargo test + tsc --noEmit
make lint         # cargo clippy + cargo fmt --check
```

## Working on the docs site

```bash
pip install -r docs/requirements.txt
cd docs
mkdocs serve
# Open http://127.0.0.1:8000
```

Edits to `docs/docs/*.md` hot-reload.

## Code layout

| Path | Purpose |
|------|---------|
| `src/` | React frontend |
| `src/components/` | UI components: Row, Preview, Settings, HotkeyRecorder, Chips, Toggle, Select |
| `src/lib/` | Frontend helpers: api, types, color parser, hotkey parser, app-icon cache hook |
| `src-tauri/src/lib.rs` | App bootstrap, hotkey registration, tray menu |
| `src-tauri/src/clipboard.rs` | NSPasteboard watcher, dedup, classification dispatch |
| `src-tauri/src/classify.rs` | Heuristic classifier (text/url/email/code/color/file/pdf/music/video) |
| `src-tauri/src/storage.rs` | SQLite + FTS5 with bm25 ranking, retention enforcement |
| `src-tauri/src/paste.rs` | Write-back to NSPasteboard, ⌘V synthesis via CGEventPost |
| `src-tauri/src/settings.rs` | JSON-persisted user settings |
| `src-tauri/src/app_icons.rs` | App-icon cache (extraction in `macos.rs`) |
| `src-tauri/src/commands.rs` | Tauri IPC command handlers |
| `src-tauri/src/macos.rs` | NSWindow level, NSWorkspace lookups |
| `scripts/make_icon.py` | Regenerates the app icon at every required size |
| `docs/` | MkDocs Material documentation site |

## Adding a new content kind

1. Extend `Kind` in `src-tauri/src/classify.rs` and update `as_str` / `from_str`.
2. Add a detector — either text-based (`classify_text`) or path-based (`kind_for_file`).
3. Mirror the union in `src/lib/types.ts`.
4. Add an icon to `src/lib/icons.tsx` and a colored tile rule to `styles.css` (`.row-icon.kind-<your-kind>`).
5. Add a chip to `src/components/Chips.tsx`.
6. Render a preview in `src/components/Preview.tsx`.

## Pull requests

- One feature / fix per PR. Small PRs review faster.
- Include a screenshot/GIF for UI changes.
- Make sure `make test` passes locally.
- Reference the issue you're closing in the PR description.

By contributing, you agree your work is released under the [MIT license](LICENSE).
