#!/usr/bin/env bash
# End-to-end CLI integration test.
# Builds the release binary, runs against a throwaway data dir, asserts every
# subcommand's behavior. Returns non-zero on any failed assertion.
#
# Usage:
#   scripts/test-cli.sh                # uses target/release/clipboarder
#   CLIPBOARDER_BIN=/path/to/bin ./scripts/test-cli.sh

set -euo pipefail

CLI=${CLIPBOARDER_BIN:-"$(cd "$(dirname "$0")/.." && pwd)/src-tauri/target/release/clipboarder"}
TMPHOME=$(mktemp -d -t clipboarder-test.XXXXXX)
trap 'rm -rf "$TMPHOME"' EXIT

# Isolate the DB: HOME redirects the data dir resolution in cli.rs.
export HOME="$TMPHOME"
mkdir -p "$HOME/Library/Application Support/com.clipboarder.app"

pass=0
fail=0

assert() {
  local name=$1 cond=$2
  if eval "$cond" >/dev/null 2>&1; then
    printf '  \033[32m✓\033[0m %s\n' "$name"
    pass=$((pass + 1))
  else
    printf '  \033[31m✗\033[0m %s\n' "$name"
    printf '    failed: %s\n' "$cond"
    fail=$((fail + 1))
  fi
}

section() { printf '\n\033[1m── %s ──\033[0m\n' "$1"; }

# ── 1. Smoke: --help, --version ────────────────────────────────────
section "help / version"
assert "--help"     '"$CLI" --help    | head -1 | grep -q clipboarder'
assert "--version"  '"$CLI" --version | grep -qE "clipboarder [0-9]+\.[0-9]+\.[0-9]+"'

# ── 2. Empty history ──────────────────────────────────────────────
section "empty state"
assert "list returns empty array"          'test "$("$CLI" list --json)" = "[]"'
assert "stats --json shows zero"           'test "$("$CLI" stats --json | jq .total)" = "0"'
assert "p exits 1 on empty"                '! "$CLI" p >/dev/null 2>&1'

# ── 3. Add (positional + stdin) + classification ──────────────────
section "add / cp + classification"
ID_TEXT=$(echo "hello from test" | "$CLI" add --json | jq .id)
assert "stdin add returns numeric id"      'test "$ID_TEXT" -gt 0'
assert "kind = text"                       'test "$("$CLI" show '"$ID_TEXT"' --json | jq -r .kind)" = "text"'

# URL classification
"$CLI" add "https://github.com/tauri-apps/tauri" >/dev/null
assert "URL classified as repo"            'test "$("$CLI" search tauri --kind repo --json | jq length)" -ge 1'

# Hex color classification
"$CLI" add "#7c8cff" >/dev/null
assert "hex classified as color"           'test "$("$CLI" list --kind color --json | jq length)" -ge 1'
assert "color meta = hex"                  'test "$("$CLI" list --kind color --json | jq -r ".[0].meta")" = "hex"'

# Email classification
"$CLI" add "demo@example.com" >/dev/null
assert "email classified as email"         'test "$("$CLI" list --kind email --json | jq length)" -ge 1'

# Dedup
ID_TEXT2=$(echo "hello from test" | "$CLI" add --json | jq .id)
assert "dedup bumps existing row"          'test "$ID_TEXT" = "$ID_TEXT2"'

# ── 4. Search ──────────────────────────────────────────────────────
section "search / FTS"
assert "search hits the right row"         'test "$("$CLI" search "tauri" --json | jq length)" -ge 1'
assert "prefix match works"                'test "$("$CLI" search "tau" --json | jq length)" -ge 1'
assert "kind filter narrows"               'test "$("$CLI" search "tauri" --kind code --json | jq length)" = "0"'

# ── 5. cb p / paste ────────────────────────────────────────────────
section "cb p"
LATEST=$("$CLI" p)
assert "cb p prints content"               'test -n "$LATEST"'
assert "cb p --kind text returns text"     '"$CLI" p --kind text | head -1 | grep -q -e hello -e example -e tauri'
assert "cb p --json returns full row"      'test -n "$("$CLI" p --json | jq -r .id 2>/dev/null)"'
assert "cb p --grep filters"               'test "$("$CLI" p --grep tauri)" = "https://github.com/tauri-apps/tauri"'
assert "cb p --all returns multiple"       'test "$("$CLI" p --all --json | wc -l)" -gt 1'

