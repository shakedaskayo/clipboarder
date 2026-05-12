# `clipboarder list`

List the most recent clipboard items.

## Synopsis

```bash
clipboarder list [--limit N] [--kind K] [--json]
clipboarder ls   [--limit N] [--kind K] [--json]
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `-l`, `--limit <N>` | `20` | How many items to return. |
| `-k`, `--kind <K>`  | `all` | Restrict to one kind. See [content types](../usage/content-types.md). Use `pinned` for the pin list. |
| `--json` | off | Emit JSON instead of a table. |

## Output (table)

```
   ID  KIND       AGE  SOURCE          PREVIEW
   17  url         5s  Safari          https://anthropic.com/news/claude-3-5-sonnet
   16  repo        2m  Safari          https://github.com/tauri-apps/tauri
   15  code        4m  VS Code         const fetchUser = async (id: string) => { …
```

- `ID` is stable across reboots; use it with `show`, `pin`, `delete`, `copy`.
- `AGE` is `last_used_at` (bumped on re-copy or paste-back).
- `SOURCE` is the localized app name where the copy happened.
- Pinned items show a leading ★.

## Output (JSON)

```bash
clipboarder list --limit 2 --json
```

```json
[
  {
    "id": 17,
    "kind": "url",
    "content": "https://anthropic.com/news/claude-3-5-sonnet",
    "preview": "https://anthropic.com/news/claude-3-5-sonnet",
    "source_app": "Safari",
    "source_app_id": "com.apple.Safari",
    "meta": "anthropic.com",
    "image_path": null,
    "pinned": false,
    "size": 44,
    "created_at": 1778619961050,
    "last_used_at": 1778619961050
  },
  …
]
```

## Examples

```bash
clipboarder list                       # 20 most recent items, table
clipboarder list --limit 5             # 5 most recent
clipboarder list --kind url            # only URLs
clipboarder list --kind code --json    # only code, as JSON
clipboarder list --kind pinned         # pinned items
```
