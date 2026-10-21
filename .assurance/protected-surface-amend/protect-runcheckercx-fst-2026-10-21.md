## Protected-Surface Amendment

**Target file(s):** `.claude/rules/protected-surfaces.md`, `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`
**Class:** CGV proof surface (the theorem table, the manifest and its generator); A (harness rules)
**Matched rule:** `.claude/rules/**`, `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`
**Date:** 2026-10-21

### Change Description

1. `.claude/rules/protected-surfaces.md`: two rows added to the CGV theorem table, `runCheckerCx_fst` in `cgv/prover/ContractGraph/Main.lean`, and `counterexampleOf_sound` and `checkConstraintPair_counterexample` in `cgv/prover/ContractGraph/Counterexample.lean`.
2. `cgv/prover/scripts/ProtectedStatements.lean`: `ContractGraph.runCheckerCx_fst`, `ContractGraph.counterexampleOf_sound` and `ContractGraph.checkConstraintPair_counterexample` added to `protectedTheorems`, so the generator's lists match the table.
3. `cgv/prover/protected-statements.txt`: regenerated. It gains the statement of `runCheckerCx_fst` and the blocks of the 39 constants that statement reaches and the manifest did not already hash: `runCheckerCx` and the paired reporting functions (`collectEntriesCx`, `guardedEntriesCx`, `stateEntriesCx`, `DedupeCx` and its `step` and `push`, `reportStatesWithCx`, `runStatesWithCx`), `Counterexample` with its constructors, `admits`, `counterexampleCandidate`, `counterexampleOf` and `DiagnosticInfo.counterexample`, and their compiler helpers. It also gains the statements of `counterexampleOf_sound` and `checkConstraintPair_counterexample`, which reach no constant the manifest did not already hash. No earlier block and no earlier statement changes. `Counterexample.toPython` and the JSON printer are not reached.

### Rationale

Task TB-1.43. This amendment was first part of PR #133 (task CG-1.35, issue #8). It moved to its own pull request because its judgment items need the maintainer, and rule 4 of `.claude/rules/protected-surfaces.md` makes a merge with an unresolved marker a governance violation. Since #133, `main` calls `runCheckerCx` and takes the exit code and the report from its first component. `runChecker_sound_all` and the exit-code theorems describe `runChecker`, so they describe the binary only through `runCheckerCx_fst`, which proves the first component is `runChecker`'s output. The rules file says a theorem that becomes part of the end-to-end guarantee goes into the table and the generator in the same PR. Without it, a later PR could weaken or delete `runCheckerCx_fst`, or let `runCheckerCx` drift from `runChecker`, with no manifest change. The pr-swarm review of #133 found this (thread on `cgv/prover/ContractGraph/Main.lean:987`).

Protecting `runCheckerCx_fst` froze the counterexample definitions in the manifest, but not the theorems that make the new `counterexample` field a proved claim: `counterexampleOf_sound` (every printed counterexample is admitted by the source and rejected by the target) and `checkConstraintPair_counterexample` (WT-10, every error of the six kinds outside WT-1 has one). They are not part of the exit-0 guarantee, but they are the guarantee of the field, and without them in the table a later PR could narrow or delete them with no manifest change. The pr-swarm review of #133 found this too (thread on `.claude/rules/protected-surfaces.md:89`). `DiagnosticInfo.counterexample_sound` is a two-line corollary of `counterexampleOf_sound` and is not added.

