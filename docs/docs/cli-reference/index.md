# CLI Reference

clipboarder ships a single binary that runs in two modes:

- **No args** → GUI overlay (the floating window you summon with `⌘⇧V`)
- **With a subcommand** → CLI

The installer symlinks `/usr/local/bin/clipboarder` (or `~/.local/bin/clipboarder` if the former isn't writable) to the .app's binary, so `clipboarder list` works from any shell.

## Overview

```bash
clipboarder --help

clipboarder — CLI for the clipboard manager.
Search, ingest, pin, paste-back, and stream your local clipboard history.
Designed for shell pipelines and AI agents.

Usage: clipboarder <COMMAND>

Commands:
  list    List most recent items                     (alias: ls)
  search  Full-text search the clipboard history     (alias: find)
  show    Print one item's full content              (alias: cat, get)
  add     Add a new item from stdin or argument      (alias: ingest)
  pin     Pin an item                                (alias: star)
  unpin   Unpin an item                              (alias: unstar)
  delete  Delete an item                             (alias: rm)
  clear   Clear all non-pinned items
  copy    Copy item to the macOS clipboard
  stats   Print database statistics
  watch   Stream new items as JSON Lines on stdout
```

## Quick recipes

### Copy the latest captured URL to the pasteboard

```bash
clipboarder copy "$(clipboarder list --kind url --limit 1 --json | jq '.[0].id')"
```

### Pin every item that mentions "deploy"

```bash
clipboarder search deploy --json | jq -r '.[].id' | xargs -n1 clipboarder pin
```

### Stream new copies into a log file

```bash
clipboarder watch >> ~/clipboard-log.jsonl &
```

### Ingest the output of any command

```bash
git log --oneline -10 | clipboarder add --source "git"
```

### Show stats in a one-liner

```bash
clipboarder stats --json | jq '{total, pinned, by_kind}'
```

## See also

- [Commands → list](list.md)
- [Commands → search](search.md)
- [Commands → show](show.md)
- [Commands → add](add.md)
- [Agents integration](../agents/index.md)
