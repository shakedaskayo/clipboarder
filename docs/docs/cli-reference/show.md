# `clipboarder show`

Print the full content of one item by id.

## Synopsis

```bash
clipboarder show <id> [--json]
clipboarder cat  <id> [--json]
clipboarder get  <id> [--json]
```

## Options

| Flag | Description |
|------|-------------|
| `<id>` | Numeric id from `list` or `search`. |
| `--json` | Print the full row as JSON (default: just the content body). |

## Output

By default `show` prints only the `content` field, with a trailing newline. This makes piping easy:

```bash
clipboarder show 42 | pbcopy        # copy to system clipboard via stdout pipe
clipboarder show 42 > snippet.txt
clipboarder show 42 | jq .          # this fails — content isn't JSON-encoded
```

With `--json`, you get the full schema:

```bash
clipboarder show 42 --json
{
  "id": 42,
  "kind": "code",
  "content": "fn main() { println!(\"hello\"); }",
  …
}
```

## Exit codes

| Exit | Meaning |
|------|---------|
| 0 | Found, printed |
| 1 | Item id not found |
| 3 | Storage error |
