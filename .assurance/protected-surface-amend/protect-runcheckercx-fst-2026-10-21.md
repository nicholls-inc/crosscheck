## Protected-Surface Amendment

**Target file(s):** `.claude/rules/protected-surfaces.md`, `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`
**Class:** CGV proof surface (the theorem table, the manifest and its generator); A (harness rules)
**Matched rule:** `.claude/rules/**`, `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`
**Date:** 2026-10-21

### Change Description

1. `.claude/rules/protected-surfaces.md`: one row added to the CGV theorem table, `runCheckerCx_fst` in `cgv/prover/ContractGraph/Main.lean`.
2. `cgv/prover/scripts/ProtectedStatements.lean`: `ContractGraph.runCheckerCx_fst` added to `protectedTheorems`, so the generator's lists match the table.
3. `cgv/prover/protected-statements.txt`: regenerated. It gains the statement of `runCheckerCx_fst` and the blocks of the 39 constants that statement reaches and the manifest did not already hash: `runCheckerCx` and the paired reporting functions (`collectEntriesCx`, `guardedEntriesCx`, `stateEntriesCx`, `DedupeCx` and its `step` and `push`, `reportStatesWithCx`, `runStatesWithCx`), `Counterexample` with its constructors, `admits`, `counterexampleCandidate`, `counterexampleOf` and `DiagnosticInfo.counterexample`, and their compiler helpers. No earlier block and no earlier statement changes. `Counterexample.toPython` and the JSON printer are not reached.

### Rationale

Task CG-1.35 (issue #8), PR #133. Since that PR, `main` calls `runCheckerCx` and takes the exit code and the report from its first component. `runChecker_sound_all` and the exit-code theorems describe `runChecker`, so they describe the binary only through `runCheckerCx_fst`, which proves the first component is `runChecker`'s output. The rules file says a theorem that becomes part of the end-to-end guarantee goes into the table and the generator in the same PR. Without it, a later PR could weaken or delete `runCheckerCx_fst`, or let `runCheckerCx` drift from `runChecker`, with no manifest change. The pr-swarm review of #133 found this (thread on `cgv/prover/ContractGraph/Main.lean:987`). Intent: `intent/2026-10-21-error-counterexample.md`. Spec: `intent/2026-10-07-error-witness-spec.md` (WT-12, amended). Plan: `intent/2026-10-21-error-counterexample-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (item TB-1)
- **Title:** Shrink the trusted base, and measure what remains
- **Scope coverage:** TB-1 covers what CGV's exit code rests on that the kernel does not check for you. A link from the binary's output to the protected theorems that is itself unprotected is part of that base. The change is part of task CG-1.35 under CG-1, which created the link.

### Authority

- **Authoriser:** harry-nicholls. The maintainer gave the coordinating agent of this run a full-autonomy and merge grant in chat on 2026-10-21, and the coordinator decided this amendment under that grant. This note records the decision; it does not constitute the authorization.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/rules/protected-surfaces.md` | CGV theorem statements table | one row added |
| 2 | `cgv/prover/scripts/ProtectedStatements.lean` | `protectedTheorems` | one name added |
| 3 | `cgv/prover/protected-statements.txt` | whole file | regenerated; 1 statement and 39 constant blocks added, none changed or removed |

### Test / Coverage Impact

- `lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing after the regeneration. Before it, the diff was 207 added lines and no removed line.
- The generator's rules-table check passes, so the table and `protectedTheorems` agree on names and files.
- The axiom check now covers `runCheckerCx_fst`, and it depends only on `propext`, `Classical.choice` and `Quot.sound`. The compiled-code check now walks `runCheckerCx` and the counterexample definitions, and `scripts/compiled-code-selftest.sh` passes.
- `scripts/manifest-selftest.sh` passes all ten cases.
- No statement and no definition changes. Exit 0 promises the same. More changes now need protected-surface review: any change to `runCheckerCx`, the paired reporting functions or the counterexample definitions they call.

### Review Checklist

- [x] Rationale is anchored to the pr-swarm thread on `cgv/prover/ContractGraph/Main.lean:987` of #133.
- [x] Authoriser is a named human.
- [x] TB-1 is in `docs/assurance/ROADMAP.md`.
- [x] The diff plan names every changed protected file: `git diff --name-only main...HEAD` lists `.claude/rules/protected-surfaces.md`, `cgv/prover/protected-statements.txt` and `cgv/prover/scripts/ProtectedStatements.lean` among the protected paths, and no other.
- [x] No check is weakened: the manifest only gains blocks, and the generator only gains a theorem to check.
- [x] The coordinating agent accepted, under the maintainer's grant of 2026-10-21, that a change to `runCheckerCx` or the counterexample definitions it reaches now needs Tier 3 review.
