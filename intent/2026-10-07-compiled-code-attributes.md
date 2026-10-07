# Intent: Reject `implemented_by` and `extern` on what the soundness theorems reach

Task TB-1.2. Issue #48. Roadmap item TB-1.

## Problem statement

The `runChecker_*` soundness theorems are proved about the Lean definitions as written. The `contract-graph-checker` binary runs the compiled code. Two attributes make the compiled code differ from the definition without adding an axiom:

- `@[implemented_by g] def f` makes the compiler run `g` wherever `f` is called. The kernel still checks proofs against `f`.
- `@[extern "sym"] def f` makes the compiler call a C symbol for `f`. The kernel still checks proofs against `f`.

If either attribute sat on `runChecker`, or on anything it calls, exit 0 from the binary would no longer mean what `runChecker_sound_all` proves. The statement manifest would not change, and the SM-6 axiom check would pass, because neither attribute is an axiom. Nothing in `ContractGraph/` uses either attribute today, so this is a guard against a future change, not a fix of a present bug.

## Proposed outcome

`cgv/prover/scripts/ProtectedStatements.lean` fails, and so CGV CI fails, if any constant defined in a `ContractGraph` module and reachable from a protected theorem's statement or a protected definition carries `implemented_by` or `extern`. A self-test in CGV CI injects each attribute into a reached definition and fails if the generator stops rejecting it.

## Affected users and systems

- Anyone who relies on CGV's exit 0. For the project constants that a protected theorem's statement or a protected definition reaches, the binary runs the code the theorems are about. `main`, the translation and the JSON output are not reached: see the spec.
- The CGV maintainer. A change that adds either attribute to the reached code fails CI and has to be argued in a protected-surface amendment.
- `cgv/prover/scripts/ProtectedStatements.lean` (protected), `.github/workflows/cgv-ci.yml` (protected), `.claude/rules/protected-surfaces.md` (protected), `cgv/CLAUDE.md`, `cgv/README.md`.

## Constraints

- The manifest `cgv/prover/protected-statements.txt` must not change. The check adds a failure mode, not a printed line.
- The check reads only the built environment. No LLM, no network.
- The reach follows what the compiled code can run, so it includes every constant the theorems' statements name (such as `runChecker`), not only the three protected definitions the manifest hashes.

## Open questions

None. Two related gaps are recorded as not yet reached in the spec and the queue, not settled here: library code outside the project, and `@[csimp]` lemmas.
