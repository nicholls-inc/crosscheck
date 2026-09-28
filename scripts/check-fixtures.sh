#!/usr/bin/env bash
# Run the full pipeline (Rust extractor + Lean checker) on every fixture under
# test_fixtures/ that has an expected.json, and compare the reported errors and
# exit code with it. Warnings are printed but not compared.
#
# Usage: scripts/check-fixtures.sh [--release|--debug]
# Requires: cargo build (--release by default), cd prover && lake build, python3.
set -euo pipefail

cd "$(dirname "$0")/.."
profile="${1:---release}"
case "$profile" in
  --release) cli=./target/release/crosscheck-contracts ;;
  --debug) cli=./target/debug/crosscheck-contracts ;;
  *) echo "usage: $0 [--release|--debug]" >&2; exit 2 ;;
esac
checker=./prover/.lake/build/bin/contract-graph-checker
for bin in "$cli" "$checker"; do
  [ -x "$bin" ] || { echo "missing $bin (build it first)" >&2; exit 2; }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0

for expected in test_fixtures/*/expected.json; do
  dir=$(dirname "$expected")
  name=$(basename "$dir")
  set +e
  "$cli" contracts check "$dir/" --lean-checker "$checker" \
    --output-db "$tmp/$name.sqlite" >"$tmp/$name.json" 2>"$tmp/$name.err"
  code=$?
  set -e
  if python3 - "$expected" "$tmp/$name.json" "$code" "$name" <<'PY'
import json, sys
expected_path, actual_path, code, name = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
expected = json.load(open(expected_path))
try:
    actual = json.load(open(actual_path))
except json.JSONDecodeError:
    print(f"FAIL {name}: checker output is not JSON")
    sys.exit(1)
# An expected error may also name the node whose requirement failed ("target").
key = lambda e: (e["path"], e["source_guarantee"], e["target_requirement"], e.get("target"))
errors = [r for r in actual["results"] if r["severity"] == "error"]
targets_wanted = any("target" in e for e in expected["errors"])
want = sorted(key(e) for e in expected["errors"])
got = sorted(
    (" -> ".join(r["path"]), r["source_guarantee"], r["target_requirement"],
     r["target"]["name"] if any(" -> ".join(r["path"]) == e["path"] and "target" in e
                                for e in expected["errors"]) else None)
    for r in errors
)
warning_results = [r for r in actual["results"] if r["severity"] != "error"]
warnings = len(warning_results)
# Required warnings: each must match some warning's path and contain the text.
missing_warnings = [
    w for w in expected.get("warnings", [])
    if not any(" -> ".join(r["path"]) == w["path"] and w["contains"] in r["suggestion"]
               for r in warning_results)
]
got = [g for g in got]
ok = want == got and code == expected["exit_code"] and not missing_warnings
print(f"{'ok  ' if ok else 'FAIL'} {name}: {len(got)} errors, {warnings} warnings, exit {code}")
if not ok:
    for e in sorted(set(want) - set(got)):
        print("  missing:   ", " | ".join(str(x) for x in e if x is not None))
    for e in sorted(set(got) - set(want)):
        print("  unexpected:", " | ".join(str(x) for x in e if x is not None))
    for w in missing_warnings:
        print("  missing warning:", w["path"], "|", w["contains"])
    if code != expected["exit_code"]:
        print(f"  exit code {code}, expected {expected['exit_code']}")
sys.exit(0 if ok else 1)
PY
  then :; else failed=1; cat "$tmp/$name.err" >&2; fi
done

# A database the checker cannot translate faithfully fails with exit code 2
# (not 1, "inconsistencies found", and not a check that silently skips the
# constraint): a malformed contract value, or a file that is not SQLite.
expect_exit_2() {
  local label=$1 db=$2
  set +e
  "$checker" "$db" >"$tmp/$label.out" 2>"$tmp/$label.err"
  local code=$?
  set -e
  if [ "$code" -eq 2 ] && grep -q '^Error:' "$tmp/$label.err"; then
    echo "ok   $label: exit 2"
  else
    echo "FAIL $label: exit $code, expected 2 with an error on stderr"
    cat "$tmp/$label.err" >&2
    failed=1
  fi
}
corrupt() {
  python3 - "$tmp/r3_choices_forms.sqlite" "$tmp/$1.sqlite" "$2" "$3" <<'PY'
import shutil, sqlite3, sys
src, dst, col, val = sys.argv[1:5]
shutil.copy(src, dst)
db = sqlite3.connect(dst)
if col == "param_choices":
    n = db.execute("UPDATE contracts SET param_choices = ? WHERE id = "
                   "(SELECT MIN(id) FROM contracts WHERE param_choices IS NOT NULL)", (val,)).rowcount
else:
    n = db.execute(f"UPDATE contracts SET {col} = ? WHERE id = "
                   "(SELECT MIN(id) FROM contracts WHERE constraint_type = 'range')", (val,)).rowcount
db.commit()
sys.exit(0 if n == 1 else 1)
PY
}
corrupt malformed_choices param_choices '["a"'
expect_exit_2 malformed_choices "$tmp/malformed_choices.sqlite"
corrupt malformed_decimal param_max_decimal '1e-3'
expect_exit_2 malformed_decimal "$tmp/malformed_decimal.sqlite"
echo "not a database" >"$tmp/garbage.sqlite"
expect_exit_2 not_sqlite "$tmp/garbage.sqlite"

exit "$failed"
