<p align="center">
  <img src="docs/docs/assets/logo.png" alt="clipboarder" width="180">
</p>

<h3 align="center">A clipboard for humans <em>and</em> coding agents — searchable history, smart classification, native macOS UI, scriptable CLI.</h3>

<p align="center">
  <a href="https://github.com/shakedaskayo/clipboarder/actions/workflows/ci.yml"><img src="https://github.com/shakedaskayo/clipboarder/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/shakedaskayo/clipboarder/releases/latest"><img src="https://img.shields.io/github/v/release/shakedaskayo/clipboarder?include_prereleases&label=release&color=7c8cff" alt="Release"></a>
  <a href="https://github.com/shakedaskayo/clipboarder/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-5B8CA8.svg" alt="License"></a>
  <a href="https://shakedaskayo.github.io/clipboarder"><img src="https://img.shields.io/badge/docs-shakedaskayo.github.io%2Fclipboarder-7c8cff" alt="Docs"></a>
  <a href="https://github.com/shakedaskayo/clipboarder/stargazers"><img src="https://img.shields.io/github/stars/shakedaskayo/clipboarder?style=social" alt="GitHub Stars"></a>
</p>

<br>

**clipboarder** is a native macOS clipboard manager that captures everything you copy — text, links, images, code, colors, files, PDFs, music links, video links — and makes it searchable in milliseconds.

Press `⌘⇧V` from any app. Type. Hit Enter. The selected item is pasted back into whichever app you were in.

Built with **Rust** (Tauri 2) and **React**. Single ~10 MB binary. Zero dependencies. Stores history locally in SQLite with FTS5 full-text search. No cloud, no telemetry, no account.

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
```

<br>

## Why this matters for coding agents

The clipboard is the oldest piece of working memory on your computer. For 40 years it's been the glue between apps — copy an error from your terminal, paste it into Slack; copy a snippet from Stack Overflow, paste it into your editor; copy a PR URL from the browser, paste it into a doc. *You* use the clipboard as a scratch space between contexts dozens of times a day.

**Coding agents can't.** Claude Code, Codex, Cursor, and every other LLM-powered assistant lives inside its context window. It can't see what you just copied from your terminal. It can't put a generated command on your clipboard for you to paste somewhere else. It can't recall what you had on your clipboard ten minutes ago.

clipboarder fixes that. The same captured history that powers the GUI overlay is exposed to your agent through a tiny CLI (`cb cp` / `cb p`). **Your agent uses the clipboard the way you do** — read what's there, drop something on it for you to paste, search history, persist context across sessions.

```bash
# You copy a stack trace from your terminal.
# You switch to Claude Code and say: "fix the error I just copied"
#
# Claude runs:                                      # what it does
cb p                                                # reads your last clipboard entry
# … figures out the fix …
echo "cargo update -p tokio" | cb cp                # puts the fix on YOUR clipboard
#
# Claude replies: "I've put `cargo update -p tokio` on your clipboard — ⌘V into the terminal."
# You hit ⌘V. Done. No paste-into-Claude. No copy-from-Claude. Zero friction.
```

Other flows it unlocks:

- **"Find that PR URL I copied earlier"** → `cb p --kind repo --grep "auth"` returns the most recent matching GitHub link in <2 ms.
- **"Save this for me to share in Slack"** → agent writes its output via `cb cp` and you paste it wherever, whenever.
- **Persistent agent context** — the agent can `cb pin` items it wants to remember across sessions.
- **Privacy-aware by default** — apps you exclude (1Password, Bitwarden, etc.) are never captured, so agents can't read them either.

### One-liner Claude Code skill

```bash
mkdir -p ~/.claude/skills/clipboarder && \
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \
    -o ~/.claude/skills/clipboarder/SKILL.md
