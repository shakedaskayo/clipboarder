# `clipboarder pin` / `unpin`

Pin and unpin items. Pinned items always sort to the top of the list, survive `clear`, and are never affected by history limits or auto-clear.

## Synopsis

```bash
clipboarder pin   <id>      # alias: star
clipboarder unpin <id>      # alias: unstar
```

## Behavior

- Pinning an already-pinned item is a no-op (exit 0).
- Unpinning an already-unpinned item is a no-op (exit 0).
- Item ids that don't exist exit 1 with a stderr message.

## Examples

```bash
# Find the latest GitHub repo URL and pin it
clipboarder list --kind repo --limit 1 --json | jq '.[0].id' | xargs clipboarder pin

# Pin every item that mentions "deploy"
clipboarder search deploy --json | jq -r '.[].id' | xargs -n1 clipboarder pin

# Unpin everything in one shot
clipboarder list --kind pinned --json | jq -r '.[].id' | xargs -n1 clipboarder unpin
```

## Exit codes

| Exit | Meaning |
|------|---------|
| 0 | Pinned / unpinned (or already in that state) |
| 1 | Item id not found |
| 3 | Storage error |
