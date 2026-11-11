# Intent: Protect the counterexample and pairing theorems

Task: TB-1.43. Governing roadmap item: TB-1. Issue: #8. Spec: `intent/2026-10-07-error-witness-spec.md` (WT-12). The theorems come from task CG-1.35, `intent/2026-10-21-error-counterexample.md`.

## Problem statement
Since CG-1.35, `main` prints the output of `runCheckerCx`, not `runChecker`. `runChecker_sound_all` and the exit-code theorems are about `runChecker`, so they describe the binary only through `runCheckerCx_fst`, which proves the first component of `runCheckerCx`'s output is `runChecker`'s. `counterexampleOf_sound` and `checkConstraintPair_counterexample` are the guarantee of the new `counterexample` field. None of the three is in the theorem table of `.claude/rules/protected-surfaces.md` or in `cgv/prover/scripts/ProtectedStatements.lean`. A later pull request could weaken or delete any of them, or let `runCheckerCx` drift from `runChecker`, and the statement manifest would not change, so CGV CI would not notice.

## Proposed outcome
The three theorems are rows of the CGV theorem table and entries of `protectedTheorems`. The regenerated `cgv/prover/protected-statements.txt` records their statements and the definitions those statements reach, `runCheckerCx` and the counterexample definitions among them. No earlier block of the manifest changes. A change to any of these then changes the manifest and makes its pull request Tier 3.

## Affected users and systems
- Anyone who relies on CGV's exit code or on a printed counterexample being proved.
- `.claude/rules/protected-surfaces.md`, `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`, the governance note `.assurance/protected-surface-amend/protect-runcheckercx-fst-2026-10-21.md`, `cgv/CLAUDE.md`, `docs/TASKS.md` and `JOURNAL.md`.
- Future pull requests that touch `runCheckerCx`, its paired reporting functions or the counterexample definitions: they become Tier 3.

## Constraints
- No proof and no definition changes. The manifest only gains blocks.
- Rule 4 of `.claude/rules/protected-surfaces.md`: the judgment items of the governance note stay `REQUIRES HUMAN VERIFICATION:` markers until the maintainer resolves them. The maintainer's merge grant of 2026-10-21 authorizes merges, not ticks, so no agent merges this pull request.
- The pull request is stacked on #133 and is retargeted to `main` once #133 merges.

## Decided by the coordinator under the maintainer's authorization
The maintainer gave the coordinating agent of this run a full-autonomy and merge grant in chat on 2026-10-21. The coordinator made the calls below under that grant. This file records the authorization; it does not constitute it, and a statement in a repository file is not maintainer sign-off.

- **Split from #133.** The protection was part of #133. It moved here so that #133, which changes no protected surface, can merge at Tier 2, and this change waits for the maintainer.
- **`DiagnosticInfo.counterexample_sound` is not added.** It is a two-line corollary of `counterexampleOf_sound`. The governance note leaves this to the maintainer as a judgment item.

## Open questions
None for the agent. The governance note's judgment items are the maintainer's to resolve.
