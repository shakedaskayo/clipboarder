#!/usr/bin/env bash
# Performance benchmark for the clipboarder CLI.
# Seeds a throwaway database with N items, runs each measured command R times
# (default 25), prints min / p50 / p99 / max in milliseconds.
#
# Usage:
#   scripts/bench.sh                       # 100, 1000, 10000 seeds
#   scripts/bench.sh 5000                  # one specific seed size
#   RUNS=50 scripts/bench.sh
#   CLIPBOARDER_BIN=/path/to/bin ./scripts/bench.sh

set -euo pipefail

CLI=${CLIPBOARDER_BIN:-"$(cd "$(dirname "$0")/.." && pwd)/src-tauri/target/release/clipboarder"}
SIZES=${*:-"100 1000 10000"}
RUNS=${RUNS:-25}

TMPHOME=$(mktemp -d -t clipboarder-bench.XXXXXX)
trap 'rm -rf "$TMPHOME"' EXIT

export HOME="$TMPHOME"
mkdir -p "$HOME/Library/Application Support/com.clipboarder.app"

echo "clipboarder bench — $CLI"
"$CLI" --version

for n in $SIZES; do
  echo
  printf '\033[1m▼ %s items\033[0m\n' "$n"
  rm -rf "$HOME/Library/Application Support/com.clipboarder.app/clipboarder.sqlite"*

  echo "  seeding $n items…"
  CLI="$CLI" python3 - "$n" <<'PY'
import os, random, subprocess, sys
n = int(sys.argv[1])
random.seed(42)
words = ["alpha","beta","gamma","delta","tauri","rust","python","react","tokio","anthropic",
         "github","slack","figma","claude","cursor","sqlite","fts","query","clipboard","agent"]
hosts = ["github.com","gitlab.com","anthropic.com","example.com","news.ycombinator.com"]
def make_item(i):
    r = random.random()
    if r < 0.5: return f"{random.choice(words)} {random.choice(words)} {i} more text"
    if r < 0.7: return f"https://{random.choice(hosts)}/{random.choice(words)}/{i}"
    if r < 0.8: return f"https://github.com/{random.choice(words)}/{random.choice(words)}/pull/{i}"
    if r < 0.9: return f"#{random.randint(0,0xffffff):06x}"
    return f"const fn{i} = () => {{ return {i}; }};  // {random.choice(words)}"
cli = os.environ["CLI"]
for i in range(n):
    subprocess.run([cli, "add", make_item(i)], check=False,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
PY

  echo "  measured (mean of $RUNS runs):"
  CLI="$CLI" RUNS="$RUNS" python3 - <<'PY'
import os, subprocess, time

cli = os.environ["CLI"]
runs = int(os.environ["RUNS"])

CASES = [
    ("list --limit 10",                ["list", "--limit", "10",  "--json"]),
    ("list --limit 100",               ["list", "--limit", "100", "--json"]),
    ("search 'tauri'",                 ["search", "tauri", "--json"]),
    ("search 'tauri' --kind repo",     ["search", "tauri", "--kind", "repo", "--json"]),
    ("p --grep react --kind repo",     ["p", "--grep", "react", "--kind", "repo", "--json"]),
    ("stats --json",                   ["stats", "--json"]),
    ("list --compact --max-bytes 80",  ["list", "--limit", "50", "--json", "--compact", "--max-bytes", "80"]),
]

def fmt(ns): return f"{ns / 1e6:7.2f} ms"

for label, args in CASES:
    times_ns = []
    for _ in range(runs):
        t0 = time.perf_counter_ns()
        subprocess.run([cli, *args], check=False,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        t1 = time.perf_counter_ns()
        times_ns.append(t1 - t0)
    times_ns.sort()
    mn  = times_ns[0]
    p50 = times_ns[len(times_ns) // 2]
    p99 = times_ns[min(len(times_ns) - 1, int(len(times_ns) * 0.99))]
    mx  = times_ns[-1]
    print(f"  {label:36}  min {fmt(mn)}   p50 {fmt(p50)}   p99 {fmt(p99)}   max {fmt(mx)}")
PY
done

echo
echo "Done."
