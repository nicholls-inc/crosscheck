# Plan: Lean v4.28.0 image and the imports the skills name

Intent: `intent/2026-10-27-lean-image-v4-28.md`. Spec: `intent/2026-10-27-lean-image-v4-28-spec.md`. Governing roadmap item: LT-1, added by this change. Tier 3, because the change edits three `SKILL.md` files and `docs/assurance/ROADMAP.md`.

## Files that change

1. `crosscheck/mcp-server/lean-harness/lean-toolchain`, `lakefile.lean` and `Crosscheck/Program.lean`, and `crosscheck/scripts/build-lean-docker.sh`. These pin Lean and Mathlib v4.28.0 and import `Mathlib.Data.Nat.Basic` in place of `Mathlib.Data.Nat.Defs`. Done in `cba551e`.
2. `docs/assurance/ROADMAP.md`. Adds item LT-1 in the Next horizon, and nothing else.
3. `docs/TASKS.md`. Adds task LT-1.1 for this pull request.
4. `crosscheck/skills/lean-spec/SKILL.md`, the import table:
   - `Mathlib.Data.Nat.Defs` becomes `Mathlib.Data.Nat.Basic`.
   - `Mathlib.Tactic.DeriveDecidableEq` becomes "no import", because `deriving DecidableEq` is in core Lean. The build-error table says the same.
   - `Mathlib.Analysis.SpecialFunctions.Basic` becomes `Mathlib.Data.Real.Basic`.
5. `crosscheck/skills/lean-impl/SKILL.md`, the import table:
   - `Mathlib.Data.Array.Basic` becomes "no import", because `Array.foldl` and `Array.mapIdx` are in core Lean.
   - `Mathlib.Data.HashMap.Basic` becomes `Std.Data.HashMap`.
6. `crosscheck/skills/drt-oracle/SKILL.md`. `Mathlib.Data.Json` becomes `Lean.Data.Json`.
7. `.assurance/protected-surface-amend/lean-image-v4-28-2026-10-28.md`. This is the governance note for items 2, 4, 5 and 6.

## Order

1. The image pins (item 1). Done.
2. The governance note (item 7). The hook refuses an edit to items 2, 4, 5 and 6 without it.
3. The roadmap item and the task row (items 2 and 3).
4. The skill tables (items 4 to 6).

## Risks

- **The image grows from 6.3 GB to 10.5 GB.** This was measured. A host with little disk can fail to build it, as one did on 2026-10-27.
- **Hosts keep the old image.** `crosscheck-lean:latest` stays v4.10.0 until the host rebuilds it. On the old image, the new skill tables name `Mathlib.Data.Nat.Basic`, which does not exist there. The PR body says to rebuild.
- **CI does not build the Lean image.** So CI does not catch the pins drifting again. LT-1's acceptance names this as not yet reached.

## Proof that the change is correct

- Each import in the old tables was compiled in both images. Each probe file is `import <module>` followed by `#eval 1`. The command is `docker run --rm --network none -v <dir>:/work <image> check /work/program.lean`, the same invocation `lean_check` makes through `runLean`. The results:
  - `Mathlib.Data.Nat.Defs` passes on v4.10.0 and fails on v4.28.0.
  - `Mathlib.Data.Nat.Basic` does the reverse.
  - Five imports fail on both images: `Mathlib.Analysis.SpecialFunctions.Basic`, `Mathlib.Data.Array.Basic`, `Mathlib.Data.HashMap.Basic`, `Mathlib.Data.Json` and `Mathlib.Tactic.DeriveDecidableEq`.
  - `Mathlib.Data.List.Basic`, `Mathlib.Data.Finset.Basic`, `Mathlib.Data.Multiset.Basic` and `Mathlib.Tactic.Linarith` pass on both.
- Each replacement was compiled in the v4.28.0 image, and all of them pass:
  - `Mathlib.Data.Real.Basic` (the import alone).
  - `Std.Data.HashMap`, with an insert and a lookup.
  - `Lean.Data.Json`, with a parse.
  - An inductive type with `deriving DecidableEq` and no import.
  - `Array.foldl` and `Array.mapIdx` with no import.
- The spec's LI-1 to LI-5 checks, and `npm test` and `npm run test:e2e`, are in the PR body.
