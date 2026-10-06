> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `cgv/prover/ContractGraph/BehaviorModel.lean` (+ 2 others, see Diff Plan)
**Class:** CGV proof surfaces (trusted semantics and the statement manifest). Guarded like Class A.
**Matched rule:** `cgv/prover/ContractGraph/BehaviorModel.lean`, `cgv/prover/protected-statements.txt`
**Date:** 2026-10-06

### Change Description

1. `cgv/prover/ContractGraph/BehaviorModel.lean`: added `annotationAcceptsNumeric`, the rule that an annotation `float` accepts a value of type `int` (PEP 484 numeric tower), with the evidence and the two cases it does not state (`bool` into `float`, and the `complex` half of the tower).
2. `cgv/prover/protected-statements.txt`: regenerated. The value hash of `constraintImplies` changes (1577221521 to 564077362), and a new entry records `ContractGraph.typeAccepts : String → String → Bool`, which `constraintImplies` now reaches. No theorem statement changes.

3. `cgv/prover/ContractGraph/Checker.lean`: the value of `constraintImplies`, a protected definition in the statement table of `.claude/rules/protected-surfaces.md`, changes. Its `.type` case goes from `st = tt` to `typeAccepts st tt = true`, with a new `typeAccepts st tt := st == tt || (st == "int" && tt == "float")`, which `checkTypeConsistency` also uses. The file as a whole is not a protected surface and no theorem statement in it changes, but this definition is why the manifest changes.

### Rationale

Task CG-1.7, issue #5. An `int` passed where `float` is annotated was 35 of the 53 triaged false positives in `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md`. PEP 484 says an `int` is acceptable where `float` is annotated. Reproduced on `origin/main` 9cb8972: `cgv/test_fixtures/numeric_tower/ok.py` gives two errors, `type = int` against `type = float`. The intent (`intent/2026-10-06-cgv-numeric-tower.md`) shows that an extractor-side fix would weaken the guarantee for more pairs than this change does.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item CG-1)
- **Title:** Make CGV findings worth reading on a real codebase
- **Scope coverage:** CG-1's scope is fewer false errors, and its issues include #5. Task CG-1.7 in `docs/TASKS.md` names this exact change and says it needs Tier 3, a governance note and a rule in `BehaviorModel.lean`.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `cgv/prover/ContractGraph/BehaviorModel.lean` | new section "Numeric tower" before `end` | added |
| 2 | `cgv/prover/protected-statements.txt` | `constraintImplies` hash, new `typeAccepts` entry | regenerated |
| 3 | `cgv/prover/ContractGraph/Checker.lean` | `constraintImplies` (`.type` case), new `typeAccepts` | protected definition edited, no statement changed |

### Test / Coverage Impact

- `cgv/test_fixtures/numeric_tower/` (new): `ok.py` must have no errors and `code.py` must keep three (`Decimal` into `float`, `str` into `float`, `float` into `int`). Mutation checked: with `typeAccepts` reverted to `st == tt` the fixture fails with the two `ok.py` errors, and with `typeAccepts` returning `true` it fails with the three `code.py` errors missing.
- `cgv/prover/ContractGraphTest/NumericTower.lean` (new): `#guard` lines on `checkConstraintPair`, two `constraintImplies` examples, a theorem `typeAccepts_iff_annotationAcceptsNumeric` that ties the executable check to the trusted rule (it stops building if either changes), and a multi-hop check that a value widened from `int` to `float` is a `float` at the next hop and is still rejected where an `int` is required.
- `pair_sound` and every soundness theorem build unchanged. The manifest generator reports no `sorry` or non-standard axiom.
- The guarantee shrinks for one pair: exit 0 no longer says anything about an `int` reaching a `float` requirement. That includes the value's magnitude: an `int` is exact and unbounded, but `float(v)` loses precision above 2^53 and raises `OverflowError` above about 1.8e308. PEP 484 checkers accept the pair regardless, and no extractor path is known that carries such a bound into a `float` requirement, so the rule does not state it.

### Review Checklist

- [x] Rationale is anchored to a concrete trigger (issue #5, the 2026-09 evaluation, a local reproduction).
- [x] Authoriser is a named human.
- [x] CG-1 exists and covers this change through task CG-1.7.
- [x] The diff plan names every changed protected file.
- [x] No invariant is weakened to make a failing test pass: the fixture failed because the check was stricter than PEP 484.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that exit 0 promises less for an `int` into a `float` requirement, and that `bool` into `float` and the `complex` half of the tower stay unstated (see the intent's Constraints).
