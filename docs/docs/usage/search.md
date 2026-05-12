# Search & filters

clipboarder's search box is a thin frontend over a SQLite FTS5 index.

## How the index works

Every captured item lands in the `items` table plus a mirrored FTS5 virtual table `items_fts`. Three columns are indexed:

- `content` — the full clipboard payload
- `preview` — the single-line preview
- `meta` — the kind-specific tag (e.g. host for URLs, language for code, format for colors)

Triggers keep the FTS index in sync on every insert/update/delete.

## What you can search for

- **Words** — `react`, `auth`, `2024` — case-insensitive substring match
- **Multi-word** — `auth react` — both terms must appear (AND)
- **Prefix** — clipboarder appends `*` to every token automatically, so typing `anth` finds `anthropic.com`

Searches are debounced 60 ms after the last keystroke. Results are ranked by:

1. Pinned items first
2. `bm25(items_fts)` score (lower is more relevant)
3. `last_used_at` desc

## Filters

The chips below the search bar restrict by kind:

| Chip | Filters to |
|------|------------|
| **All** | Everything (default) |
| **Pinned** | Items you've starred |
| **Text** | Plain text without classification |
| **Links** | Generic URLs |
| **Repos** | GitHub / GitLab / Bitbucket / Codeberg / Gist URLs |
| **Code** | Detected as code (with language guess) |
| **Images** | Bitmap captures (PNG/screenshot) |
| **Colors** | `#hex`, `rgb()`, `hsl()` |
| **Music** | Spotify / Apple Music / YT Music / SoundCloud / Bandcamp |
| **Video** | YouTube / Vimeo / Twitch |
| **PDFs** | `.pdf` files |
| **Emails** | `user@host.tld` |
| **Files** | Other file paths |

Combining a filter with a search query narrows further — e.g. **Repos** + `tauri` shows only repo URLs whose host/title matches `tauri`.

## Performance

On a database with 5,000 items the cold-cache search is sub-5 ms; subsequent searches are sub-ms.