```

Claude Code auto-loads the skill on its next session. No plugin, no config edit. It triggers on phrases like *"what did I copy"*, *"find that link I had"*, *"save this for later"*.

For LangChain, OpenAI Assistants, Cursor, or any other harness — tool definitions, JSON schemas, privacy guidance, and secret-detection heuristics live in [docs / For agents](https://shakedaskayo.github.io/clipboarder/agents/).

<br>

## Screenshots

### The list

Every copy is classified, captured with its source-app icon, and ranked by recency + bm25. Filter chips above let you jump straight to Links, Repos, Code, Colors, Music, Video, PDFs.

<p align="center">
  <img src="docs/docs/assets/screenshots/main_v2.png" alt="Main view" width="100%">
</p>

### Rich previews per kind

#### Repos (GitHub / GitLab / Bitbucket / Codeberg)

Detect `<host>/<owner>/<repo>` URLs, fetch the OpenGraph card from the host, and render owner/repo with the right resource type — *Repository*, *Pull request #N*, *Issue #N*, *Commit \<sha\>*, *Release \<tag\>*, *File*, *Folder*, *Wiki*, *Actions*.

<p align="center">
  <img src="docs/docs/assets/screenshots/repo_v2.png" alt="Repo card with GitHub OG metadata" width="100%">
</p>

#### Music / video

Spotify, Apple Music, YouTube + YouTube Music, SoundCloud, Bandcamp, Vimeo, Twitch — each gets a branded card with platform glow and a one-click *Open in platform* button.

<p align="center">
  <img src="docs/docs/assets/screenshots/music_v2.png" alt="Spotify music card" width="100%">
</p>

#### Colors

Hex / rgb / hsl in any form gets a big swatch plus all three notations side-by-side for easy copy-paste into any tool.

<p align="center">
  <img src="docs/docs/assets/screenshots/color_v2.png" alt="Color swatch with HEX/RGB/HSL" width="100%">
</p>

#### Code

Heuristic-detected code (with language guess) renders in a styled monospace block. Shell one-liners get tagged `shell`.

<p align="center">
  <img src="docs/docs/assets/screenshots/code_v2.png" alt="Code preview" width="100%">
</p>

### Settings

Rebind the hotkey by recording any combo. Launch at login. Cap history size or auto-clear after N days. Add per-app exclusions (e.g. 1Password) so sensitive clipboard activity is never captured.

<p align="center">
  <img src="docs/docs/assets/screenshots/settings_v2.png" alt="Settings panel" width="100%">
</p>

<br>

## Features

| | |
|---|---|
| **Instant search** | SQLite FTS5 with bm25 ranking. Sub-millisecond results across thousands of items. |
| **Smart classification** | Every copy is auto-tagged at capture time: text, url, email, code, color, image, file, pdf, music, video. |
| **Rich previews** | Color swatches with HEX/RGB/HSL conversion, image thumbnails, inline PDF embed, music/video platform cards with parsed metadata. |
| **Source app icons** | Each row shows the real icon of the app you copied from — Safari, VS Code, Figma — extracted via NSWorkspace and cached. |
| **Quick-paste** | `⌘1`–`⌘9` paste the top 9 results directly into the previously-focused app, no extra keystrokes. |
| **Pinning** | Star any item. Pinned items stay forever and float to the top. |
| **Custom hotkey** | Record any combination. Defaults to `⌘⇧V`, swap to anything in Settings. |
| **Privacy mode** | Add app bundle IDs (e.g. `com.1password.1password`) to skip clipboard capture from sensitive apps. |
| **History limits** | Hard cap by item count (100 – unlimited) and auto-clear after 1 day / 1 week / 1 month / etc. |
| **Launch at login** | Optional macOS LaunchAgent registration via the autostart plugin. |
| **Menu-bar tray** | Always accessible — Show, Settings, Clear history, Quit. Templated icon adapts to your menu-bar theme. |
| **Floats above everything** | Joins all Spaces, stays above fullscreen apps. |
| **Local-only** | All data stays in `~/Library/Application Support/com.clipboarder.app/`. No network calls, no telemetry. |

<br>

## How It Works

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  NSPasteboard   │───▶│  Watcher (Rust)  │───▶│  SQLite + FTS5  │
│  (system-wide)  │    │  classify+hash   │    │  with triggers  │
└─────────────────┘    └──────────────────┘    └────────┬────────┘
                                                        │
┌─────────────────┐    ┌──────────────────┐    ┌────────▼────────┐
│   ⌘⇧V hotkey    │───▶│  Tauri commands  │◀───│  React frontend │
│  (CGEventTap)   │    │  (IPC bridge)    │    │  (Vite + TSX)   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                ▲
                                │ paste-back via CGEventPost
                                ▼
                         previously-focused app
```

- A Rust thread watches `NSPasteboard` change-count and reads every clipboard event.
- Content is classified (text / url / email / code / color / image / file / pdf / music / video), deduplicated via SHA-256, and persisted with FTS5 triggers keeping the search index in sync.
- The overlay window is a frameless transparent Tauri window that floats above other apps and joins every macOS Space.
- Selecting an item writes it back to `NSPasteboard`, hides the overlay, and synthesizes `⌘V` via `CGEventPost` into the previously-focused app.

