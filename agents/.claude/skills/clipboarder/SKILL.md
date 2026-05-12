---
name: clipboarder
description: Search, retrieve, and add items to the user's local macOS clipboard history. Use this when the user references something they previously copied, asks "what did I just have on my clipboard", needs to search across the last N copies, or wants you to ingest content into their clipboard history for future use.
allowed-tools: Bash
---

# clipboarder skill

clipboarder is a local macOS clipboard manager. Everything the user has copied — text, URLs, repo links, code, colors, emails, files, PDFs, music/video links — is captured into a SQLite + FTS5 store and exposed via the `clipboarder` CLI on the user's PATH.

All data is local. No network calls happen when you query.

## When to use this

Trigger this skill if the user says (or paraphrases) any of:

- "What did I just copy?" / "What's on my clipboard?"
- "Find that link/code/color/email I had a minute ago"
- "Search my clipboard for X"
- "I copied a github URL recently, what was it"
- "Add this to my clipboard for later"
- "Show me my last N clipboard items"
- "Pin this so I don't lose it"

If the user is just asking you to put something on the macOS pasteboard *right now* (not retrieve from history), use `pbcopy` instead — it's faster.

## Commands you should know

The CLI is on the user's PATH as both `clipboarder` (full name) and `cb` (short alias). Prefer `cb` in your tool calls — it's shorter and easier to compose. Pass `--json` on anything that returns data so you get a stable schema.

