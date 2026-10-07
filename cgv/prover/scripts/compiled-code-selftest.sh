#!/usr/bin/env bash
# SM-8 in intent/2026-10-07-compiled-code-attributes-spec.md. Puts
# @[implemented_by] or @[extern] on one constant of ContractGraph/Main.lean,
# rebuilds, runs the statement generator, and checks that it fails exactly when
# a protected theorem or definition reaches the constant. Main.lean is restored
# on exit.
#
# Usage, from cgv/prover after `lake build ContractGraph ContractGraph.Main`:
#   scripts/compiled-code-selftest.sh
set -euo pipefail

cd "$(dirname "$0")/.."
main=ContractGraph/Main.lean
backup="$(mktemp "${TMPDIR:-/tmp}/cgv-selftest.XXXXXX")"
cp "$main" "$backup"
restore() {
  cp "$backup" "$main"
  rm -f "$backup"
  lake build ContractGraph.Main >/dev/null 2>&1 || true
}
trap restore EXIT

failures=0

# run_case NAME EXPECT IMPL_DEFS EDIT
#   EXPECT is "reject" (the generator must fail and name ContractGraph.NAME)
#   or "accept". IMPL_DEFS is inserted after the namespace line, and EDIT is a
#   perl substitution applied to each line of Main.lean.
run_case() {
  local name=$1 expect=$2 impl=$3 edit=$4
  cp "$backup" "$main"
  IMPL="$impl" perl -0pi -e 's/^namespace ContractGraph\n/namespace ContractGraph\n\n$ENV{IMPL}\n/m' "$main"
  local inserted
  inserted="$(mktemp "${TMPDIR:-/tmp}/cgv-selftest.XXXXXX")"
  cp "$main" "$inserted"
  perl -pi -e "$edit" "$main"
  if cmp -s "$main" "$inserted"; then
    rm -f "$inserted"
    echo "FAIL $name: the edit matched nothing; the definition moved or changed"
    failures=$((failures + 1))
    return
  fi
  rm -f "$inserted"
  if ! lake build ContractGraph.Main >/dev/null 2>&1; then
    echo "FAIL $name: the edited Main.lean does not build"
    failures=$((failures + 1))
    return
  fi
  local out status=0
  out=$(lake env lean --run scripts/ProtectedStatements.lean 2>&1 >/dev/null) || status=$?
  case "$expect" in
    reject)
      if [ "$status" -eq 0 ]; then
        echo "FAIL $name: the generator accepted the attribute on a reached constant"
        failures=$((failures + 1))
      elif ! grep -qF "ContractGraph.$name " <<<"$out"; then
        echo "FAIL $name: the generator failed without naming ContractGraph.$name:"
        echo "$out"
        failures=$((failures + 1))
      else
        echo "ok   $name: rejected"
      fi ;;
    accept)
      if [ "$status" -ne 0 ]; then
        echo "FAIL $name: the generator rejected the attribute on a constant no protected theorem reaches:"
        echo "$out"
        failures=$((failures + 1))
      else
        echo "ok   $name: accepted"
      fi ;;
  esac
}

# Reached only through runChecker's value.
run_case lastN reject \
  'def selftestLastN {α : Type} (n : Nat) (xs : List α) : List α := xs.drop (xs.length - n)' \
  's/^def lastN /\@[implemented_by selftestLastN] def lastN /'

# Named in the statement of runChecker_exitCode_eq_zero_iff.
run_case defaultMaxStates reject \
  '' \
  's/^def defaultMaxStates /\@[extern "cgv_selftest_default_max_states"] def defaultMaxStates /'

# Reached only through the compiled body of a partial def: the kernel sees an
# opaque constant, and the compiler runs selftestPartial._unsafe_rec.
run_case selftestHidden reject \
  'def selftestImpl (n : Nat) : Nat := n
@[implemented_by selftestImpl] def selftestHidden (n : Nat) : Nat := n
partial def selftestPartial (n : Nat) : Nat := if n = 0 then selftestHidden n else selftestPartial (n - 1)' \
  's/^def defaultMaxStates : Nat := 2000000$/def defaultMaxStates : Nat := selftestPartial 2000000/'

# Reached only through the value of an opaque constant, which the compiler
# runs and the kernel never unfolds.
run_case selftestInner reject \
  'def selftestInnerImpl (n : Nat) : Nat := n
@[implemented_by selftestInnerImpl] def selftestInner (n : Nat) : Nat := n
opaque selftestOpaque : Nat → Nat := selftestInner' \
  's/^def defaultMaxStates : Nat := 2000000$/def defaultMaxStates : Nat := selftestOpaque 2000000/'

# The JSON output: no protected theorem or definition reaches it.
run_case outputToJson accept \
  'def selftestOutputToJson (_ : CheckOutput) : String := ""' \
  's/^def outputToJson /\@[implemented_by selftestOutputToJson] def outputToJson /'

if [ "$failures" -ne 0 ]; then
  echo "$failures self-test case(s) failed"
  exit 1
fi
