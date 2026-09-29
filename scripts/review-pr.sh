#!/usr/bin/env bash
# Check a pull request's base and head trees and print the findings the head
# introduces. Extra arguments go to `contracts check` for both runs (for
# example `--exclude '**/__tests__/**'`).
#
# Usage: scripts/review-pr.sh BASE_DIR HEAD_DIR [check options...]
# Requires: cargo build --release, cd prover && lake build, python3.
set -euo pipefail

[ "$#" -ge 2 ] || { echo "usage: $0 BASE_DIR HEAD_DIR [check options...]" >&2; exit 2; }
root="$(cd "$(dirname "$0")/.." && pwd)"
base_dir=$1 head_dir=$2
shift 2
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

for side in base head; do
  dir=base_dir
  [ "$side" = head ] && dir=head_dir
  set +e
  "$root/target/release/crosscheck-contracts" contracts check "${!dir}" \
    --lean-checker "$root/prover/.lake/build/bin/contract-graph-checker" \
    --format json --output-db "$out/$side.sqlite" "$@" >"$out/$side.json" 2>"$out/$side.err"
  code=$?
  set -e
  if [ "$code" -eq 2 ]; then
    echo "$side: exit 2 (extraction failed or run incomplete)" >&2
    cat "$out/$side.err" >&2
    exit 2
  fi
  python3 - "$side" "$code" "$out/$side.json" <<'PY'
import json, sys
side, code, path = sys.argv[1], sys.argv[2], sys.argv[3]
results = json.load(open(path))["results"]
count = lambda s: sum(r["severity"] == s for r in results)
print(f"{side}: exit {code}, {count('error')} errors, {count('warning')} warnings")
PY
done

echo "new in head:"
"$root/scripts/diff-findings.sh" "$out/base.json" "$out/head.json"