**Pipe one-liners (use these first — they're the most powerful and easiest):**

| Command | Purpose |
|---------|---------|
| `echo "X" \| cb cp [--source NAME] --json` | Ingest stdin into history + system clipboard. Best for "save this for me". |
| `cb p` | Print most recent item content (1 line for most kinds; full body for code/text). |
| `cb p --grep "<query>"` | Most recent item matching FTS query. Sub-ms. **Prefer this over `cb search` + `cb show`.** |
| `cb p --kind <K> --grep "<query>"` | Same, restricted to one kind. |
| `cb p N` | Nth most recent (1-indexed). |
| `cb p --all --kind <K>` | All items of a kind, one body per line. |
| `cb p --json` | Full row as JSON instead of just the content body. |
| `cb pop` | Print + delete the most recent item. |

**Agent-friendly flags (combine freely with the commands above):**

| Flag | Effect | When to use |
|------|--------|-------------|
| `--compact` | Minimal JSON: `{id, kind, content, meta}` only. ~40% fewer tokens than the full row. | **Default to this for any agent-facing call.** |
| `--max-bytes N` | Truncate each item's content to N bytes at a UTF-8 char boundary, append `…`. | When dumping multiple items into your context. |
| `--since 30s\|5m\|1h\|2d\|1w` | Only items used within the window. | "What did I just copy" / "recent" queries. |
| `--no-secrets` | Items that look like API keys / OAuth tokens / JWTs / private keys are replaced with `[redacted: <kind>]`. | **Always set this** unless the user explicitly asked you to see the raw content. |
| `--snippet N` (search / `p --grep` only) | Replace content with an N-byte window around the matching token. | When the user wants context, not the whole snippet. |

A typical agent-facing call combines several:

```bash
cb p --grep "react" --kind repo --since 1d --no-secrets --compact --max-bytes 200 --json
```

**Structured queries:**

| Command | Purpose | Read-only? |
|---------|---------|------------|
| `cb list [--limit N] [--kind K] --json` | Most recent items | yes |
| `cb search "<query>" [--limit N] [--kind K] --json` | Full-text search with bm25 ranking | yes |
| `cb show <id> --json` | Full row of one item | yes |
| `cb stats --json` | Counts by kind, db size | yes |

**Mutations (use sparingly):**

| Command | Purpose |
|---------|---------|
| `cb pin <id>` / `cb unpin <id>` | Star/unstar |
| `cb delete <id>` | Remove one item |
| `cb copy <id>` | Put item on system pasteboard |
| `cb clear -y` | Remove all non-pinned items |

## Item kinds

Every item is auto-classified at capture time. Filtering by `--kind` narrows results sharply.

- `text` — plain text
- `url` — generic URL
- `repo` — GitHub / GitLab / Bitbucket / Codeberg / Gist URL
- `email` — `user@host.tld`
- `code` — code with optional language tag in `meta` (rust, python, typescript, sql, shell, …)
- `color` — `#hex`, `rgb()`, or `hsl()` with format in `meta`
- `music` — Spotify, Apple Music, YouTube Music, SoundCloud, Bandcamp
- `video` — YouTube, Vimeo, Twitch
- `image` — bitmap captured to disk
- `file` — file path on disk
- `pdf` — single .pdf file path

## JSON schema (search / list / show)

```json
{
  "id": 17,
  "kind": "repo",
  "content": "https://github.com/owner/repo/pull/123",
  "preview": "https://github.com/owner/repo/pull/123",
  "source_app": "Safari",
  "source_app_id": "com.apple.Safari",
  "meta": "github",
  "image_path": null,
  "pinned": false,
  "size": 43,
  "created_at": 1778619961050,
  "last_used_at": 1778619961050
}
```

- `id` — stable integer; use with `show`, `pin`, `delete`
- `created_at` / `last_used_at` — unix milliseconds; `last_used_at` is bumped each time the same content is re-copied
- `image_path` — non-null for `kind: image`; reading the file is your job
- `meta` — kind-specific: host for url, language for code, color format for color, platform for music/video/repo

## Examples

### "Find that PR I copied recently"

```bash
cb p --kind repo --grep "pull" --json
```

This returns the row most recently used that's both a repo URL and matches "pull". One shot, ranked by bm25 + recency.

### "What was the last hex color I had?"

```bash
cb p --kind color
```

Returns just the content (e.g. `#7c8cff`) to stdout. Use `--json` if you also need the parsed `meta` field.

### "Ingest this for me"

```bash
echo "<content>" | cb cp --source "claude" --json
# emits {"id": 42, "inserted": true, "kind": "..."}
```

`cb cp` writes to **both** the clipboarder history and the macOS pasteboard. Add `--no-clipboard` if you only want the history side. Prefer stdin so multi-line content is preserved.

### "Open the last URL I copied in my browser"

```bash
open "$(cb p --kind url)"
```

### "Save this so I don't lose it"

```bash
# Ingest + pin in one shot
ID=$(echo "<content>" | cb cp --source "claude" --json | jq .id)
cb pin "$ID"
```

### "Show me my last 5 things"

```bash
cb list --limit 5 --json
```

### "What did I just copy?"

```bash
cb p
# or for a full row:
cb p --json
```

## Privacy & best practices

- **Default to `--no-secrets --compact --max-bytes 400`** on every read. The clipboarder CLI does the secret detection for you (API keys, OAuth tokens, JWTs, private key blocks, `password=` patterns); just trust the flag.
- **Don't dump the whole history into your context.** Use `--limit` and `--kind` filters. Default to 10 items max unless the user explicitly asks for more.
- **Prefer `cb p --grep`** over `cb search` + `cb show`. One round-trip instead of two, and you get just the most relevant content rather than the full row.
- **Excluded apps are respected at capture time.** Anything from apps the user added to Privacy exclusions (e.g. 1Password) was never captured, so you can trust the history to be intentional.
- **`add` is intentional**: only ingest content the user explicitly asked you to save. Don't auto-ingest summaries or intermediate work. Always pass `--source <your name>` so the user can see what came from you in the GUI.

## Error codes

| Exit | Meaning |
|------|---------|
| 0    | Success |
| 1    | Item id not found (for show/pin/delete) |
| 2    | Argument error (bad flag, empty stdin) |
| 3    | Storage error (DB locked, disk full, schema mismatch) |

If you see exit 3, surface the underlying error to the user — it usually means clipboarder isn't installed or the data dir was deleted.

## When clipboarder isn't installed

If `command -v clipboarder` returns empty, the user hasn't installed clipboarder yet. Suggest:

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
```

Then the CLI is on their PATH and this skill works.
