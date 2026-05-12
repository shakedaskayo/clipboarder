# Architecture

clipboarder is a Tauri 2 desktop app: Rust backend, React + TypeScript frontend, communicating via Tauri's IPC bridge.

## High-level dataflow

```
┌──────────────────┐
│  NSPasteboard    │  the macOS system clipboard
└────────┬─────────┘
         │ change-count poll
         ▼
┌──────────────────┐    classify::classify_text(...)
│  watcher thread  │──▶ SHA-256 hash, dedup
│ (clipboard.rs)   │    storage::upsert(...)
└──────────────────┘
         │ emit "clipboard:new"
         ▼
┌──────────────────┐
│ React frontend   │  re-runs search_items, re-renders list
└────────┬─────────┘
         │ user picks an item
         ▼
┌──────────────────┐    paste::copy_to_clipboard
│  paste_item IPC  │──▶ NSApp.hide:
│   (commands.rs)  │    paste::simulate_paste (CGEventPost ⌘V)
└──────────────────┘
         │
         ▼
   previously-focused app receives ⌘V
```

## Module map

| Module | Responsibility |
|--------|----------------|
| `src-tauri/src/lib.rs` | App bootstrap, hotkey registration, tray menu, window lifecycle |
| `src-tauri/src/clipboard.rs` | NSPasteboard watcher thread, text/image/file capture, dedup |
| `src-tauri/src/classify.rs` | Heuristic classification ([details](classification.md)) |
| `src-tauri/src/storage.rs` | SQLite + FTS5 ([details](storage.md)) |
| `src-tauri/src/paste.rs` | Write-back, ⌘V synthesis ([details](paste-back.md)) |
| `src-tauri/src/settings.rs` | JSON-persisted user settings |
| `src-tauri/src/app_icons.rs` | On-demand app-icon extraction + cache |
| `src-tauri/src/url_meta.rs` | OpenGraph fetcher with on-disk cache |
| `src-tauri/src/commands.rs` | Tauri IPC handlers (search/paste/copy/pin/delete/settings) |
| `src-tauri/src/macos.rs` | NSWindow level, NSWorkspace lookups, app-hide |
| `src/components/` | React UI: Row, Preview, UrlCard, RepoCard, Settings, HotkeyRecorder |
| `src/lib/` | Frontend helpers: api, types, color parser, hotkey parser, hooks |

## Why Tauri (not Electron)

- **Native webview** — uses macOS WKWebView. No bundled Chromium. ~10 MB shipped binary vs 100+ MB.
- **Rust backend** — the hot path (clipboard watcher, classifier, SQLite, FTS5, app-icon extraction, OG fetcher) runs in Rust. Zero GC pauses, predictable latency.
- **IPC bridge** — typed Tauri commands keep the frontend/backend contract honest.

## Why local SQLite + FTS5

- Sub-millisecond search on tens of thousands of items
- bm25 ranking is built in
- Triggers keep the FTS index in sync with the source table without app-level bookkeeping
- WAL mode means the read-mostly UI doesn't block writes from the watcher thread

## Why a separate watcher thread

The watcher uses `clipboard-rs::ClipboardWatcherContext`, which blocks on a CFRunLoop. Putting it on its own thread keeps the Tauri main event loop responsive. The two communicate via shared state (`Arc<parking_lot::Mutex<Storage>>`) and emitted events.

## Why all data stays local

clipboarder makes a network call **only** when you preview a URL and we haven't fetched its OpenGraph metadata yet — and the request is for the URL you copied, with a clipboarder-tagged User-Agent. Everything else is on-disk.

## Read on

- [Classification](classification.md) — the heuristics in `classify.rs`
- [Storage & search](storage.md) — schema, triggers, query patterns
- [Paste-back](paste-back.md) — the ⌘V synthesis dance
