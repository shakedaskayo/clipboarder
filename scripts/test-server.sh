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
assert "config stores argon2 hash, not plaintext" \
  '! grep -F "$TOKEN_ALICE" "$HOME/Library/Application Support/com.clipboarder.app/server.toml" && grep -q "argon2" "$HOME/Library/Application Support/com.clipboarder.app/server.toml"'

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

section "transparent CLI client (env vars → remote backend)"
RUN_REMOTE() {
  env -u CLIPBOARDER_NAMESPACE \
    CLIPBOARDER_SERVER="$BASE" CLIPBOARDER_TOKEN="$TOKEN_ALICE" "$CLI" "$@"
}
RUN_REMOTE_BOB() {
  env -u CLIPBOARDER_NAMESPACE \
    CLIPBOARDER_SERVER="$BASE" CLIPBOARDER_TOKEN="$TOKEN_BOB" "$CLI" "$@"
}
echo "alice CLI test note about anthropic" | RUN_REMOTE add --json --source claude > /dev/null
echo "alice another note about react"      | RUN_REMOTE add --json --source claude > /dev/null
echo "bob   private note"                  | RUN_REMOTE_BOB add --json --source bob   > /dev/null

assert "remote doctor mentions remote backend" \
  'out=$(RUN_REMOTE doctor 2>&1); echo "$out" | grep -q "remote "'
assert "remote cb list ≥ 2 alice items" \
  '[ "$(RUN_REMOTE list --limit 10 --json | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" -ge 2 ]'
assert "remote cb search anthropic → 1 (alice)" \
  '[ "$(RUN_REMOTE search anthropic --json | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" = "1" ]'
assert "remote cb search anthropic → 0 (bob)" \
  '[ "$(RUN_REMOTE_BOB search anthropic --json | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" = "0" ]'
assert "remote cb p --grep returns content" \
  '[ "$(RUN_REMOTE p --grep anthropic)" = "alice CLI test note about anthropic" ]'
assert "remote cb stats describes backend" \
  'RUN_REMOTE stats 2>&1 | grep -q "remote "'
assert "remote cb pin + pinned filter" \
  'ID=$(RUN_REMOTE list --limit 1 --json | python3 -c "import json,sys; print(json.load(sys.stdin)[0][\"id\"])") && RUN_REMOTE pin "$ID" && [ "$(RUN_REMOTE list --kind pinned --json | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")" -ge 1 ]'

section "transparent CLI watch (SSE consumption)"
WATCH_OUT=$(mktemp -t clipboarder-watch.XXXXXX)
RUN_REMOTE watch > "$WATCH_OUT" 2>/dev/null &
WATCH_PID=$!
# Give SSE a beat to connect + send the initial `ready` event.
sleep 0.6
echo "sse smoke test — $(date +%s%N)" | RUN_REMOTE add --json --source claude > /dev/null
echo "sse second event"               | RUN_REMOTE add --json --source claude > /dev/null
# SSE delivery is sub-100 ms in practice; allow a generous wait for CI.
sleep 0.8
kill "$WATCH_PID" 2>/dev/null || true
wait "$WATCH_PID" 2>/dev/null || true
assert "watch received SSE smoke item" \
  'grep -q "sse smoke test" "$WATCH_OUT"'
assert "watch received second SSE item" \
  'grep -q "sse second event" "$WATCH_OUT"'
assert "watch emitted both events on separate lines" \
  '[ "$(grep -c "sse " "$WATCH_OUT")" -ge 2 ]'
rm -f "$WATCH_OUT"

section "admin web UI"
# /admin returns the static HTML for anyone (no auth on the page itself —
# the JS asks for the bearer on the client side).
assert "GET /admin → 200 + HTML" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" "$BASE/admin")" = "200" ]'
assert "/admin payload looks like the console" \
  'curl -s "$BASE/admin" | grep -q "clipboarder admin"'
# A regular (non-admin) token is rejected with 403 on every admin endpoint.
assert "regular token → 403 on /v1/admin/tokens" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/admin/tokens")" = "403" ]'
# Mint an admin token via the CLI; live reloader picks it up within ~2 s.
TOKEN_ADMIN=$("$CLI" admin token create --namespace adminns --label "ci admin" --admin 2>/dev/null)
# Wait for reload (server polls every 2s).
adm_ok=""
for _ in 1 2 3 4 5; do
  sleep 1
  code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ADMIN" "$BASE/v1/admin/tokens")
  if [ "$code" = "200" ]; then adm_ok="yes"; break; fi