<br>

## Install

**One-liner** (recommended):

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
```

The installer downloads the latest signed `.dmg` from GitHub Releases, mounts it, copies `clipboarder.app` to `/Applications`, and unmounts.

**Manual install:**

Download the latest `clipboarder_<version>_aarch64.dmg` from the [Releases page](https://github.com/shakedaskayo/clipboarder/releases/latest), open it, and drag `clipboarder` into `Applications`.

**From source:**

```bash
git clone https://github.com/shakedaskayo/clipboarder.git
cd clipboarder
make dmg          # produces src-tauri/target/release/bundle/dmg/clipboarder_*.dmg
```

### First launch

On first launch macOS will ask for:

1. **Accessibility permission** — required to synthesize `⌘V` into the focused app after pasting. Grant in *System Settings → Privacy & Security → Accessibility* and toggle clipboarder on. Until you do, *Copy to clipboard* still works but paste-back doesn't.
2. **Global hotkey** — `⌘⇧V` is registered automatically. Open Settings (gear icon or `⌘,`) to rebind.

<br>

## Quick Start

```bash
# 1. Install
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash

# 2. Launch (also auto-launches at login if enabled later)
open -a clipboarder

# 3. Use it
#    - Copy anything from any app
#    - Press ⌘⇧V from anywhere
#    - Type to search, ↑↓ to navigate
#    - Press Enter or ⌘1–⌘9 to paste
#    - Esc to close
```

<br>

## CLI — `clipboarder` and `cb`

The same binary that runs the GUI also runs as a CLI. `install.sh` puts **two symlinks** on your PATH: `clipboarder` (full name) and `cb` (short alias).

### Pipe ergonomics — the part that makes it powerful

`pbcopy` / `pbpaste`, but with history, search, and kind filters baked in:

```bash
echo "remember this"            | cb cp        # stdin → history + macOS clipboard
git log --oneline -5            | cb cp        # multi-line ingestion
curl -s api.github.com/users/me | cb cp --source github

cb p                                            # print most recent item
cb p --kind url                                 # most recent URL
cb p --grep "react"                             # most recent match for "react"
cb p --kind repo --grep tauri --copy            # composed + also put on pasteboard
cb p --all --kind code                          # every code item, one per line

open "$(cb p --kind url)"                       # open the last URL in your browser
cb p --grep "auth token" --copy                 # search → pasteboard in one step
```

> **Think of clipboarder as a local context database that your shell and AI agents can drive with one-liners.** Every copy you've ever made is searchable in sub-millisecond time, kind-filterable, and pipeable into any tool.

Drop these into your `~/.zshrc` for a `pbcopy` replacement:

```bash
alias pbcopy='cb cp'
alias pbpaste='cb p'
```

### Full subcommand list

```
cp / pipe             stdin → history + system pasteboard
p  / paste / last     Nth recent item → stdout (with --kind / --grep / --copy)
pop                   print + delete most recent

list / ls             recent items (table or --json)
search / find         FTS5 search with bm25 ranking
show / cat / get      full content of one item
add / ingest          ingest from stdin or arg (no pasteboard mutation)
pin / star, unpin     toggle star
delete / rm, clear    remove items
copy <id>             put a history item on macOS pasteboard
stats                 totals + by-kind breakdown + db size
watch                 stream new items as JSON Lines
```

Every command supports `--json` for machine-readable output. Full reference: <https://shakedaskayo.github.io/clipboarder/cli-reference/>.

### Shared server mode

clipboarder can run as a multi-namespace HTTP backend. Multiple clients connect with bearer tokens; each is scoped to its own namespace — content, FTS index, pins, and stats are fully isolated.

```bash
# Server side
clipboarder admin token create --namespace alice --label "Alice's MacBook"
# → tk_…  (paste this on the client)
clipboarder serve --bind 0.0.0.0:7474

