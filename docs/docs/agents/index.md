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

## Best practices

### 1. Cap the result set aggressively

Don't pass `--limit 1000` "just in case". The agent's context window is precious.

- For browsing: 5–10 items
- For targeted search: 3–5 items
- For "show me everything of kind X": 20 items, then ask the user if they want more

### 2. Filter by kind whenever possible

`--kind repo` returns only GitHub-family URLs. `--kind code` only code. The cost is the same — but the relevance is much higher.

### 3. Prefer `search` over `list` when there's a query term

bm25 ranking surfaces relevant items even if they're old. `list` just gives recency.

### 4. Don't echo secrets back

If a returned item looks like an API key, JWT, password, or other credential:

- Don't put it in your reply to the user
- Don't include it in tool inputs to other tools
- Summarize: "Found a token-shaped item from <source_app>, id <id>" — let the user explicitly ask to see it.

Heuristics for secret-like content:

- Looks like base64/hex and is > 32 chars
- Contains `sk-`, `pk_`, `xoxb-`, `xoxp-`, `ghp_`, `github_pat_` prefixes
- Matches `key=`, `token=`, `password=`, `secret=` patterns
- A row's source app was a password manager (`com.1password.*`, `com.bitwarden.*`, `com.agilebits.*`, `com.lastpass.*`) — though these are usually already excluded by the user's privacy settings.

### 5. Don't auto-ingest

`clipboarder add` is for explicit "save this for me" intent. Don't ingest intermediate work, scratch outputs, or anything the user didn't ask to save.

### 6. Source-tag your ingestion

```bash
clipboarder add --source "claude" --json
```

The `--source` field appears in the row meta in the GUI, so the user can see which agent added which items.

### 7. Tail with `watch` for ambient awareness

If your agent is long-running and wants to react to new copies as they happen:

```bash
clipboarder watch --kind url
```

Prints one JSON line per new item, indefinitely. Polls the DB every 500 ms.

## Security model

- The clipboarder DB lives at `~/Library/Application Support/com.clipboarder.app/clipboarder.sqlite`
- POSIX permissions default to `600` (user-only)
- Any process running as the user can read it — including any agent the user has running
- Items from apps the user excluded (Privacy settings → excluded apps) were **never captured**, so the agent can't access them
- The CLI does not bypass the GUI's privacy filter — same DB, same exclusions

If you're building an agent for a multi-tenant environment, run the agent under a different user account; clipboarder data is per-user.
