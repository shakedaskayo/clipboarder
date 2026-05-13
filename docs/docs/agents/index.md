# Integrating clipboarder with AI agents

clipboarder's CLI is designed to be agent-friendly. Every command supports `--json`, the schema is stable, and read operations are safe and sub-millisecond. This page walks through the patterns we recommend for any agent harness — Claude Code, LangChain, OpenAI Assistants, custom — to wire up clipboard history.

## Why it's a good fit

- **Local** — no network calls when querying. Nothing leaves the user's machine.
- **Stable schema** — JSON output is versioned with the binary; field names don't change between minor versions.
- **Fast** — FTS5 + bm25 returns sub-millisecond results across thousands of items.
- **Per-kind filtering** — `--kind repo` / `--kind code` / `--kind color` etc. lets the agent narrow context cheaply.
- **Pinning is a write-light operation** — agents can mark items the user wants to keep around without restructuring storage.

## Drop-in Claude Skill (one-liner)

```bash
mkdir -p ~/.claude/skills/clipboarder && \
  curl -fsSL https://raw.githubusercontent.com/shakedaskayo/clipboarder/main/agents/.claude/skills/clipboarder/SKILL.md \
    -o ~/.claude/skills/clipboarder/SKILL.md
```

Claude Code auto-loads the skill on every session. The skill triggers on phrases like *"what did I copy"*, *"find that link I had"*, *"save this for later"* — no further config, no plugin install.

## CLI cheat-sheet

Prefer **`cb`** (the short alias) over `clipboarder` in tool calls — it's the same binary.

```bash
# Pipe one-liners — recommended path for agents
echo "save this" | cb cp --source claude --json    # stdin → history + clipboard
cb p                                               # most recent item to stdout
cb p --kind url                                    # most recent URL
cb p --kind repo --grep "tauri" --json             # composed search → JSON row
cb p --all --kind code --limit 5                   # all matching, body per line
cb pop                                             # print + delete most recent

# Structured search
cb list --limit 10 --json
cb search "anthropic" --json
cb search "react hooks" --kind code --json
cb show 42 --json                                  # full row by id

# Mutations
cb pin 42
cb delete 42
cb clear -y                                        # nuke non-pinned

# Watch new items (JSONL, polls every 500ms)
cb watch --kind url | while read -r line; do
  echo "$line" | jq -r .content | xargs notify-send "New URL captured"
done

# Stats
cb stats --json
```

## JSON schema

Every item from `list`/`search`/`show`/`watch` has the same shape:

```json
{
  "id": 17,
  "kind": "repo",
  "content": "https://github.com/owner/repo/pull/123",
  "preview": "https://github.com/owner/repo/pull/123",
  "source_app": "Safari",
  "source_app_id": "com.apple.Safari",
  "meta": "github",
  "image_path": null,
  "pinned": false,
  "size": 43,
  "created_at": 1778619961050,
  "last_used_at": 1778619961050
}
```

`kind` is one of:

- `text` · `url` · `repo` · `email` · `code` · `color`
- `image` · `file` · `pdf`
- `music` · `video`

`meta` is kind-specific:

| Kind | `meta` contains |
|------|-----------------|
| `url` | the host (`anthropic.com`) |
| `repo` | platform tag (`github`, `gitlab`, `bitbucket`, `codeberg`, `gist`) |
| `code` | language guess (`rust`, `python`, `typescript`, `sql`, `shell`, …) |
| `color` | format (`hex`, `rgb`, `hsl`) |
| `music` | platform (`spotify`, `apple-music`, `youtube-music`, `soundcloud`, `bandcamp`) |
| `video` | platform (`youtube`, `vimeo`, `twitch`) |
| `image` | `{w}x{h}` |
| `pdf` / `text` / `file` / `email` | `null` |

## Tool definitions

### LangChain

```python
from langchain.tools import Tool
import subprocess, json

def clipboarder_search(query: str, limit: int = 10) -> str:
    result = subprocess.run(
        ["clipboarder", "search", query, "--limit", str(limit), "--json"],
        capture_output=True, text=True, timeout=5,
    )
    return result.stdout

def clipboarder_list(kind: str = "all", limit: int = 10) -> str:
    args = ["clipboarder", "list", "--limit", str(limit), "--json"]
    if kind != "all":
        args += ["--kind", kind]
    result = subprocess.run(args, capture_output=True, text=True, timeout=5)
    return result.stdout

tools = [
    Tool(
        name="clipboarder_search",
        func=clipboarder_search,
        description="Full-text search the user's local clipboard history. "
                    "Returns up to 10 items as JSON. Use when the user references "
                    "something they previously copied.",
    ),
    Tool(
        name="clipboarder_list",
        func=clipboarder_list,
        description="List recent clipboard items, optionally filtered by kind "
                    "(url|repo|code|color|email|music|video|image|file|pdf|text). "
                    "Returns up to 10 items as JSON.",
    ),
]
```

### OpenAI Assistants / Function calling

```json
{
  "name": "clipboarder_search",
  "description": "Search the user's local macOS clipboard history. Returns JSON of matching items.",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {"type": "string"},
      "kind": {
        "type": "string",
        "enum": ["all", "text", "url", "repo", "email", "code", "color", "image", "file", "pdf", "music", "video"]
      },
      "limit": {"type": "integer", "default": 10}
    },
    "required": ["query"]
  }
}
```

