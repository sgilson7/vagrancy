#!/usr/bin/env bash
# How many tests there are, counted so the answer is the same twice.
#
# Ported from gear-master-2d. **`cargo test`'s own output cannot be summed.**
# Test binaries and their threads write to one stream, so a summary line gets
# cut in half, and totting up the fourth field gave 978, 1,025 and 1,053 off
# one run there; a suite figure was published three times as 1,137 and could
# never be got back. So the binaries are asked one at a time, with `--list`,
# which prints one line per test and no summary to garble. Nothing is
# executed: this counts, and `cargo test` decides whether they pass.
#
# The whole workspace by default; pass a package name to count one.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ $# -gt 0 ]; then SEL=(-p "$1"); LABEL="$1"; else SEL=(--workspace); LABEL=workspace; fi

BINS="$(cargo test "${SEL[@]}" --no-run --message-format=json 2>/dev/null |
  python3 -c '
import json, sys
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("profile", {}).get("test") and m.get("executable"):
        print(m["executable"])
')"

total=0
count=0
while IFS= read -r b; do
  [ -z "$b" ] && continue
  n="$("$b" --list 2>/dev/null | grep -cE ': test$' || true)"
  total=$(( total + n ))
  count=$(( count + 1 ))
done <<< "$BINS"
printf '%s: %d tests in %d binaries\n' "$LABEL" "$total" "$count"
