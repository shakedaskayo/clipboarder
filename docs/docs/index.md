<div class="clipboarder-hero" markdown>

![clipboarder](assets/logo.png)

# clipboarder

<p class="tagline">A clipboard for humans <em>and</em> coding agents. Searchable history, smart classification, native macOS overlay, scriptable CLI.</p>

[Install](getting-started/installation.md){ .md-button .md-button--primary }
[For Agents](agents/index.md){ .md-button }

</div>

<div class="install-cmd" markdown>

```bash
curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/install.sh | bash
```

</div>

---

![Main view](assets/screenshots/main_v2.png)

---

## The clipboard is working memory. clipboarder makes it shared.

For 40 years humans have used the clipboard as scratch space between apps — copy an error from your terminal, paste into Slack; copy a snippet from Stack Overflow, paste into your editor. You use the clipboard as glue between contexts dozens of times a day.

**Coding agents can't.** Claude Code, Codex, Cursor, and every other LLM assistant lives inside its context window. It can't see what you just copied from your terminal. It can't put a generated command on your clipboard for you to paste somewhere else. It can't recall what you copied ten minutes ago.

clipboarder fixes that. The same searchable history that powers the GUI overlay is exposed to your agent through a tiny CLI — `cb cp` to drop something on your clipboard, `cb p` to read what's there. **Your agent uses the clipboard the way you do.**

=== "Read your clipboard"

    ```bash
    # Agent runs:
    cb p
    # → prints your most-recent clipboard entry to stdout
    ```

    Use case — *"fix the error I just copied"*: the agent calls `cb p`, gets the stack trace, generates a fix.

=== "Write to your clipboard"

    ```bash
    # Agent runs:
    echo "cargo update -p tokio" | cb cp
    # → puts the fix on your clipboard, AND in clipboarder history
    ```

    Use case — *"give me a command I can paste in terminal"*: the agent calls `cb cp`, then says "⌘V into terminal". Zero manual copy-paste between you and the agent.

=== "Search your history"

    ```bash
    cb p --kind repo --grep "auth"
    # → most-recent GitHub URL matching "auth"
    ```

    Use case — *"find that PR URL I copied earlier"*: agent calls `cb p --grep` and returns the link in under 2 ms.

=== "Persistent context"

    ```bash
    cb pin 42   # agent stars an item to survive history limits
    cb watch    # stream new copies as JSON Lines as they happen
    ```

    Use case — *"remember this for next session"*: agent pins items it wants persistent recall on.

### One-liner Claude Code skill

```bash
mkdir -p ~/.claude/skills/clipboarder && \
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \
    -o ~/.claude/skills/clipboarder/SKILL.md
```

Claude Code auto-loads the skill on its next session. No plugin install, no config edit. The skill triggers on phrases like *"what did I copy"*, *"find that link I had"*, *"save this for later"*.

For LangChain / OpenAI Assistants / Cursor / any other harness → [For agents](agents/index.md) (JSON schema, tool definitions, privacy guidance, secret-detection heuristics).

---

## Why clipboarder

<div class="feature-grid" markdown>

<div class="feature-card" markdown>
### Built for agents, beloved by humans
A polished native overlay AND a `cb` CLI on your PATH. Both read the same SQLite history. Same data, two interfaces.
</div>

<div class="feature-card" markdown>
### Instant search
SQLite FTS5 with bm25 ranking. Sub-millisecond results across thousands of items, even on a cold cache.
</div>

<div class="feature-card" markdown>
### Smart classification
Every copy is auto-tagged at capture time: text, URL, email, code, color, image, file, PDF, music link, video link, repo.
</div>

<div class="feature-card" markdown>
### Rich previews
Color swatches with HEX/RGB/HSL. Inline PDF embed. Branded music/video cards. Repo cards for GitHub/GitLab with OpenGraph hero.
</div>

<div class="feature-card" markdown>
### Source app icons
Each row shows the real icon of the app you copied from — Safari, VS Code, Figma — extracted live via NSWorkspace.
</div>

<div class="feature-card" markdown>
### Private by default
All data stays local. No telemetry. No cloud. No account. Privacy exclusions keep password managers out of history.
</div>

</div>

---

## A preview for every kind

=== "Repos"

    ![Repo card](assets/screenshots/repo_v2.png)

=== "Music"

    ![Spotify card](assets/screenshots/music_v2.png)

=== "Colors"

    ![Color swatch](assets/screenshots/color_v2.png)

=== "Code"

    ![Code preview](assets/screenshots/code_v2.png)

=== "Settings"

    ![Settings panel](assets/screenshots/settings_v2.png)

---

## How It Works

```
       ┌────────────────────────────────────────────────────────┐
       │                  Your macOS clipboard                  │
       │                    (NSPasteboard)                      │
       └─────────────┬──────────────────────────┬───────────────┘
                     │                          │
              read   │                          │  write
                     ▼                          ▼
┌──────────────────────────┐         ┌──────────────────────────┐
│   GUI overlay (⌘⇧V)      │         │   cb CLI                 │
│   for humans             │         │   for agents + shells    │
│                          │         │                          │
│   Search, filter, paste  │         │   cb cp / cb p / cb pop  │
└─────────────┬────────────┘         └─────────────┬────────────┘
              │                                    │
              └─────────────────┬──────────────────┘
                                ▼
              ┌─────────────────────────────────┐
              │  SQLite + FTS5 (local, on disk) │
              │  ~/Library/Application Support/ │
              │   com.clipboarder.app/          │
              └─────────────────────────────────┘
```

A Rust thread watches `NSPasteboard` change-count. Every copy is classified, deduplicated via SHA-256, persisted with FTS5 triggers keeping the search index in sync. The overlay is a frameless transparent Tauri window that floats above other apps. Selecting an item writes it back to the pasteboard, hides the overlay, deactivates clipboarder (so macOS surfaces your previous app), and synthesizes `⌘V` via `CGEventPost`. The CLI talks to the same SQLite store directly — works whether or not the GUI is running.

---

## Get Started

- **[Installation](getting-started/installation.md)** — one-liner installer or manual `.dmg`
- **[Quickstart](getting-started/quickstart.md)** — your first 60 seconds with clipboarder
- **[Pipe one-liners (`cb cp` / `cb p`)](cli-reference/pipes.md)** — the flagship CLI ergonomics
- **[For agents](agents/index.md)** — Claude Skill, LangChain, OpenAI, schema
- **[Keyboard shortcuts](usage/shortcuts.md)** — the moves that make it fast
- **[Settings](settings/index.md)** — every knob, explained

---

## Open Source

clipboarder is [MIT-licensed](https://github.com/shakedaskayo/clipboarder/blob/main/LICENSE) and lives on [GitHub](https://github.com/shakedaskayo/clipboarder).

- File a bug or request a feature in [Issues](https://github.com/shakedaskayo/clipboarder/issues)
- See [CONTRIBUTING](https://github.com/shakedaskayo/clipboarder/blob/main/CONTRIBUTING.md) for the development setup
