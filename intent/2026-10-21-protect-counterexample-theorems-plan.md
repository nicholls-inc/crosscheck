# Plan: Protect the counterexample and pairing theorems

Intent: `intent/2026-10-21-protect-counterexample-theorems.md`. Spec: `intent/2026-10-07-error-witness-spec.md` (WT-12). Task TB-1.43, roadmap item TB-1. Tier 3: the rules file, the generator and the manifest change.

## Files, in order

1. `.assurance/protected-surface-amend/protect-runcheckercx-fst-2026-10-21.md`. The governance note, before the protected edits. Mechanical items ticked with evidence, judgment items left as markers.
2. `.claude/rules/protected-surfaces.md`. Two rows in the CGV theorem table: `runCheckerCx_fst` in `cgv/prover/ContractGraph/Main.lean`, and `counterexampleOf_sound`, `checkConstraintPair_counterexample` in `cgv/prover/ContractGraph/Counterexample.lean`.
3. `cgv/prover/scripts/ProtectedStatements.lean`. The three names in `protectedTheorems`.
4. `cgv/prover/protected-statements.txt`. Regenerated.
5. `cgv/CLAUDE.md`, `docs/TASKS.md` and `JOURNAL.md`.

## Risks

- **An earlier block changes.** The diff of the manifest must only add lines.
- **The table and the generator disagree.** The generator's rules-table check fails the build if they do.

## Proof that it works

- `lake build` passes with no `sorry`.
- `lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing, and `git diff origin/task/CG-1.35 -- cgv/prover/protected-statements.txt` has no removed line.
- `scripts/manifest-selftest.sh` and `scripts/compiled-code-selftest.sh` exit 0.
- The tier gate passes at Tier 3 on the pull request body.
