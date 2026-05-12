# `clipboarder watch`

Stream newly-captured items as JSON Lines on stdout. One row per line. Runs forever (or until `SIGTERM`).

## Synopsis

```bash
clipboarder watch [--kind K]
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `-k`, `--kind <K>` | `all` | Only emit items of this kind. |

## Output

Each new clipboard item produces one line of compact JSON:

```json
{"id":17,"kind":"url","content":"https://anthropic.com","preview":"https://anthropic.com","source_app":"Safari","source_app_id":"com.apple.Safari","meta":"anthropic.com","image_path":null,"pinned":false,"size":21,"created_at":1778619961050,"last_used_at":1778619961050}
{"id":18,"kind":"code","content":"…","preview":"…","source_app":"VS Code","source_app_id":"com.microsoft.VSCode","meta":"typescript","image_path":null,"pinned":false,"size":48,"created_at":1778619971200,"last_used_at":1778619971200}
```

Polling interval: 500 ms.

## How it works

`watch` reads the highest `id` in your selected kind, sleeps 500 ms, then queries for any rows with a higher id. New rows are sorted by id and emitted in order. There's no event subscription — the CLI doesn't need the GUI to be running.

## Examples

```bash
# Log everything to a JSONL file
clipboarder watch > ~/clipboard.jsonl &

# Notify on every new URL (using `terminal-notifier`)
clipboarder watch --kind url | while read -r line; do
  url=$(echo "$line" | jq -r .content)
  terminal-notifier -title "New URL captured" -message "$url"
done

# Pipe new code snippets into a tagged file
clipboarder watch --kind code | jq -r '"--- " + .meta + "\n" + .content' >> code-stream.txt
```

## Exit / cleanup

`watch` keeps running until you `Ctrl-C` it or send `SIGTERM`. Buffered output is flushed on every emit, so consumers see new lines immediately.

If clipboarder isn't running, `watch` still works — it reads directly from the SQLite DB. But you won't get new rows unless something else (the GUI watcher, or `clipboarder add`) is inserting.
