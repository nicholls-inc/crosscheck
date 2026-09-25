-- NoErrorsSoundness.lean
-- Soundness for what the tool reports: exit code 0 with warnings still gives
-- `stepwiseSound` for every enumerated path (`runChecker_sound`), while the
-- older `checkPath_sound` hypothesis (every result consistent) fails as soon
-- as a hop warns.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Main

namespace ContractGraphTest.NoErrorsSoundness

open ContractGraph

def c (k : ConstraintKind) (b : Int) : Constraint :=
  { kind := k, staticBound := some b, sourceFile := "t.py", sourceLine := 1,
    verificationLevel := .extracted }

/-- `make` guarantees 2dp but says nothing about nullability; the field
    requires 2dp and non-null. Consistent, with a missing-nullability warning. -/
def make : Node :=
  { id := 1, name := "make", kind := "function", preconditions := [],
    postconditions := [c .precision 2] }

def total : Node :=
  { id := 2, name := "Invoice.total", kind := "model",
    preconditions := [c .precision 2, c .nullability 0], postconditions := [] }

def graph : ContractGraph :=
  { nodes := [make, total]
    edges := [{ source := make, target := total, relationship := .writesTo }] }

-- The run reports a warning but exits 0.
#guard (runChecker graph).exitCode == 0
#guard (runChecker graph).results.map (·.severity) == ["warning"]

-- Not every result is consistent (so `checkPath_sound` does not apply) ...
#guard !((checkPath graph.edges).all fun r => match r with
    | .consistent => true
    | .inconsistent _ => false)
-- ... but none is an error.
#guard (checkPath graph.edges).all (·.isError == false)

-- `isError` distinguishes errors from warnings.
#guard CheckResult.isError .consistent == false
#guard (checkPath [{ source := { make with postconditions := [c .precision 4] },
                     target := total, relationship := .writesTo }]).any (·.isError)

-- `runChecker` goes through the `partial` path enumeration, which the kernel
-- cannot unfold, so this concrete fact is checked by compiled evaluation
-- (test module only; the library has no `native_decide`).
theorem graph_exit_zero : (runChecker graph).exitCode = 0 := by native_decide

-- End-to-end: every path the checker enumerates is stepwise sound.
theorem graph_sound : ∀ p ∈ enumeratePaths graph, p ≠ [] → stepwiseSound p :=
  runChecker_sound graph graph_exit_zero

-- The exit-code characterisation, instantiated.
example : ∀ e ∈ (runChecker graph).results, e.severity ≠ "error" :=
  (runChecker_exitCode_eq_zero_iff graph).mp graph_exit_zero

-- Pair-check inconsistencies are errors; hop tagging keeps the severity.
example (di : DiagnosticInfo)
    (h : checkConstraintPair (c .precision 4) (c .precision 2) = .inconsistent di) :
    di.severity = .error :=
  checkConstraintPair_severity _ _ di h

end ContractGraphTest.NoErrorsSoundness
