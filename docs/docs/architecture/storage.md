# Storage & search

clipboarder uses SQLite with the FTS5 extension, both bundled directly into the binary via `rusqlite`'s `bundled` feature.

## Schema

```sql
CREATE TABLE items (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    kind          TEXT NOT NULL,    -- text | url | email | code | color | image | file | pdf | music | video | repo
    content       TEXT NOT NULL,    -- full payload (empty for images; image_path holds the file)
    preview       TEXT NOT NULL,    -- single-line preview shown in the list
    meta          TEXT,             -- kind-specific: host, language, color format, platform
    source_app    TEXT,             -- localized name of the frontmost app at capture
    source_app_id TEXT,             -- its bundle id (e.g. "com.apple.Safari")
    image_path    TEXT,             -- path on disk for image captures
    pinned        INTEGER NOT NULL DEFAULT 0,
    size          INTEGER NOT NULL DEFAULT 0,
    content_hash  TEXT NOT NULL,    -- SHA-256 (32 bytes hex) for dedup
    created_at    INTEGER NOT NULL, -- unix ms
    last_used_at  INTEGER NOT NULL  -- unix ms; bumped on re-copy or paste-back
);

CREATE UNIQUE INDEX idx_items_hash       ON items(content_hash);
CREATE INDEX        idx_items_last_used  ON items(last_used_at DESC);
CREATE INDEX        idx_items_kind       ON items(kind, last_used_at DESC);

CREATE VIRTUAL TABLE items_fts USING fts5(
    content, preview, meta,
    content='items', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers keep items_fts in sync without app-level bookkeeping.
CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, content, preview, meta)
    VALUES (new.id, new.content, new.preview, COALESCE(new.meta, ''));
END;
CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, content, preview, meta)
    VALUES ('delete', old.id, old.content, old.preview, COALESCE(old.meta, ''));
END;
CREATE TRIGGER items_au AFTER UPDATE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, content, preview, meta)
    VALUES ('delete', old.id, old.content, old.preview, COALESCE(old.meta, ''));
    INSERT INTO items_fts(rowid, content, preview, meta)
    VALUES (new.id, new.content, new.preview, COALESCE(new.meta, ''));
END;
```

## Pragmas

```sql
PRAGMA journal_mode = WAL;       -- reader/writer concurrency
PRAGMA synchronous  = NORMAL;    -- WAL is safe enough at NORMAL
PRAGMA foreign_keys = ON;
```

WAL mode is critical: the watcher thread writes constantly, and the UI thread reads on every keystroke. WAL means they don't block each other.

## Insert path

```
storage::upsert(NewItem { ... }) -> (id, was_inserted)
```

1. Look up the row by `content_hash`. If present, bump `last_used_at` and return.
2. Otherwise INSERT new row.
3. Triggers replicate the change into `items_fts`.

## Search path

```
storage::search(query, kind_filter, limit) -> Vec<ClipItem>
```

Two code paths:

**Empty query** (just filters):

```sql
SELECT … FROM items
WHERE 1=1 [ AND kind = ? ] [ AND pinned = 1 ]
ORDER BY pinned DESC, last_used_at DESC
LIMIT ?
```

**Non-empty query** — FTS MATCH plus bm25 ranking:

```sql
SELECT … FROM items_fts f JOIN items i ON i.id = f.rowid
WHERE items_fts MATCH ?
  [ AND i.kind = ? ]
  [ AND i.pinned = 1 ]
ORDER BY i.pinned DESC, bm25(items_fts) ASC, i.last_used_at DESC
LIMIT ?
```

The match expression is built defensively — each whitespace-separated user token is alphanumeric-stripped, quoted, and suffixed with `*` for prefix search. That keeps user input from breaking FTS syntax.

## Retention enforcement

Two operations, both cheap and pinned-aware:

```
storage::enforce_limit(max_items) -> deleted_image_paths
storage::prune_older_than(days)   -> deleted_image_paths
```

Run at startup and on every capture. Image files referenced by deleted rows are unlinked from disk by the caller.

## File-system layout

```
~/Library/Application Support/com.clipboarder.app/
├── clipboarder.sqlite       # data + FTS index
├── clipboarder.sqlite-wal   # WAL
├── clipboarder.sqlite-shm   # shared memory file
├── settings.json
├── images/<hash16>.png      # captured PNG payloads
├── app_icons/<bundle>.png   # cached app icons
└── url_meta/<hash12>.json   # cached OG metadata
```

## Migrations

The current schema is forward-only. New columns are added via `ALTER TABLE ... ADD COLUMN ... DEFAULT NULL` calls wrapped in `let _ = ...` (so the column-already-exists error is silently ignored). When a column needs to be split or removed, we'll introduce a real migration ledger (probably as a `schema_version` table) at the same time.