done
assert "admin token created + recognized after reload" '[ "$adm_ok" = "yes" ]'
N_TOKENS=$(curl -s -H "Authorization: Bearer $TOKEN_ADMIN" "$BASE/v1/admin/tokens" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "admin sees all tokens"        '[ "$N_TOKENS" -ge 3 ]'
N_NS=$(curl -s -H "Authorization: Bearer $TOKEN_ADMIN" "$BASE/v1/admin/namespaces" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
assert "admin sees ≥ alice + bob + adminns namespaces" '[ "$N_NS" -ge 2 ]'
# Create a token via the admin REST API; check the plaintext is returned.
NEW_BODY='{"namespace":"carol","label":"created via admin api","admin":false}'
NEW_REPLY=$(curl -s -X POST -H "Authorization: Bearer $TOKEN_ADMIN" -H "Content-Type: application/json" -d "$NEW_BODY" "$BASE/v1/admin/tokens")
NEW_BEARER=$(echo "$NEW_REPLY" | python3 -c 'import json,sys; print(json.load(sys.stdin)["bearer"])')
NEW_FP=$(echo "$NEW_REPLY" | python3 -c 'import json,sys; print(json.load(sys.stdin)["fingerprint"])')
assert "POST /v1/admin/tokens returned a tk_ bearer" '[[ "$NEW_BEARER" == tk_* ]]'
# The new token must work after a brief reload window.
new_ok=""
for _ in 1 2 3 4 5; do
  sleep 1
  code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $NEW_BEARER" "$BASE/v1/whoami")
  if [ "$code" = "200" ]; then new_ok="yes"; break; fi
done
assert "freshly-minted token authenticates (200 on whoami)" '[ "$new_ok" = "yes" ]'
# Revoke it via DELETE /v1/admin/tokens/:fp; should stop working.
assert "DELETE /v1/admin/tokens/:fp → 204" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -X DELETE -H "Authorization: Bearer $TOKEN_ADMIN" "$BASE/v1/admin/tokens/$NEW_FP")" = "204" ]'
# auth_cache is cleared on revoke so this is immediate.
assert "revoked token → 401" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $NEW_BEARER" "$BASE/v1/whoami")" = "401" ]'
DELETE_404=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE -H "Authorization: Bearer $TOKEN_ADMIN" "$BASE/v1/admin/tokens/tk_nope000000")
assert "DELETE unknown fingerprint → 404" '[ "$DELETE_404" = "404" ]'

section "image endpoint"
# A minimal 67-byte 1x1 PNG. Sidestepping POST /v1/items because that route
# only ingests text — image-kind rows are normally written by the GUI watcher
# directly via SQLite. Insert one by hand so we can exercise GET …/image.
IMG_PATH="$HOME/Library/Application Support/com.clipboarder.app/test.png"
python3 -c 'import sys, base64; sys.stdout.buffer.write(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="))' > "$IMG_PATH"
DB="$HOME/Library/Application Support/com.clipboarder.app/clipboarder.sqlite"
SHA=$(shasum -a 256 "$IMG_PATH" | cut -d' ' -f1)
SIZE=$(wc -c < "$IMG_PATH" | tr -d ' ')
NOW=$(date +%s)000
# `trusted_schema=ON` lets us touch items_fts via its INSERT trigger from a
# sqlite3 CLI session (SQLite's default for unknown connections is OFF).
sqlite3 "$DB" "PRAGMA trusted_schema=ON; INSERT INTO items (kind, content, preview, image_path, source_app, meta, content_hash, size, pinned, created_at, last_used_at, namespace) VALUES ('image', '[image]', '[image]', '$IMG_PATH', 'test', NULL, '$SHA', $SIZE, 0, $NOW, $NOW, 'alice');"
IMG_ID=$(sqlite3 "$DB" "SELECT id FROM items WHERE image_path='$IMG_PATH' LIMIT 1;")
assert "image item inserted via sqlite"     '[ -n "$IMG_ID" ]'

TMP_OUT=$(mktemp -t clipboarder-img.XXXXXX)
HTTP_CODE=$(curl -s -o "$TMP_OUT" -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$IMG_ID/image")
assert "alice GET /v1/items/:id/image → 200" '[ "$HTTP_CODE" = "200" ]'
GOT_SHA=$(shasum -a 256 "$TMP_OUT" | cut -d' ' -f1)
assert "image bytes round-trip intact"       '[ "$GOT_SHA" = "$SHA" ]'
assert "Content-Type was image/png"          \
  '[ "$(curl -s -o /dev/null -w "%{content_type}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items/$IMG_ID/image")" = "image/png" ]'
assert "bob → 404 on alice's image (namespace isolated)" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_BOB" "$BASE/v1/items/$IMG_ID/image")" = "404" ]'
assert "no auth → 401" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/items/$IMG_ID/image")" = "401" ]'
rm -f "$TMP_OUT"

section "revoke + live config reload"
# Bob's fingerprint is the first 11 chars of his bearer.
BOB_FP=${TOKEN_BOB:0:11}
"$CLI" admin token revoke "$BOB_FP" 2>/dev/null
assert "config no longer references bob's fingerprint" \
  '! grep -q "fingerprint = \"$BOB_FP\"" "$HOME/Library/Application Support/com.clipboarder.app/server.toml"'
# Reloader polls every 2 s; allow up to ~5 s for the running server to pick it up.
hit_401=""
for _ in 1 2 3 4 5; do
  sleep 1
  code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_BOB" "$BASE/v1/items")
  if [ "$code" = "401" ]; then
    hit_401="yes"
    break
  fi
done
assert "running server rejects revoked token (live reload)" '[ "$hit_401" = "yes" ]'
assert "alice's token still works after bob revoked" \
  '[ "$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items")" = "200" ]'

section "last_used_at"
# Provoke a fresh auth on alice's token; the touch should bubble to disk.
curl -s -H "Authorization: Bearer $TOKEN_ALICE" "$BASE/v1/items?limit=1" >/dev/null
sleep 0.2
assert "alice's entry has a last_used_at timestamp" \
  'grep -A6 "namespace = \"alice\"" "$HOME/Library/Application Support/com.clipboarder.app/server.toml" | grep -q "last_used_at"'

echo
total=$((pass+fail))
if [ "$fail" = "0" ]; then
  printf '\033[1;32m✓ %d/%d assertions passed\033[0m\n' "$pass" "$total"
  exit 0
else
  printf '\033[1;31m✗ %d/%d failed (%d passed)\033[0m\n' "$fail" "$total" "$pass"
  exit 1
fi
