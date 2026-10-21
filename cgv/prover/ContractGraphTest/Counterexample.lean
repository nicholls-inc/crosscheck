-- Counterexample.lean
-- The counterexample of each error kind (WT-1 to WT-8 of
-- intent/2026-10-07-error-witness-spec.md): the value, its Python text, the
-- cases that carry none, and the pairing in runCheckerCx.

import ContractGraph.Types
import ContractGraph.Main

namespace ContractGraphTest.Counterexample

open ContractGraph

def c (k : ConstraintKind) (b : Option Int := none) (cs : Option (List String) := none)
    (ty : Option String := none) (scale : Nat := 6) : Constraint :=
  { kind := k, staticBound := b, choicesList := cs, typeName := ty, scale := scale,
    sourceFile := "m.py", sourceLine := 1, verificationLevel := .extracted }

def cx (s t : Constraint) : Option String := (counterexampleOf s t).map (·.toPython)

-- WT-2 length
#guard cx (c .length (some 64)) (c .length (some 32)) == some "\"x\" * 64"
#guard cx (c .length (some 0)) (c .length (some (-1))) == some "\"x\" * 0"
-- WT-3 precision
#guard cx (c .precision (some 4)) (c .precision (some 2)) == some "Decimal(\"0.0001\")"
#guard cx (c .precision (some 1)) (c .precision (some 0)) == some "Decimal(\"0.1\")"
-- WT-4 range: a whole number is an integer literal, otherwise a Decimal
#guard cx (c .range (some 11000000)) (c .range (some 10000000)) == some "11"
#guard cx (c .range (some 700000)) (c .range (some 500000)) == some "Decimal(\"0.7\")"
#guard cx (c .range (some (-500000))) (c .range (some (-1000000))) == some "Decimal(\"-0.5\")"
#guard cx (c .range (some 25) (scale := 1)) (c .range (some 20) (scale := 1)) == some "Decimal(\"2.5\")"
-- WT-5 range_min
#guard cx (c .rangeMin (some (-1000000))) (c .rangeMin (some 0)) == some "-1"
#guard cx (c .rangeMin (some (-50000000))) (c .rangeMin (some 0)) == some "-50"
-- WT-6 choices: the first source choice the target lacks, as a Python string
#guard cx (c .choices (cs := some ["a", "zz", "yy"])) (c .choices (cs := some ["a", "x"])) ==
  some "\"zz\""
#guard cx (c .choices (cs := some ["3"])) (c .choices (cs := some ["1", "2"])) == some "\"3\""
-- WT-7 nullability
#guard cx (c .nullability (some 1)) (c .nullability (some 0)) == some "None"

-- WT-6 escapes: `\` and `"`, and below U+0020 or from U+007F the shortest of
-- \xNN, \uNNNN, \UNNNNNNNN with lowercase hex digits
#guard pythonStringLiteral "a\"b\\c" == "\"a\\\"b\\\\c\""
#guard pythonStringLiteral "\n\t" == "\"\\x0a\\x09\""
#guard pythonStringLiteral "\x7f" == "\"\\x7f\""
#guard pythonStringLiteral "é" == "\"\\xe9\""
#guard pythonStringLiteral "€" == "\"\\u20ac\""
#guard pythonStringLiteral "😀" == "\"\\U0001f600\""
#guard pythonStringLiteral "plain ~" == "\"plain ~\""

-- WT-1: no counterexample for a consistent pair, a type pair, the three
-- excluded cases, or kinds that differ
#guard cx (c .length (some 10)) (c .length (some 32)) == none
#guard cx (c .type (ty := some "str")) (c .type (ty := some "int")) == none
#guard cx (c .precision (some 0)) (c .precision (some (-1))) == none
#guard cx (c .length (some (-1))) (c .length (some (-2))) == none
#guard cx (c .range (some 11000000)) (c .range (some 1) (scale := 0)) == none
#guard cx (c .rangeMin (some 0) (scale := 0)) (c .rangeMin (some 5)) == none
#guard cx (c .nullability (some 0)) (c .nullability (some 0)) == none
#guard cx (c .choices (cs := some ["a"])) (c .choices (cs := some ["a", "b"])) == none
#guard cx (c .length (some 64)) (c .precision (some 2)) == none

-- A warning carries none, even over constraints that would give a value.
#guard (missingPostconditionWarning (c .nullability (some 0)) "f" "M.x").counterexample == none
#guard ({ severity := .error, sourceConstraint := c .length (some 9),
          targetConstraint := c .length (some 5), path := [], suggestion := "" } :
          DiagnosticInfo).counterexample == some (.str 9)

-- runCheckerCx pairs each entry with its own counterexample (two errors on one
-- edge) and prints runChecker's output.
def producer : Node :=
  { id := 1, name := "make_label", kind := "function", preconditions := []
    postconditions := [c .length (some 64), c .nullability (some 1)] }

def field : Node :=
  { id := 2, name := "Invoice.label", kind := "model", postconditions := []
    preconditions := [c .length (some 32), c .nullability (some 0)] }

def graph : ContractGraph :=
  { nodes := [producer, field]
    edges := [{ source := producer, target := field, relationship := .writesTo }] }

#guard ((runCheckerCx graph).1.results.map (·.sourceGuarantee)).zip
    ((runCheckerCx graph).2.map (·.map (·.toPython))) ==
  [("length ≤ 64", some "\"x\" * 64"), ("nullable", some "None")]
#guard (runCheckerCx graph).1.results.length == (runChecker graph).results.length
#guard (runCheckerCx graph).1.exitCode == 1

#guard ((outputToJson (runCheckerCx graph).1 (runCheckerCx graph).2).splitOn
    "\"counterexample\": \"\\\"x\\\" * 64\"").length == 2
#guard ((outputToJson (runCheckerCx graph).1 (runCheckerCx graph).2).splitOn
    "\"counterexample\"").length == 3

-- WT-10: the proof uses no axiom beyond Lean's standard three.
/--
info: 'ContractGraph.checkConstraintPair_counterexample' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms checkConstraintPair_counterexample

/--
info: 'ContractGraph.runCheckerCx_fst' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms runCheckerCx_fst

end ContractGraphTest.Counterexample
