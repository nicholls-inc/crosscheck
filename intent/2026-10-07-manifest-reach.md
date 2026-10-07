# Intent: Hash what the protected statements mention, and check the name lists against the rules table

Task TB-1.3 in `docs/TASKS.md`, issue #51, roadmap item TB-1.

## Problem statement

The CGV statement manifest, `cgv/prover/protected-statements.txt`, has two gaps.

1. **Some definitions that the protected theorems mention are not hashed.** The manifest records the statement of each protected theorem, and the type and value hash of `constraintImplies`, `IsDataPath`, `stepwiseSound` and every `ContractGraph` constant those three reach. A statement can mention a definition that none of the three reaches. `checkEdgeAll_sound_noErrors` assumes `r.isError = false` for every result. If `CheckResult.isError` were redefined to return `true` for every result, that hypothesis would hold only when the edge yields no result at all, so the theorem would promise almost nothing. The manifest would not change, so if the proofs still built, CI would pass. The same holds for `ResultEntry.severity`, `runChecker`, `checkPath`, `enumeratePaths`, `closedStates`, `searchSetup`, `checkHop` and `incompleteWith`. On `main`, 107 constants are hashed. The protected statements mention 35 project constants, and those reach 386.
2. **The generator's name lists are kept in step with the rules table by hand.** `protectedTheorems` and `protectedDefinitions` in `cgv/prover/scripts/ProtectedStatements.lean` mirror the CGV table in `.claude/rules/protected-surfaces.md`. Only a comment says to keep the two in step. A theorem added to the table and not to the generator is not in the manifest, so a later change to its statement passes CI. `cgv-ci.yml` does not run when only `.claude/rules/**` changes, so a pull request that edits only the table runs no CGV check at all.

## Proposed outcome

- The manifest hashes every non-theorem constant, defined in a `ContractGraph` module, that a protected theorem's statement or a protected definition reaches. Changing what a protected statement means changes the manifest.
- A constant defined in a `ContractGraph` module counts whatever its namespace. A `private` helper's name starts with `_private`, so today's name-prefix scope would skip it. No reached constant is private today, so this changes no line of the manifest.
- The generator reads the CGV table in `.claude/rules/protected-surfaces.md` and fails if the table and its two lists name different constants, name one twice, or give a constant a file it is not defined in.
- CGV CI runs when `.claude/rules/protected-surfaces.md` changes.
- A self-test script shows that each of these checks fails when it should, and CI runs it.

## Affected users and systems

- The maintainer, who reviews changes to CGV's guarantee. More changes now show up in the manifest.
- Every CGV change that edits the checker's definitions. The checker (`checkEdge`, `checkPath`, `runChecker` and what they call) is now hashed, so a change to it changes the manifest and becomes Tier 3. That is the point of the change: the statements are about these definitions. It costs a governance note on each checker change.
- `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`, `.github/workflows/cgv-ci.yml` and `.claude/rules/protected-surfaces.md`, all protected surfaces.
- Open pull requests #99 (TB-1.1) and #101 (TB-1.2) edit the same generator, workflow and rules paragraph. Whichever merges second resolves the conflict.

## Constraints

- A proof-only edit must still leave the manifest unchanged.
- Theorem statements and the three protected definitions keep their current lines. The manifest only gains lines.
- No CI step calls an LLM.
- Use the toolchain pinned in `cgv/prover/lean-toolchain` (Lean 4.28.0).

## Open questions

None.