Handler in your runner:

```python
import subprocess

def call_clipboarder_search(args):
    cmd = ["clipboarder", "search", args["query"], "--limit", str(args.get("limit", 10)), "--json"]
    if args.get("kind") and args["kind"] != "all":
        cmd += ["--kind", args["kind"]]
    return subprocess.check_output(cmd, text=True, timeout=5)
```

### Plain shell agent

```bash
# Inside any shell-tool-using agent:
clipboarder list --kind url --limit 5 --json
clipboarder search "github" --json
clipboarder show 17 --json
```

## Agent-friendly flags

Combine these freely with `cb list`, `cb search`, and `cb p`. They're the difference between dumping everything into the agent's context and giving it just the relevant slice.

| Flag | Effect |
|------|--------|
| `--compact` | Minimal JSON `{id, kind, content, meta}`. ~40% fewer tokens. |
| `--max-bytes N` | Truncate content to N bytes at a UTF-8 char boundary, append `…`. |
| `--since 30s\|5m\|1h\|2d\|1w` | Only items used within the window. |
| `--no-secrets` | Items that look like API keys / JWTs / private keys → `[redacted: <kind>]`. Built-in detection covers OpenAI, Anthropic, GitHub PATs, AWS, Slack, GitLab, Google, Stripe, Twilio, JWTs, PEM blocks, and `password=`/`token=`/`api_key=` patterns. |
| `--snippet N` | (search / `p --grep` only) Replace content with an N-byte window around the match. |

Typical agent call:

```bash
cb p --grep "react" --kind repo --since 1d --no-secrets --compact --max-bytes 200 --json
```

## Best practices

### 1. Always set `--no-secrets`

Unless the user explicitly asks for the raw content of a credential, pass `--no-secrets`. Tokens get replaced with `[redacted: anthropic api key]` style placeholders so you know one was there but can't accidentally leak it.

### 2. Cap the result set aggressively

Don't pass `--limit 1000` "just in case". Agent context windows are precious.

- For browsing: 5–10 items
- For targeted search: 3–5 items
- For "show me everything of kind X": 20 items, then ask

### 3. Prefer `cb p --grep` over `cb search` + `cb show`

One round-trip instead of two. `--snippet` gives you just the matching context.

### 4. Filter by kind whenever possible

`--kind repo` only GitHub-family. `--kind code` only code. Same query cost, much higher relevance.

### 5. Truncate aggressively with `--max-bytes`

A 5 KB code item costs ~1500 tokens. With `--max-bytes 400` it costs ~120 and you still see what kind of code it is.

### 6. Don't auto-ingest

`cb cp` / `cb add` is for explicit "save this for me" intent. Don't ingest intermediate work the user didn't ask to save.

### 7. Source-tag your ingestion

```bash
echo "..." | cb cp --source "claude" --json
```

The `--source` shows up in the GUI row meta — the user can see which agent added what.

### 8. Tail with `watch` for ambient awareness

```bash
cb watch --kind url
```

JSONL on stdout, indefinitely. Local mode polls the DB every 500 ms; in remote mode (`CLIPBOARDER_SERVER` set) it subscribes to the server's SSE stream and delivers events in ~5 ms.

## Performance

Measured on M2 Pro, 10,000-item DB ([scripts/bench.sh](https://github.com/shakedaskayo/clipboarder/blob/main/scripts/bench.sh)):

| Command | p50 | p99 |
|---------|-----|-----|
| `cb list --limit 10` | 6.7 ms | 7.5 ms |
| `cb search "<query>"` | 5.2 ms | 5.7 ms |
| `cb p --grep "<query>" --kind repo` | 4.9 ms | 5.2 ms |
| `cb stats --json` | 14.5 ms | 15.3 ms |

The 4–5 ms baseline is Rust binary cold-start; actual SQL query time is sub-millisecond.

### Tracing slow queries

Set `CLIPBOARDER_TRACE=1` to log every SQL query + elapsed time to stderr:

```bash
$ CLIPBOARDER_TRACE=1 cb search "react" --json 2>&1 | head -3
[trace] search 0.33ms — SELECT i.id, i.kind, i.content, i.preview, i.meta, i.source_app, i.source_app_id, i.image_path, i.pinned, i.size, i.created_at, i.last_used_at FROM items_fts f JOIN items i ON i.id = f.rowid WHERE items_fts MATCH ?1 ORDER BY i.pinned DESC, bm25(items_fts) ASC, i.last_used_at DESC LIMIT ?3
```

## Security model

- The clipboarder DB lives at `~/Library/Application Support/com.clipboarder.app/clipboarder.sqlite`
- POSIX permissions default to `600` (user-only)
- Any process running as the user can read it — including any agent the user has running
- Items from apps the user excluded (Privacy settings → excluded apps) were **never captured**, so the agent can't access them
- The CLI does not bypass the GUI's privacy filter — same DB, same exclusions

If you're building an agent for a multi-tenant environment, run the agent under a different user account; clipboarder data is per-user.
