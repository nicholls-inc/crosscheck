#!/usr/bin/env bash
# SM-10 in intent/2026-10-07-manifest-reach-spec.md. Checks what the statement
# generator records and when it fails: rules-table cases run the generator on an
# edited copy of .claude/rules/protected-surfaces.md, and source cases edit
# ContractGraph/Main.lean or ContractGraph/Composition.lean, rebuild, and compare
# the manifest with the one from the unedited sources. Both files are restored
# on exit.
#
# Usage, from cgv/prover after `lake build ContractGraph ContractGraph.Main`:
#   scripts/manifest-selftest.sh
set -euo pipefail

cd "$(dirname "$0")/.."
sources=(ContractGraph/Main.lean ContractGraph/Composition.lean)
rules=../../.claude/rules/protected-surfaces.md
work="$(mktemp -d "${TMPDIR:-/tmp}/cgv-manifest-selftest.XXXXXX")"
mkdir "$work/orig"
for f in "${sources[@]}"; do cp "$f" "$work/orig/"; done
reset_sources() {
  for f in "${sources[@]}"; do cp "$work/orig/$(basename "$f")" "$f"; done
}
restore() {
  reset_sources
  rm -rf "$work"
  lake build ContractGraph ContractGraph.Main >/dev/null 2>&1 || true
}
trap restore EXIT

failures=0
fail() {
  echo "FAIL $*"
  failures=$((failures + 1))
}

generate() {
  lake env lean --run scripts/ProtectedStatements.lean "$@"
}

# block FILE NAME: the manifest block of NAME, from its header line to the
# next blank line.
block() {
  awk -v name="$2" '
    $0 ~ /^[a-z]+ / && $2 == name && $3 == ":" { on = 1 }
    on && $0 == "" { exit }
    on { print }
  ' "$1"
}

# 1. The unedited sources and rules file give the committed manifest.
if ! generate >"$work/base.txt" 2>"$work/base.err"; then
  fail "baseline: the generator failed:"; cat "$work/base.err"
  exit 1
fi
if cmp -s "$work/base.txt" protected-statements.txt; then
  echo "ok   baseline: matches protected-statements.txt"
else
  fail "baseline: differs from protected-statements.txt"
fi

# table_case LABEL EXPECTED_MESSAGE PERL_EDIT
#   Applies PERL_EDIT to a copy of the rules file and expects the generator to
#   fail with EXPECTED_MESSAGE on stderr.
table_case() {
  local label=$1 message=$2 edit=$3
  local copy="$work/rules-$label.md"
  perl -pe "$edit" "$rules" >"$copy"
  if cmp -s "$copy" "$rules"; then
    fail "$label: the edit matched nothing; the rules table moved or changed"
    return
  fi
  local status=0
  generate "$copy" >/dev/null 2>"$work/err" || status=$?
  if [ "$status" -eq 0 ]; then
    fail "$label: the generator accepted the edited rules table"
  elif ! grep -qF -- "$message" "$work/err"; then
    fail "$label: the generator failed without saying \"$message\":"; cat "$work/err"
  else
    echo "ok   $label: rejected"
  fi
}

# 2-6. The rules table and the generator's lists must agree.
table_case missing-theorem \
  'ContractGraph.checkPath_sound is in the generator'"'"'s lists but not in the rules table' \
  's/^\| `checkPath_sound`, `checkPath_sound_noErrors` \|/| `checkPath_sound_noErrors` |/'
table_case unlisted-name \
  'ContractGraph.selftestUnlisted is in the rules table but not in protectedTheorems or protectedDefinitions' \
  's/^\| `enumeratePaths_complete` \|/| `enumeratePaths_complete`, `selftestUnlisted` |/'
table_case wrong-file \
  'the rules table puts ContractGraph.constraintImplies in cgv/prover/ContractGraph/Composition.lean, but it is defined in cgv/prover/ContractGraph/Checker.lean' \
  's/^\| `constraintImplies` \| `cgv\/prover\/ContractGraph\/Checker.lean` \|/| `constraintImplies` | `cgv\/prover\/ContractGraph\/Composition.lean` |/'
