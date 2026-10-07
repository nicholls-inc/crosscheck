# Spec: The manifest hashes what the protected statements mention, and the generator's lists match the rules table

Intent: `intent/2026-10-07-manifest-reach.md`. It amends SM-1, SM-4 and CI-1 of `intent/2026-09-29-deterministic-evidence-spec.md` and adds SM-9 to SM-11. SM-7, SM-7a and SM-8 are taken by open PR #101. Governing roadmap item: TB-1. Task TB-1.3, issue #51.

## Requirements

- **SM-1 (amended).** `cgv/prover/scripts/ProtectedStatements.lean` prints, in a fixed order:
  - the type of every theorem in `protectedTheorems`;
  - the type and a structural value hash of every *reached* constant. A reached constant is not a theorem, is defined in a module whose name starts with `ContractGraph` whatever its own namespace, and is reached from a root through the constants used in types and values, and an inductive type's constructors. The roots are the protected definitions `constraintImplies`, `IsDataPath` and `stepwiseSound`, and every constant used in the statement of a protected theorem. Inductive types include their constructors.
- **SM-4 (amended).** Changing only a proof leaves the output unchanged. Changing a listed statement, or the type or value of a reached constant, changes the output. A termination proof (`decreasing_by`) counts as a proof: replacing `checkPath`'s `decreasing_by simp_wf` with `decreasing_by all_goals (simp only [List.length_cons]; omega)` left the output unchanged.
- **SM-9.** The generator reads the CGV table in `.claude/rules/protected-surfaces.md`: the first Markdown table after the line that starts `**CGV theorem statements`. Each row has two columns. The first holds one or more backticked names, which the generator reads as names in the `ContractGraph` namespace. The second holds one backticked file path. The generator fails with a non-zero exit, before printing anything, and names each difference, if:
  - a name in the table is in neither `protectedTheorems` nor `protectedDefinitions`;
  - a name in either list is not in the table;
  - a name appears in the table more than once;
  - a row's file is not `cgv/prover/` followed by the path of the module that defines the constant, or the constant is not defined;
  - the heading line is missing, the table has no rows, or a row does not have two columns.

  It reads `../../.claude/rules/protected-surfaces.md`, relative to `cgv/prover`, unless a path is given as its first argument.
- **SM-10.** `cgv/prover/scripts/manifest-selftest.sh` checks SM-1, SM-4 and SM-9 against the real sources and exits non-zero if any case gives the other result, or if an edit matches nothing. Each source edit is to `ContractGraph/Main.lean`, which the script restores on exit. Each table edit is to a copy of the rules file.
  1. The unedited sources and rules file give the committed manifest.
  2. Table without `checkPath_sound`: the generator fails and says `ContractGraph.checkPath_sound` is not in the rules table.
  3. Table with an extra name `selftestUnlisted`: the generator fails and says it is not in the generator's lists.
  4. Table that puts `constraintImplies` in `Composition.lean`: the generator fails and names the file it is defined in.
  5. Table that lists `enumeratePaths_complete` twice: the generator fails and says it appears more than once.
  6. Rules file without the heading line: the generator fails and names the missing line.
  7. `incompleteWith` returns status `"incomplete!"` instead of `"incomplete"`. Only `incompleteWith_exitCode`'s statement reaches it. The manifest block of `ContractGraph.incompleteWith` changes.
  8. `incompleteWith` takes its status from a `private def`. Changing that helper's string changes the helper's manifest block. A name-prefix scope would skip the helper, which is the reason SM-1 scopes by module.
  9. `outputToJson`, which nothing reaches, changes: the manifest does not change.
  10. The proof of `incompleteWith_exitCode` changes from `rfl` to `by rfl`: the manifest does not change.
- **SM-11.** CGV CI runs the self-test after the manifest step.
- **CI-1 (amended).** CGV CI runs on pull requests and on pushes to `main` that change `cgv/**`, the workflow file, or `.claude/rules/protected-surfaces.md`.

## Effect on the guarantee

Exit 0 promises the same. What changes is what a reviewer sees: a change to any definition that a protected statement mentions, or that such a definition reaches, now changes the manifest, and so needs Tier 3 review. Before, a change to `CheckResult.isError`, `runChecker` or the rest of the checker could change what the theorems promise without any protected-surface review.

## Not yet reached

- **Opaque values.** The hash covers `ConstantInfo.value?`, which is empty for an `opaque` constant and for a `partial def`. The kernel cannot unfold either, so no theorem can depend on the body, and the statements mean the same whatever the body is. What the binary runs for them is a separate question, which #101 (TB-1.2, compiled-code attributes) takes up. No reached constant is opaque today.
- **Toolchain changes.** A Lean upgrade can change the pretty-printed types or the hash function and so the whole manifest. SM-3 already accepts that: an upgrade changes the trusted kernel and deserves Tier 3 review.
- **Rules table prose.** SM-9 checks the table's names and files. It does not check the sentences around the table, for example the statement that a new soundness theorem must be added to it. Whether a new theorem belongs in the guarantee is a human judgement.
