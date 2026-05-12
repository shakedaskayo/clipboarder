# Using clipboarder

clipboarder is intentionally keyboard-first. You shouldn't have to take your hands off the keyboard to find and paste a piece of history.

## The overlay

The clipboarder overlay is a single 880×520 window with five regions:

```
┌─ Search ──────────────────────────────────────┐
│ 🔍  Search clipboard…              ↑↓ ↵       │
├─ Filter chips ────────────────────────────────┤
│ All  Pinned  Text  Links  Repos  Code  …      │
├─ List ──────────────────┬─ Preview ───────────┤
│ ⌘1  📄  hello world     │ TEXT · 11 chars     │
│ ⌘2  🔗  github.com/…   │                     │
│ ⌘3  🎨  #7c8cff        │ hello world         │
│ ⌘4  📁  ~/Downloads/…  │                     │
├─ Footer ──────────────────────────────────────┤
│ ● clipboarder   ⌘1-9 ⌘K esc        24 items ⚙ │
└───────────────────────────────────────────────┘
```

- **Search** — type to filter; clear with `⌘K`
- **Filter chips** — narrow by content kind, plus *All* and *Pinned*
- **List** — most-recently-used at the top (pinned always above non-pinned)
- **Preview** — rich, kind-aware: color swatch, image thumbnail, PDF embed, music/repo cards
- **Footer** — keyboard hints, item count, and the Settings gear

## Read these next

- [Search & filters](search.md) — how the indexer works and what tokens it accepts
- [Keyboard shortcuts](shortcuts.md) — the full list
- [Content types](content-types.md) — every kind clipboarder recognizes and the previews you'll see
- [Pinning & history](pinning.md) — how items stay around (or get cleaned up)
