# `clipboarder add`

Ingest a new item into the clipboard history from stdin or an argument.

## Synopsis

```bash
clipboarder add [TEXT] [--kind K] [--source S] [--copy] [--json]
clipboarder ingest [TEXT] ...
```

## Options

| Flag | Description |
|------|-------------|
| `[TEXT]` | Optional positional content. If omitted, read from stdin. |
| `--kind <K>` | Override auto-classification. One of `text`, `url`, `email`, `code`, `color`, `repo`, `music`, `video`, `file`, `pdf`. |
| `--source <S>` | Tag where the content came from. Shows in the row meta. |
| `--copy` | Also write to the macOS pasteboard (the GUI watcher will then capture it normally). |
| `--json` | Emit `{"id":42,"inserted":true,"kind":"…"}` on success (default: silent). |

## Dedup

clipboarder hashes content with SHA-256. Adding the same content twice doesn't create a second row — the existing row's `last_used_at` is bumped instead, and `inserted` in the JSON output is `false`.

## Classification

By default, `add` runs the same auto-classifier the GUI watcher uses. URLs become `url` or `repo` or `music`; `#hex` becomes `color`; code-shaped text becomes `code` with a language guess. Pass `--kind` to override.

## Examples

```bash
# From an argument
clipboarder add "remember the milk"

# From stdin
git log --oneline -5 | clipboarder add --source git

# As a specific kind
echo "TODO(later): wire up rate limits" | clipboarder add --kind text

# Also write to system clipboard (so other apps see it immediately)
echo "shared snippet" | clipboarder add --copy

# Get the new id back
echo "tagged note" | clipboarder add --source claude --json | jq .id
```

## Exit codes

| Exit | Meaning |
|------|---------|
| 0 | Inserted or bumped existing row |
| 2 | Empty stdin / no content given |
| 3 | Storage error |
