# `clipboarder search`

Full-text search the clipboard history. Uses SQLite FTS5 with bm25 ranking — sub-millisecond across thousands of items.

## Synopsis

```bash
clipboarder search <query> [--limit N] [--kind K] [--json]
clipboarder find   <query> [--limit N] [--kind K] [--json]
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `<query>` | — | Free-text query. Multiple whitespace-separated words = AND. Each token is prefix-matched. |
| `-l`, `--limit <N>` | `20` | How many items to return. |
| `-k`, `--kind <K>` | `all` | Restrict to one kind. |
| `--json` | off | Emit JSON instead of a table. |

## How tokens are processed

Each whitespace-separated word in the query is:

1. Stripped to alphanumerics + `-_.`
2. Quoted as a single FTS5 phrase
3. Suffixed with `*` for prefix matching

So `clipboarder search "anth"` matches `anthropic.com`, and `clipboarder search "react hooks"` matches anything containing both `react*` and `hooks*`.

## Ranking

Results are sorted by:

1. **Pinned items first** (pinned ★ above unpinned)
2. **bm25 score** (lower is more relevant)
3. **`last_used_at` desc** (recency tiebreaker)

## Examples

```bash
clipboarder search github
clipboarder search "pull request" --kind repo
clipboarder search "#ff" --kind color
clipboarder search anth --json | jq '.[0].content'
```

## See also

- [list](list.md) when you don't have a query
