# `clipboarder stats`

Print database statistics.

## Synopsis

```bash
clipboarder stats [--json]
```

## Output (human)

```
items:    1287
pinned:   14
by kind:
  code      87
  color     12
  email     3
  file      9
  image     38
  music     14
  pdf       2
  repo      203
  text      481
  url       418
  video     20
db:       /Users/you/Library/Application Support/com.clipboarder.app/clipboarder.sqlite
db size:  4194304 bytes
```

## Output (JSON)

```bash
clipboarder stats --json
```

```json
{
  "total": 1287,
  "pinned": 14,
  "by_kind": {
    "code": 87,
    "color": 12,
    "email": 3,
    "file": 9,
    "image": 38,
    "music": 14,
    "pdf": 2,
    "repo": 203,
    "text": 481,
    "url": 418,
    "video": 20
  },
  "db_path": "/Users/you/Library/Application Support/com.clipboarder.app/clipboarder.sqlite",
  "db_size_bytes": 4194304
}
```

## Notes

- `db_size_bytes` is the main SQLite file only — not the WAL/SHM sidecars or the `images/` and `url_meta/` cache directories. To see total on-disk footprint, use `du -sh "$(dirname "$(clipboarder stats --json | jq -r .db_path)")"`.
- `total` includes both pinned and non-pinned items.