table_case duplicate \
  'ContractGraph.enumeratePaths_complete appears more than once in the rules table' \
  's/^\| `enumeratePaths_complete` \|/| `enumeratePaths_complete`, `enumeratePaths_complete` |/'
table_case no-heading \
  'the rules file has no line starting "**CGV theorem statements"' \
  's/^\*\*CGV theorem statements/**CGV statements/'

# edit_source LABEL FILE PERL_EDIT: applies PERL_EDIT to FILE and rebuilds.
edit_source() {
  local label=$1 file=$2 edit=$3 before
  before="$(mktemp "$work/before.XXXXXX")"
  cp "$file" "$before"
  perl -0pi -e "$edit" "$file"
  if cmp -s "$file" "$before"; then
    fail "$label: the edit matched nothing; the definition moved or changed"
    return 1
  fi
  if ! lake build ContractGraph ContractGraph.Main >/dev/null 2>&1; then
    fail "$label: the edited $file does not build"
    return 1
  fi
}

# source_case LABEL EXPECT NAME FILE SETUP_EDIT EDIT
#   Applies SETUP_EDIT (if any) to FILE and generates the reference manifest,
#   then applies EDIT and generates again. EXPECT "changed" wants NAME's block
#   to differ between the two, with both present. EXPECT "same" wants the two
#   manifests identical.
source_case() {
  local label=$1 expect=$2 name=$3 file=$4 setup=$5 edit=$6 ref="$work/base.txt"
  reset_sources
  if [ -n "$setup" ]; then
    edit_source "$label (setup)" "$file" "$setup" || return
    ref="$work/ref-$label.txt"
    generate >"$ref"
  fi
  edit_source "$label" "$file" "$edit" || return
  local out="$work/out-$label.txt"
  if ! generate >"$out" 2>"$work/err"; then
    fail "$label: the generator failed:"; cat "$work/err"
    return
  fi
  case "$expect" in
    changed)
      local old new
      old="$(block "$ref" "$name")"
      new="$(block "$out" "$name")"
      if [ -z "$old" ] || [ -z "$new" ]; then
        fail "$label: the manifest has no block for $name"
      elif [ "$old" = "$new" ]; then
        fail "$label: the manifest block of $name did not change"
      else
        echo "ok   $label: the block of $name changed"
      fi ;;
    same)
      if cmp -s "$ref" "$out"; then
        echo "ok   $label: the manifest did not change"
      else
        fail "$label: the manifest changed:"; diff -u "$ref" "$out" | head -20
      fi ;;
  esac
}

# 7. Only incompleteWith_exitCode's statement reaches incompleteWith.
source_case statement-mention changed ContractGraph.incompleteWith ContractGraph/Main.lean '' \
  's/(\ndef incompleteWith .*?status := "incomplete)"/$1!"/s'

# 8. A private helper of a reached definition is hashed too.
source_case private-helper changed _private.ContractGraph.Main.0.ContractGraph.selftestStatus ContractGraph/Main.lean \
  's/(\ndef incompleteWith )/\nprivate def selftestStatus : String := "incomplete"\n$1/; s/(\ndef incompleteWith .*?status := )"incomplete"/$1selftestStatus/s' \
  's/selftestStatus : String := "incomplete"/selftestStatus : String := "incomplete!"/'

# 9. Nothing protected reaches the JSON output.
source_case unreached same '' ContractGraph/Main.lean '' \
  's/(\ndef outputToJson \(output : CheckOutput\) : String :=\n  let resultsJson := .*?\n)/$1  "" ++\n/'

# 10. A proof-only edit: checkPath's termination proof, which Lean keeps in the
# auxiliary theorem checkPath._proof_1 that checkPath's value names.
source_case proof-only same '' ContractGraph/Composition.lean '' \
  's/(\n    checkHop edge \+\+ checkPath \(stepEdge edge nextEdge :: remainingEdges\)\ntermination_by path.length\ndecreasing_by )simp_wf\n/$1all_goals (simp only [List.length_cons]; omega)\n/'

if [ "$failures" -ne 0 ]; then
  echo "$failures self-test case(s) failed"
  exit 1
fi