# ── 6. Agent flags ─────────────────────────────────────────────────
section "agent flags"

# --compact
COMPACT=$("$CLI" list --limit 1 --json --compact | jq ".[0] | keys | sort")
assert "--compact JSON has exactly id/kind/content/meta" \
  'test "$COMPACT" = "$(echo "[\"content\",\"id\",\"kind\",\"meta\"]" | jq .)"'

# --max-bytes truncation with ellipsis
"$CLI" add "this is a long string that we will truncate aggressively to verify the boundary handling does the right thing" >/dev/null
assert "--max-bytes truncates with ellipsis" \
  '"$CLI" list --limit 1 --json --compact --max-bytes 30 | jq -r ".[0].content" | grep -q "…"'

# --since filter
assert "--since 1h returns items"          'test "$("$CLI" list --since 1h --json | jq length)" -ge 1'
assert "--since 1s on cold DB returns 0"   'test "$("$CLI" list --since 1s --json | jq length)" -lt 100'

# --no-secrets redaction
ID_SECRET=$(echo "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" | "$CLI" add --json | jq .id)
assert "--no-secrets redacts anthropic key" \
  '"$CLI" list --limit 5 --json --no-secrets | jq -r ".[].content" | grep -q "\[redacted: anthropic api key\]"'

# --snippet
"$CLI" add "the quick brown fox jumps over the lazy dog and runs into the forest where it eats apples" >/dev/null
SNIP=$("$CLI" search "lazy" --json --snippet 40 | jq -r '.[0].content')
assert "--snippet returns context window"   '[[ "$SNIP" == *lazy* && "${#SNIP}" -lt 60 ]]'
assert "--snippet wraps with ellipsis"      '[[ "$SNIP" == *…* ]]'

# ── 7. pin / unpin / delete / clear ────────────────────────────────
section "mutations"
PIN_ID=$("$CLI" list --limit 1 --json | jq ".[0].id")
"$CLI" pin "$PIN_ID"
assert "pin sets pinned=true"              'test "$("$CLI" show '"$PIN_ID"' --json | jq .pinned)" = "true"'
"$CLI" unpin "$PIN_ID"
assert "unpin clears pinned"               'test "$("$CLI" show '"$PIN_ID"' --json | jq .pinned)" = "false"'

DEL_ID=$(echo "to be deleted" | "$CLI" add --json | jq .id)
"$CLI" delete "$DEL_ID"
assert "delete removes row"                '! "$CLI" show '"$DEL_ID"' >/dev/null 2>&1'

# ── 8. stats / doctor ──────────────────────────────────────────────
section "stats / doctor"
assert "stats reports >0 items"            'test "$("$CLI" stats --json | jq .total)" -gt 0'
assert "stats has by_kind map"             'test "$("$CLI" stats --json | jq -e ".by_kind | type")" = "\"object\""'

# Doctor on a non-macOS or no-GUI env still runs.
assert "doctor exits 0"                    '"$CLI" doctor >/dev/null'

# ── 9. CLIPBOARDER_TRACE ───────────────────────────────────────────
section "TRACE"
TRACE_ERR=$(mktemp)
CLIPBOARDER_TRACE=1 "$CLI" list --limit 1 >/dev/null 2>"$TRACE_ERR"
assert "TRACE writes to stderr when env set"  'grep -q trace "$TRACE_ERR"'

"$CLI" list --limit 1 >/dev/null 2>"$TRACE_ERR"
assert "TRACE silent when env unset"          '! grep -q trace "$TRACE_ERR"'
rm -f "$TRACE_ERR"

# ── done ───────────────────────────────────────────────────────────
echo
total=$((pass + fail))
if [ "$fail" = "0" ]; then
  printf '\033[1;32m✓ %d/%d assertions passed\033[0m\n' "$pass" "$total"
  exit 0
else
  printf '\033[1;31m✗ %d/%d assertions failed (%d passed)\033[0m\n' "$fail" "$total" "$pass"
  exit 1
fi
