# Proof request for AXLE web interface or Aristotle

Paste everything between the ``` fences into the prover. This is fully self-contained Lean 4 (v4.28+, Mathlib available but not required).

## The theorem

`pair_sound`: if `checkConstraintPair c d` returns `.consistent` and `c.kind = d.kind`, then `constraintImplies c d` holds. The mathematical content is trivial — the difficulty is getting Lean's match compiler output to reduce correctly.

## Known issues from 6+ failed attempts

1. After `cases c.kind`, matches compiled from `constraintImplies` do NOT reduce via `dsimp` — even with individual (non-combined) patterns
2. `simp_all [checkNullability]` causes an infinite loop via `checkNullability.eq_1`
3. Using `simp_all` introduces fresh metavariables (`val✝`) that disconnect goal variables from hypothesis variables
4. The `split` tactic on the goal works correctly for `constraintImplies`'s match, but `split at h` creates vacuous `false = true` subgoals from the kind-mismatch guard that need explicit handling

## What works

- `split` on the goal correctly resolves `constraintImplies`'s outer and inner matches
- `kind_bne_false hk` correctly produces `(c.kind != d.kind) = false`
- After `split` + `rename_i`, the goal has concrete `sg`, `tr`, `hsb1`, `hsb2` etc.
- The hypothesis `h` needs to be rewritten using these named equations, then `checkStaticBounds`/`checkNullability`/`checkTypeConsistency` unfolded, then the if-then-else split to extract the constraint

```lean
inductive ConstraintKind where
  | precision | nullability | type | range | length | choices
  deriving Repr, BEq, Hashable, Inhabited

inductive VerificationLevel where
  | proved | tested | extracted | assumed
  deriving Repr, BEq, Hashable, Inhabited

inductive DepExpr where
  | lit : Int → DepExpr | input : String → DepExpr
  | max : DepExpr → DepExpr → DepExpr | min : DepExpr → DepExpr → DepExpr
  | add : DepExpr → DepExpr → DepExpr | sub : DepExpr → DepExpr → DepExpr
  deriving Repr, BEq, Inhabited

structure Constraint where
  kind : ConstraintKind
  staticBound : Option Int := none
  depExpr : Option DepExpr := none
  typeName : Option String := none
  choicesList : Option (List String) := none
  sourceFile : String
  sourceLine : Nat
  verificationLevel : VerificationLevel
  deriving Repr

inductive Severity where | error | warning deriving Repr, BEq, Inhabited

structure DiagnosticInfo where
  severity : Severity
  sourceConstraint : Constraint
  targetConstraint : Constraint
  path : List String
  suggestion : String
  deriving Repr

inductive CheckResult where
  | consistent : CheckResult
  | inconsistent : DiagnosticInfo → CheckResult
  deriving Repr

def checkStaticBounds (sg tr : Int) (source target : Constraint) : CheckResult :=
  if sg ≤ tr then .consistent
  else .inconsistent { severity := .error, sourceConstraint := source, targetConstraint := target, path := [],
    suggestion := s!"Source guarantees ≤ {sg}, target requires ≤ {tr}. Either tighten the source or widen the target." }

def checkNullability (sn tn : Bool) (source target : Constraint) : CheckResult :=
  if sn && !tn then .inconsistent { severity := .error, sourceConstraint := source, targetConstraint := target,
    path := [], suggestion := "Source may produce null values, but target requires non-null." }
  else .consistent

def checkTypeConsistency (st tt_ : String) (source target : Constraint) : CheckResult :=
  if st == tt_ then .consistent
  else .inconsistent { severity := .error, sourceConstraint := source, targetConstraint := target, path := [],
    suggestion := s!"Source produces type '{st}', target expects '{tt_}'." }

def checkConstraintPair (source target : Constraint) : CheckResult :=
  if source.kind != target.kind then .consistent
  else match source.kind with
    | .precision | .length | .range =>
      match source.staticBound, target.staticBound with
      | some sg, some tr => checkStaticBounds sg tr source target
      | _, _ => .consistent
    | .nullability =>
      match source.staticBound, target.staticBound with
      | some sn, some tn => checkNullability (sn != 0) (tn != 0) source target
      | _, _ => .consistent
    | .type =>
      match source.typeName, target.typeName with
      | some st, some tt_ => checkTypeConsistency st tt_ source target
      | _, _ => .consistent
    | .choices => .consistent

def constraintImplies (c d : Constraint) : Prop :=
  c.kind = d.kind →
  match c.kind with
  | .precision | .length | .range =>
    match c.staticBound, d.staticBound with
    | some sg, some tr => sg ≤ tr
    | _, _ => True
  | .nullability =>
    match c.staticBound, d.staticBound with
    | some sn, some tn => sn = 0 ∨ tn ≠ 0
    | _, _ => True
  | .type =>
    match c.typeName, d.typeName with
    | some st, some tt_ => st = tt_
    | _, _ => True
  | .choices => True

private theorem kind_bne_false {a b : ConstraintKind} (h : a = b) :
    (a != b) = false := by subst h; cases a <;> rfl

theorem pair_sound (c d : Constraint) (hk : c.kind = d.kind)
    (h : checkConstraintPair c d = .consistent) :
    constraintImplies c d := by
  sorry
```
