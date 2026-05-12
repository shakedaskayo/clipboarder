# `clipboarder copy`

Put a history item back onto the macOS clipboard. Does **not** synthesize ⌘V — use the GUI overlay if you want to paste-back into the previously-focused app.

## Synopsis

```bash
clipboarder copy <id>
```

## Behavior

- For text-shaped items (text, url, repo, code, color, email, music, video, file, pdf): writes the `content` to `NSPasteboard` as text.
- For image items (kind `image`): reads the cached PNG and writes it to `NSPasteboard` as an image.

After running, your next ⌘V (anywhere) pastes the item.

## Examples

```bash
# Re-copy the most recent code snippet
clipboarder list --kind code --limit 1 --json | jq '.[0].id' | xargs clipboarder copy

# Pipe an item's content through pbpaste-equivalent flow
clipboarder copy 42 && pbpaste > out.txt
```

For text content, an equivalent stdout-pipe is often simpler:

```bash
clipboarder show 42 | pbcopy
```

The CLI's `copy` is preferred when you need image fidelity or when you don't want to spawn `pbcopy`.

## Exit codes

| Exit | Meaning |
|------|---------|
| 0 | Wrote to NSPasteboard |
| 1 | Item id not found |
| 3 | Storage / clipboard error |
