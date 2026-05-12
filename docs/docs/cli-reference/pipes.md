# Pipe-friendly one-liners

clipboarder ships a short alias — **`cb`** — and two purpose-built subcommands that turn your shell into a clipboard pipeline.

> Think `pbcopy` / `pbpaste`, but with **history**, **search**, and **kind filters**.

## The two stars: `cb cp` and `cb p`

### `cb cp` — stdin → history + macOS clipboard

```bash
echo "remember this" | cb cp
git log --oneline -5 | cb cp           # multi-line works
date | cb cp --kind text --source cron # tag where it came from
```

Behavior:

- Reads stdin
- Classifies + dedups via SHA-256
- Writes to **both** the clipboarder history **and** the macOS clipboard (so any ⌘V uses it immediately)
- Add `--no-clipboard` to only persist (don't touch the system pasteboard)
- Add `--json` to get `{"id":42,"inserted":true,"kind":"..."}`

### `cb p` — print recent items to stdout

```bash
cb p                          # most recent item's content
cb p 2                        # 2nd most recent
cb p --kind url               # most recent URL
cb p --grep "react"           # most recent item matching "react"
cb p --kind repo --grep tauri # composed: most recent repo URL matching tauri
cb p --copy                   # ↑ AND put on macOS pasteboard
cb p --all --kind code        # all code items, one per line
cb p --json                   # full row as JSON
```

Aliases: `cb paste`, `cb last`.

## Round-trip examples

### Capture a JSON blob into history, fetch it later

```bash
curl -s api.github.com/users/octocat | cb cp --source github
# … later …
cb p --grep octocat | jq .public_repos
```

### Pipe the last URL you copied straight into a browser

```bash
open "$(cb p --kind url)"
```

### Most recent color, formatted three ways

```bash
cb p --kind color --json | jq '{hex: .content, format: .meta}'
```

### Search history, copy the best match to system clipboard

```bash
cb p --grep "auth token" --copy
```

### Pop the head of the queue

```bash
cb pop          # print most recent + remove it
cb pop --kind code   # most recent code item, remove it
```

### Stream new copies into a log

```bash
cb watch >> ~/clipboard.log.jsonl &
# every clipboard event becomes one JSON line
```

### Tag what your scripts add

```bash
echo "build $(git rev-parse --short HEAD) ok" | cb cp --source build-script
```

## How it pairs with `pbcopy`/`pbpaste`

| Goal | clipboarder | macOS built-in |
|------|-------------|----------------|
| stdin → clipboard | `cb cp` | `pbcopy` |
| clipboard → stdout | `cb p` (also history-aware!) | `pbpaste` |
| Persist for later | `cb cp` | ✗ |
| Search history | `cb p --grep …` | ✗ |
| Filter by kind | `cb p --kind url` | ✗ |
| Multi-paste workflows | `cb p N` | ✗ |

`cb cp` is a drop-in `pbcopy` replacement for users who want every copy to be searchable forever. Aliases play well in `.zshrc`:

```bash
# Optional: alias pbcopy + pbpaste to the clipboarder versions
alias pbcopy='cb cp'
alias pbpaste='cb p'
```

## For AI agents

The Claude skill at [`SKILL.md`](https://github.com/shakedaskayo/clipboarder/blob/main/agents/.claude/skills/clipboarder/SKILL.md) leans on these specifically — `cb p --grep` is recommended over a multi-step `search → show` flow. One-liner installer:

```bash
mkdir -p ~/.claude/skills/clipboarder && \
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \
    -o ~/.claude/skills/clipboarder/SKILL.md
```

Then Claude Code auto-loads the skill on every session — no config edit, no plugin install. Same one-liner is in the [Agents Integration](../agents/index.md) page.