# Client side (any machine)
export CLIPBOARDER_SERVER='http://your-server:7474'
export CLIPBOARDER_TOKEN='tk_…'
curl -s -H "Authorization: Bearer $CLIPBOARDER_TOKEN" "$CLIPBOARDER_SERVER/v1/items?limit=5"
```

REST endpoints: `/v1/health`, `/v1/whoami`, `/v1/items`, `/v1/items/:id`, `/v1/items/:id/pin`, `/v1/clear`, `/v1/stats`, `/v1/watch` (SSE).

Full deployment guide (Caddy / Nginx / systemd / launchd) at [docs / Server mode](https://shakedaskayo.github.io/clipboarder/server/). [27 integration tests](scripts/test-server.sh) gate the namespace isolation in CI.

### For AI agents — drop-in Claude Code skill

```bash
mkdir -p ~/.claude/skills/clipboarder && \
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \
    -o ~/.claude/skills/clipboarder/SKILL.md
```

Claude Code auto-loads the skill on every session. Now Claude understands phrases like *"what did I copy"*, *"find that link I had"*, *"save this for later"* and invokes `cb p --grep …` / `cb cp` for you.

For LangChain / OpenAI Assistants / any other harness, see [docs / For agents](https://shakedaskayo.github.io/clipboarder/agents/) — JSON schema, tool definitions, privacy guidance, secret-detection heuristics.

<br>

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘⇧V` | Show / hide the overlay (configurable) |
| `↑` / `↓` | Move selection |
| `Enter` | Paste selected item into previously-focused app |
| `⌘1`–`⌘9` | Quick-paste items 1 through 9 |
| `⌘K` | Clear search query |
| `⌘,` | Open Settings |
| `Esc` | Hide overlay (or back from Settings) |

<br>

## Configuration

clipboarder stores its config at `~/Library/Application Support/com.clipboarder.app/settings.json`:

```json
{
  "hotkey": "CommandOrControl+Shift+V",
  "launch_at_login": true,
  "max_items": 500,
  "auto_clear_days": 30,
  "excluded_apps": ["com.1password.1password", "com.agilebits.onepassword7"]
}
```

All settings are also editable in the in-app Settings panel.

<br>

## Architecture

```
clipboarder/
  src/                    React + TypeScript + Tailwind frontend
    components/           Row, Preview, Chips, Settings, HotkeyRecorder, Toggle, Select
    lib/                  api, types, hotkey parser, color parser, app-icon cache
  src-tauri/              Tauri 2 + Rust backend
    src/
      lib.rs              Bootstrap, hotkey registration, tray menu, window mgmt
      clipboard.rs        NSPasteboard watcher, classification, dedup
      classify.rs         text/url/email/code/color/file/pdf/music/video detection
      storage.rs          SQLite + FTS5, retention enforcement
      paste.rs            Write-back + ⌘V synthesis via CGEvent
      settings.rs         JSON-persisted user settings
      app_icons.rs        On-demand app-icon extraction + cache
      commands.rs         Tauri IPC command handlers
      macos.rs            NSWindow level, NSWorkspace bundle/icon lookup
  docs/                   MkDocs Material documentation site
  scripts/                make_icon.py (regenerates the app icon)
  .github/                Issue + PR templates, CI/release/docs workflows
```

<br>

## Development

```bash
make dev          # Vite + tauri dev with HMR
make build        # Release build (.app + .dmg)
make dmg          # Just the .dmg
make docs         # Local MkDocs server on :8000
make test         # cargo test + tsc --noEmit
make icon         # Regenerate the app icon from scripts/make_icon.py
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full setup walkthrough.

<br>

## Documentation

The full documentation is at **<https://shakedaskayo.github.io/clipboarder>**:

- [Installation](https://shakedaskayo.github.io/clipboarder/getting-started/installation/)
- [Quickstart](https://shakedaskayo.github.io/clipboarder/getting-started/quickstart/)
- [Content types](https://shakedaskayo.github.io/clipboarder/usage/content-types/)
- [Hotkeys](https://shakedaskayo.github.io/clipboarder/settings/hotkey/)
- [Privacy & exclusions](https://shakedaskayo.github.io/clipboarder/settings/privacy/)
- [Architecture deep-dive](https://shakedaskayo.github.io/clipboarder/architecture/overview/)

<br>

## Community

- [GitHub Discussions](https://github.com/shakedaskayo/clipboarder/discussions) — questions, ideas, show & tell
- [Issues](https://github.com/shakedaskayo/clipboarder/issues) — bugs and feature requests
- [Contributing](CONTRIBUTING.md) — how to contribute

If clipboarder is useful to you, a star on GitHub helps others find it.

<br>

## License

[MIT](LICENSE)
