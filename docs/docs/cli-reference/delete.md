# `clipboarder delete` / `clear`

Remove one or all non-pinned items.

## Synopsis

```bash
clipboarder delete <id>     # alias: rm
clipboarder clear [-y]
```

## `delete <id>`

Removes a single item. If the item has an associated image file on disk (kind `image`), the file is unlinked too.

```bash
clipboarder delete 42
clipboarder rm 42
```

## `clear`

Removes **all non-pinned items**. Pinned items survive. Image files associated with deleted rows are unlinked.

By default `clear` prompts for confirmation on stderr. Pass `-y` / `--yes` to skip the prompt (use this in scripts).

```bash
clipboarder clear            # interactive: prompts y/N
clipboarder clear -y         # non-interactive
clipboarder clear --yes      # same as -y
```

## Examples

```bash
# Nuke everything older than a week (in shell)
ONE_WEEK_AGO=$(($(date +%s) * 1000 - 604800000))
clipboarder list --limit 5000 --json | \
  jq -r --argjson cutoff $ONE_WEEK_AGO \
    '.[] | select(.last_used_at < $cutoff and (.pinned | not)) | .id' | \
  xargs -n1 clipboarder delete
```

(For most users, the *Auto-clear after* setting in the GUI does this automatically.)

## Exit codes

| Exit | Meaning |
|------|---------|
| 0 | Deleted (or cleared) |
| 1 | Item id not found (delete only) |
| 2 | User declined the confirmation prompt |
| 3 | Storage error |
