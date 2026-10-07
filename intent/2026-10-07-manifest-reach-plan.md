# Plan: Hash what the protected statements mention, and check the name lists against the rules table

Intent: `intent/2026-10-07-manifest-reach.md`. Spec: `intent/2026-10-07-manifest-reach-spec.md`. Task TB-1.3, issue #51, roadmap item TB-1. Tier 3: the generator, the manifest, the CI workflow and the rules file are protected.

## Files, in order

1. `.assurance/protected-surface-amend/manifest-reach-2026-10-07.md`. The governance note, first, so the protected-surface hook allows the edits below.
2. `cgv/prover/scripts/ProtectedStatements.lean`:
   - `inScope env n` is true when `env.getModuleIdxFor? n` names a module under `ContractGraph`. It replaces the namespace-prefix test.
   - `roots env` returns the protected definitions and every constant used in a protected theorem's type. `reach` starts from it and checks `inScope` when it takes a name off the work list, so library constants in the statements are dropped.
   - `parseRulesTable text` returns the (name, file) rows of the CGV table (SM-9). `checkRulesTable env table` compares them with the two lists and with `modToFilePath "cgv/prover" module "lean"` of each constant's module, and throws one error that lists every difference.
   - `main` takes an optional rules-file path, reads and parses the table before importing the environment, and `render` calls `checkRulesTable` before any output.
   - The header comment and the manifest's section line say what is now hashed.
3. `cgv/prover/protected-statements.txt`: regenerated. It gains 279 blocks. The 12 theorem statements and the 107 blocks of the old manifest are unchanged. The one changed line is the section heading.
4. `cgv/prover/scripts/manifest-selftest.sh`: the ten cases of SM-10. It backs up `ContractGraph/Main.lean` and `ContractGraph/Composition.lean`, restores them with a `trap` on exit and rebuilds, and writes rules-file copies and generator outputs to a `mktemp -d` directory. A case that compares manifests extracts one constant's block (from its header line to the next blank line) from each output and compares the blocks.
5. `.github/workflows/cgv-ci.yml`: add `.claude/rules/protected-surfaces.md` to both `paths` lists (CI-1), and a step after the manifest step that runs the self-test (SM-11).
6. Docs: `.claude/rules/protected-surfaces.md` (what the manifest records, and that the generator checks the table), `cgv/CLAUDE.md` (the generator paragraph), `cgv/README.md` (trust model row), and SM-1, SM-4, CI-1 and the paragraph after SM-6 in `intent/2026-09-29-deterministic-evidence-spec.md`.
7. `docs/TASKS.md`: TB-1.3 `done` with this intent as its record. Root `JOURNAL.md` entry.

## Risks

- **Reach too narrow.** A definition a statement depends on could still escape. Mitigation: the roots are every constant in every statement, and the walk follows types, values and constructors through every module-scoped constant. Cases 7 and 8 of SM-10 check a constant reached only through a statement and a private helper.
- **Reach too wide.** Hashing unrelated code would make unrelated changes Tier 3. Case 9 checks that an unreached definition stays out.
- **Proof edits change the manifest.** Case 10 checks a proof-only edit to a termination proof, which a reached definition names through an auxiliary theorem.
- **More Tier 3 changes.** Every change to the checker's definitions now changes the manifest. That is the issue's request. The PR says so in Blast Radius.
- **Table parse too loose or too strict.** Cases 2 to 6 cover each failure, and case 1 checks that today's table parses.
- **Conflicts with PRs #99 and #101.** All three edit `cgv-ci.yml`, and #101 edits `ProtectedStatements.lean` and the same rules paragraph. Whichever merges second keeps both changes and regenerates the manifest.

## Proof that it works

- `lake build ContractGraph ContractGraph.Main && lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing.
- `scripts/manifest-selftest.sh` exits 0.
- Mutations of the generator, one at a time, each make a case fail: roots without the statement constants (case 7); a namespace-prefix `inScope` (case 8); a walk over every project constant (case 9); a walk that enters theorems (case 10); each branch of `checkRulesTable` removed (cases 2 to 5); `checkRulesTable` not called (cases 2 to 5).
