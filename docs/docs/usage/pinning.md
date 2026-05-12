# Pinning & history

## Pinning

Open the **Preview** pane for any item and click the star icon (top-right). The row now carries a small ⭐ in its meta line.

Pinned items:

- Always sort to the top of the list, above unpinned items
- Are **never** removed by history limits or auto-clear
- Are **never** removed by *Clear history* (only their unpinned siblings are)
- Survive across launches

Click the star again to unpin.

## History limits

In **Settings → History**:

- **Maximum items** — caps non-pinned rows. Defaults to 500. Options: 100 / 250 / 500 / 1,000 / 2,500 / 5,000 / Unlimited. When you exceed the cap on capture, the oldest non-pinned rows (by `last_used_at`) are deleted.
- **Auto-clear after** — removes non-pinned items older than N days. Defaults to *Never*. Options: 1 day / 1 week / 1 month / 3 months / 1 year. The cleanup runs at startup and on every new capture.

Both controls only ever delete non-pinned items.

## Clear history

In the tray menu → **Clear history**, or in Settings → **Clear all history**. This wipes all non-pinned rows in one shot. Pinned items survive.

## Dedup

clipboarder hashes each item with SHA-256. Copying the same text twice doesn't create a second row — instead, the existing row's `last_used_at` is bumped, so it floats to the top.

## Where is the data?

```
~/Library/Application Support/com.clipboarder.app/
├── clipboarder.sqlite              # FTS5 index + items
├── clipboarder.sqlite-shm
├── clipboarder.sqlite-wal
├── settings.json                   # your preferences
├── images/<hash16>.png             # captured image clipboard payloads
├── app_icons/<bundle_id>.png       # cached source-app icons
└── url_meta/<hash12>.json          # cached OpenGraph metadata
```

Delete the whole directory to start fresh (and remove clipboarder).
