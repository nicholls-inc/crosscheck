#!/usr/bin/env bash
# RQ-1.1's checks: the build, the axioms the theorems use, and two broken
# premises under which requirement_holds must fail to build.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here"
file=Requirements/PriceAmount.lean

lake build

axioms_out="$(lake env lean Axioms.lean)"
echo "$axioms_out"
[ "$(grep -c "depends on axioms\|does not depend on any axioms" <<<"$axioms_out")" -eq 6 ] \
  || { echo "FAIL: expected an axiom line for each of the 6 theorems"; exit 1; }
extra="$(grep -o '\[.*\]' <<<"$axioms_out" | tr -d '[]' | tr ',' '\n' | tr -d ' ' \
  | grep -v -x -e propext -e Classical.choice -e Quot.sound -e '' || true)"
[ -z "$extra" ] || { echo "FAIL: non-standard axioms: $extra"; exit 1; }
echo "ok: only propext, Classical.choice and Quot.sound"

start="$(grep -n '^theorem requirement_holds' "$file" | cut -d: -f1)"
end="$(awk -v s="$start" 'NR > s && /^\/-!/ { print NR; exit }' "$file")"

broken() {
  local name="$1" from="$2" to="$3" tmp
  tmp="$(mktemp -d)"
  cp -R lakefile.toml lean-toolchain Requirements.lean Requirements "$tmp/"
  grep -qF -- "$from" "$tmp/$file" || { echo "FAIL: $name: text not found"; exit 1; }
  sed -i.bak "s|$from|$to|" "$tmp/$file" && rm "$tmp/$file.bak"
  if out="$(cd "$tmp" && lake build 2>&1)"; then
    echo "FAIL: $name: requirement_holds still builds"; exit 1
  fi
  if ! grep -E "^error: $file:[0-9]+" <<<"$out" \
      | awk -F: -v s="$start" -v e="$end" '$3 > s && $3 < e { found = 1 } END { exit !found }'; then
    echo "FAIL: $name: the build failed, but not inside requirement_holds"; echo "$out"; exit 1
  fi
  echo "ok: $name breaks requirement_holds:"
  grep -E "^error: $file:[0-9]+" <<<"$out" \
    | awk -F: -v s="$start" -v e="$end" '$3 > s && $3 < e' | head -1
  rm -rf "$tmp"
}

broken "A2 with a currency of exponent 0" \
  "Prop := w.currencyExponent = 2" "Prop := w.currencyExponent = 0"
broken "S1 with three decimal places" \
  "Prop := a.places ≤ 2" "Prop := a.places ≤ 3"
echo "all RQ-1.1 checks passed"
