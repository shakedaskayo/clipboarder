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

The CLI exposes these subcommands. Pass `--json` on anything that returns data so you get a stable schema.

| Command | Purpose | Read-only? |
|---------|---------|------------|
| `clipboarder list [--limit N] [--kind K] --json` | Most recent items | yes |
| `clipboarder search "<query>" [--limit N] [--kind K] --json` | Full-text search with bm25 ranking | yes |
| `clipboarder show <id> --json` | Full content of one item | yes |
| `clipboarder stats --json` | Counts by kind, db size | yes |
| `clipboarder add [text] [--kind K] [--source S] [--copy] --json` | Ingest an item (stdin if `text` omitted) | no |
| `clipboarder pin <id>` / `unpin <id>` | Star/unstar | no |
| `clipboarder delete <id>` | Remove one item | no |
| `clipboarder copy <id>` | Put item on system pasteboard (no paste-back) | no |
| `clipboarder clear -y` | Remove all non-pinned items | no |

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
clipboarder search "pull" --kind repo --limit 5 --json
```

Then for each item where `meta == "github"` and the URL path matches `/pull/`, present the title + URL.

### "What was that hex color I had?"

```bash
clipboarder list --kind color --limit 5 --json
```

### "Ingest this for me"

```bash
echo "<content>" | clipboarder add --source "claude" --json
# emits {"id": 42, "inserted": true, "kind": "..."}
```

Prefer ingesting via stdin so multi-line content is preserved.

### "Save this so I don't lose it"

```bash
# First find or add the item, then pin by id
clipboarder pin 42
```

### "Show me my last 5 things"

```bash
clipboarder list --limit 5 --json
```

## Privacy & best practices

- **Don't dump the whole history into your context.** Use `--limit` and `--kind` filters. Default to 10 items max unless the user asks for more.
- **Prefer `search` over `list`** when the user has a concrete query — bm25 ranking returns relevant items in 1–2 ms.
- **Don't echo passwords or secrets back to the user.** If you see an item that looks like a credential (long random string, key=value with sensitive name, JWT), summarize ("found a token-like item") instead of pasting it. The user already has a password manager — you're not it.
- **Excluded apps are respected at capture time.** Anything from apps the user added to Privacy exclusions (e.g. 1Password) was never captured, so you can trust the history to be intentional.
- **`add` is intentional**: only ingest content the user explicitly asked you to save. Don't auto-ingest summaries or intermediate work.

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