Intent: `intent/2026-10-21-protect-counterexample-theorems.md`. Spec: `intent/2026-10-07-error-witness-spec.md` (WT-12). Plan: `intent/2026-10-21-protect-counterexample-theorems-plan.md`. The theorems themselves come from `intent/2026-10-21-error-counterexample.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (item TB-1)
- **Title:** Shrink the trusted base, and measure what remains
- **Scope coverage:** TB-1 covers what CGV's exit code rests on that the kernel does not check for you. A link from the binary's output to the protected theorems that is itself unprotected is part of that base, and so is a proved output field whose theorems can change silently. Task TB-1.43 sits under TB-1. Task CG-1.35 under CG-1 created both.

### Authority

- **Authoriser:** hnipps (Harry Nicholls). The maintainer's merge is the approval. The maintainer gave the coordinating agent of this run a full-autonomy and merge grant in chat on 2026-10-21. That grant authorizes merges. It is not an instruction to tick the judgment items below, so an agent proposed this amendment, the judgment items stay open for the maintainer, and no agent merges this pull request.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/rules/protected-surfaces.md` | CGV theorem statements table | two rows added |
| 2 | `cgv/prover/scripts/ProtectedStatements.lean` | `protectedTheorems` | three names added |
| 3 | `cgv/prover/protected-statements.txt` | whole file | regenerated; 3 statements and 39 constant blocks added, none changed or removed |

### Test / Coverage Impact

- `lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing after the regeneration. Before adding the two counterexample theorems, the diff was 20 added lines and no removed line: their two statements.
- The generator's rules-table check passes, so the table and `protectedTheorems` agree on names and files.
- The axiom check now covers the three theorems, and each depends only on `propext`, `Classical.choice` and `Quot.sound`. The compiled-code check now walks `runCheckerCx` and the counterexample definitions, and `scripts/compiled-code-selftest.sh` passes.
- `scripts/manifest-selftest.sh` passes all ten cases.
- No statement and no definition changes. Exit 0 promises the same. More changes now need protected-surface review: any change to `runCheckerCx`, the paired reporting functions, the counterexample definitions they call, or the statements of `counterexampleOf_sound` and `checkConstraintPair_counterexample`.

### Review Checklist

The maintainer resolves the judgment items below before merge.

Mechanical items.

- [x] Authoriser is a named human, not a bot and not an agent.
  Command: the handle `hnipps` matches none of `*-bot`, `*[bot]*`, `claude-*` or an agent name. Output: no match.
- [x] Governing roadmap item exists.
  Command: `grep -n "TB-1 — Status" docs/assurance/ROADMAP.md`. Output: `135:**TB-1 — Status: In progress.** Serves rule 2. Scope: CGV's extraction from`.
- [x] Diff plan names every changed protected file.
  Command: `git diff --name-only origin/task/CG-1.35...HEAD`, filtered by the globs in `.claude/rules/protected-surfaces.md`. Output: `.claude/rules/protected-surfaces.md`, `cgv/prover/protected-statements.txt`, `cgv/prover/scripts/ProtectedStatements.lean`. The diff plan names the same three. `origin/task/CG-1.35` changes no protected file against `origin/main`, so the list is the same after the pull request is retargeted to `main`.
- [x] This amendment block appears in the PR body.
  Command: `gh pr view 140 --json body -q .body | grep -c '^## Protected-Surface Amendment'`. Output: `1`.
- [ ] No unresolved human-verification marker is left.
  Command: `grep -c "^- \[ \] REQUIRES HUMAN VERIFICATION:"` on this file. Output: 6. This item is ticked once the maintainer resolves the six markers below.

Judgment items. A human decides these, or an agent ticks them on the maintainer's explicit instruction.

- [ ] REQUIRES HUMAN VERIFICATION: Rationale is anchored to a concrete trigger: the pr-swarm threads on `cgv/prover/ContractGraph/Main.lean:987` and `.claude/rules/protected-surfaces.md:89` of #133.
- [ ] REQUIRES HUMAN VERIFICATION: TB-1 actually covers this change.
- [ ] REQUIRES HUMAN VERIFICATION: The diff plan covers every affected section of the three files.
- [ ] REQUIRES HUMAN VERIFICATION: No check is weakened: the manifest only gains blocks, and the generator only gains theorems to check.
- [ ] REQUIRES HUMAN VERIFICATION: Accepted trade-off. A change to `runCheckerCx`, the paired reporting functions or the counterexample definitions they reach now needs Tier 3 review.
- [ ] REQUIRES HUMAN VERIFICATION: `counterexampleOf_sound` and `checkConstraintPair_counterexample` belong in the table although they are not part of the exit-0 guarantee, and `DiagnosticInfo.counterexample_sound` does not need to be added.
