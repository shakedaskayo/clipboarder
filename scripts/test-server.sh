#!/usr/bin/env bash
# Integration test for `clipboarder serve` + `admin`.
# Spins up the server with two namespaces (alice + bob), exercises every
# endpoint via curl, asserts namespace isolation.
#
# Usage:
#   scripts/test-server.sh
#   CLIPBOARDER_BIN=/path/to/bin ./scripts/test-server.sh

set -euo pipefail

CLI=${CLIPBOARDER_BIN:-"$(cd "$(dirname "$0")/.." && pwd)/src-tauri/target/release/clipboarder"}
PORT=${PORT:-7489}
BASE="http://127.0.0.1:$PORT"

TMPHOME=$(mktemp -d -t clipboarder-srv.XXXXXX)
export HOME="$TMPHOME"
mkdir -p "$HOME/Library/Application Support/com.clipboarder.app"

SRV_PID=""
cleanup() {
  [ -n "$SRV_PID" ] && kill "$SRV_PID" 2>/dev/null || true
  rm -rf "$TMPHOME"
}
trap cleanup EXIT

pass=0; fail=0
assert() {
  local name=$1 cond=$2
  if eval "$cond" >/dev/null 2>&1; then
    printf '  \033[32m✓\033[0m %s\n' "$name"; pass=$((pass+1))
  else
    printf '  \033[31m✗\033[0m %s — \033[2m%s\033[0m\n' "$name" "$cond"; fail=$((fail+1))
  fi
}
section() { printf '\n\033[1m── %s ──\033[0m\n' "$1"; }

section "admin"
TOKEN_ALICE=$("$CLI" admin token create --namespace alice --label "alice mac"  2>/dev/null)
TOKEN_BOB=$(  "$CLI" admin token create --namespace bob   --label "bob ipad"   2>/dev/null)
assert "alice token starts with tk_"  '[[ "$TOKEN_ALICE" == tk_* ]]'
assert "bob token starts with tk_"    '[[ "$TOKEN_BOB"   == tk_* ]]'
assert "tokens differ"                '[ "$TOKEN_ALICE" != "$TOKEN_BOB" ]'
assert "admin token list shows both"  '[ "$("$CLI" admin token list 2>&1 | grep -c "tk_")" -eq 2 ]'

section "boot"
"$CLI" serve --bind "127.0.0.1:$PORT" > /tmp/clipd-srv.log 2>&1 &
SRV_PID=$!
sleep 1.0
assert "server PID alive"             'kill -0 "$SRV_PID"'
assert "server listening on $PORT"    'curl -fsS -m 2 "$BASE/v1/health" | grep -q ok'

section "auth"
assert "GET /v1/items no header → 401" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/items")" = "401" ]'
assert "bad bearer → 401" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer tk_fake" "$BASE/v1/items")" = "401" ]'
assert "valid bearer → 200" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items")" = "200" ]'

section "whoami"
ALICE_NS=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/whoami" | python3 -c 'import json,sys; print(json.load(sys.stdin)["namespace"])')
BOB_NS=$(curl   -s -H "Authorization: Bearer $TOKEN_BOB"   "$BASE/v1/whoami" | python3 -c 'import json,sys; print(json.load(sys.stdin)["namespace"])')
assert "alice whoami → alice"         '[ "$ALICE_NS" = "alice" ]'
assert "bob   whoami → bob"           '[ "$BOB_NS"   = "bob"   ]'

section "items create"
post() {
  curl -s -X POST -H "Authorization: Bearer $1" -H 'Content-Type: application/json' \
    -d "$2" "$BASE/v1/items"
}
RESP=$(post "$TOKEN_ALICE" '{"content":"alice copied https://github.com/anthropic/sdk"}')
ID_A1=$(echo "$RESP" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
RESP=$(post "$TOKEN_ALICE" '{"content":"#7c8cff"}')
ID_A2=$(echo "$RESP" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
RESP=$(post "$TOKEN_BOB"   '{"content":"BOB SECRET"}')
ID_B1=$(echo "$RESP" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
assert "alice items got ids"          '[ "$ID_A1" -gt 0 ] && [ "$ID_A2" -gt 0 ]'
assert "bob item got id"              '[ "$ID_B1" -gt 0 ]'

section "namespace isolation"
N_ALICE=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items?limit=50" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
N_BOB=$(curl   -s -H "Authorization: Bearer $TOKEN_BOB"   "$BASE/v1/items?limit=50" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "alice sees 2 items"           '[ "$N_ALICE" -eq 2 ]'
assert "bob   sees 1 item"            '[ "$N_BOB"   -eq 1 ]'
assert "alice cannot read bob's id"   \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$ID_B1")" = "404" ]'

section "FTS"
N_HIT_ALICE=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items?q=anthropic&limit=5" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
N_HIT_BOB=$(curl   -s -H "Authorization: Bearer $TOKEN_BOB"   "$BASE/v1/items?q=anthropic&limit=5" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "alice search 'anthropic' → 1" '[ "$N_HIT_ALICE" -eq 1 ]'
assert "bob   search 'anthropic' → 0" '[ "$N_HIT_BOB"   -eq 0 ]'

section "pin"
curl -s -X POST -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$ID_A1/pin" >/dev/null
PINNED=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items?kind=pinned" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "pinned filter returns 1"      '[ "$PINNED" -eq 1 ]'
curl -s -X DELETE -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$ID_A1/pin" >/dev/null
PINNED=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items?kind=pinned" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "unpin → 0"                    '[ "$PINNED" -eq 0 ]'

section "delete"
curl -s -X DELETE -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$ID_A2" >/dev/null
assert "alice deleted A2 → 404"       '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$ID_A2")" = "404" ]'
assert "alice still has 1 item"       '[ "$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" -eq 1 ]'

section "stats"
STATS=$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/stats")
assert "stats.namespace = alice"      '[ "$(echo "$STATS" | python3 -c "import json,sys; print(json.load(sys.stdin)[\"namespace\"])")" = "alice" ]'
assert "stats.total = 1"              '[ "$(echo "$STATS" | python3 -c "import json,sys; print(json.load(sys.stdin)[\"total\"])")" = "1" ]'

section "clear"
curl -s -X POST -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/clear" >/dev/null
assert "alice clear → 0 items"        '[ "$(curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" -eq 0 ]'
assert "bob untouched (still 1)"      '[ "$(curl -s -H "Authorization: Bearer $TOKEN_BOB" "$BASE/v1/items" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" -eq 1 ]'

section "revoke (file-level — live server has the old config cached)"
"$CLI" admin token revoke "$TOKEN_BOB" 2>/dev/null
assert "config no longer has bob token" '! grep -q "$TOKEN_BOB" "$HOME/Library/Application Support/com.clipboarder.app/server.toml"'
# A revoke takes effect after `clipboarder serve` restarts. The running
# server doesn't watch the config file (yet), so we deliberately skip a live
# 401 assertion here.

echo
total=$((pass+fail))
if [ "$fail" = "0" ]; then
  printf '\033[1;32m✓ %d/%d assertions passed\033[0m\n' "$pass" "$total"
  exit 0
else
  printf '\033[1;31m✗ %d/%d failed (%d passed)\033[0m\n' "$fail" "$total" "$pass"
  exit 1
fi
