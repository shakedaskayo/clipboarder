# Keyboard shortcuts

clipboarder is designed so you can do everything without touching the mouse.

## Global

| Shortcut | Action |
|----------|--------|
| `⌘⇧V` *(default)* | Show / hide the overlay. Reconfigurable in Settings. |

## Inside the overlay

| Shortcut | Action |
|----------|--------|
| `↑` / `↓` | Move selection up / down |
| `^N` / `^P` | Same as ↓ / ↑ (Emacs-style) |
| `Enter` | Paste selected item into the previously-focused app |
| `⌘1` – `⌘9` | Quick-paste the Nth item in the visible list |
| `⌘K` | Clear search query |
| `⌘,` | Open Settings panel |
| `Esc` | Hide the overlay (or back-out from Settings) |

## Inside Settings

| Shortcut | Action |
|----------|--------|
| `Esc` | Return to the main view |

## Recording a hotkey

In **Settings → Hotkey**, click **Record**. clipboarder listens for the next combination you press and saves it once you release. The recorder requires at least one modifier (`⌘`, `⌥`, `⌃`, or `⇧`) plus a non-modifier key — bare letters aren't allowed (you'd lose typing).

Press `Esc` while in recording mode to cancel.

## What if `⌘⇧V` is already taken?

Some apps bind `⌘⇧V` to "Paste and Match Style". The global hotkey wins over app-level shortcuts on macOS, so clipboarder gets the press. If you'd rather not lose Match Style in your editor, rebind clipboarder to `⌘⌥V`, `⌃Space`, or anything else.
