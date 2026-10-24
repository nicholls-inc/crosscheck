#!/usr/bin/env bash
# Builds Ocpp16Composite.lean with the pinned toolchain and fails unless the
# proved theorems use only propext, Classical.choice and Quot.sound.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
toolchain="leanprover/lean4:v4.28.0"
proved=(q1 q2 q3 i1 i2 i3 candidates_eq_of_same_nonDefault)
if [ "$#" -gt 0 ]; then proved=("$@"); fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/ocpp16-axioms.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
cp "$here/Ocpp16Composite.lean" "$tmp/Check.lean"
{
  echo
  for name in "${proved[@]}"; do
    echo "#print axioms CrosscheckModel.Ocpp16Composite.$name"
  done
} >>"$tmp/Check.lean"

elan_bin="$(command -v elan || echo "$HOME/.elan/bin/elan")"
out="$(cd "$tmp" && "$elan_bin" run "$toolchain" lean Check.lean 2>&1)"
echo "$out" | grep -E "depends on axioms|does not depend on any axioms" || true

for name in "${proved[@]}"; do
  line="$(echo "$out" | grep -E "'CrosscheckModel\.Ocpp16Composite\.$name'" || true)"
  [ -n "$line" ] || { echo "FAIL: no axiom report for $name"; exit 1; }
  if echo "$line" | grep -q "sorryAx"; then
    echo "FAIL: $name depends on sorry"; exit 1
  fi
  extra="$(echo "$line" | sed -n 's/.*\[\(.*\)\].*/\1/p' | tr ',' '\n' | sed 's/ //g' \
    | grep -v -E '^(propext|Classical\.choice|Quot\.sound)$' || true)"
  [ -z "$extra" ] || { echo "FAIL: $name uses $extra"; exit 1; }
done

if echo "$out" | grep -E "error" >/dev/null; then
  echo "$out"; echo "FAIL: build errors"; exit 1
fi
echo "PASS: ${#proved[@]} theorems use only standard axioms"
