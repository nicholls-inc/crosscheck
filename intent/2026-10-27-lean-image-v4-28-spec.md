# Spec: Lean image pins Lean v4.28.0 and Mathlib v4.28.0

Intent: `intent/2026-10-27-lean-image-v4-28.md`. Tier 2.

- **LI-1. The pins.** `crosscheck/mcp-server/lean-harness/lean-toolchain` holds `leanprover/lean4:v4.28.0`, and the `mathlib` requirement in `crosscheck/mcp-server/lean-harness/lakefile.lean` is the git tag `v4.28.0`. Both name the same release, so `lake update` selects the Mathlib that matches the compiler.
- **LI-2. The image.** `crosscheck/scripts/build-lean-docker.sh` builds an image in which `lake build Crosscheck` on the placeholder succeeds with Mathlib oleans resident. The placeholder (`Crosscheck/Program.lean`) and the script's timing snippet import `Mathlib.Data.Nat.Basic`, because `Mathlib.Data.Nat.Defs` does not exist at v4.28.0. The script honours `LEAN_DOCKER_IMAGE`, so a build can use a tag other than `latest`.
- **LI-3. What the image accepts.** A file with no imports that uses `List.Pairwise` passes `lean_check` with `success: true`. A file that uses it with `import Mathlib.Data.List.Basic` does too. A file with a type error still fails with `typecheck-error`.
- **LI-4. Existing models.** `formal-verification/lean/CrosscheckModel/Power.lean` passes `lean_check`, or each failure is listed in the PR with its cause.
- **LI-5. Limits.** A check of a file with one Mathlib import finishes in under 240 s and inside 2 GB. The measured times and the old image's times are in the PR.

**Flagged, not resolved.**
- A v4.28.0 Lake may refuse the old `lakefile.lean` syntax, the Mathlib olean cache may not fit the 240 s timeout on a cold `lake build`, and models written for v4.10.0 may break. Each is a guess until measured; the PR records the result.
- `crosscheck/skills/lean-spec/SKILL.md` line 88 still names `Mathlib.Data.Nat.Defs`. It is protected, and no roadmap item covers it, so this change leaves it. Until it is amended, a model that follows the table fails with an unknown module and the retry loop of `/lean-spec` has to find `Mathlib.Data.Nat.Basic` itself.
- No workflow builds the Lean image, so CI does not catch pin drift: not yet reached. The property that blocks it is the build time and disk of Mathlib oleans on a hosted runner, and the open question is whether a scheduled job is worth that.
