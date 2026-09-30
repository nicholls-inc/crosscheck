# Spec: Record the ruleset on the default branch

Intent: `intent/2026-09-30-record-ruleset.md`. Governing roadmap item: PB-1.

- **RS-1.** `CLAUDE.md`, `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/assurance/TIER-LAYER-MAP.md` and `docs/gates/tier-layer-gate.md` no longer say that the repository has no branch protection or that it is private.
- **RS-2.** Each of those files says that the default-branch ruleset requires no status checks, so CI cannot block a merge.
- **RS-3.** `CLAUDE.md`, `.claude/rules/protected-surfaces.md` and `docs/assurance/TIER-LAYER-MAP.md` say that the maintainer is the only person who can approve, cannot approve their own pull request, and so merges by bypassing the ruleset, and that this merge is the human sign-off.
- **RS-4.** `.claude/rules/protected-surfaces.md` lists what the ruleset enforces, notes that no CODEOWNERS file exists, and describes approval by someone other than the author as not yet reached, blocked by the absence of a second person with write access.
- **RS-5.** This supersedes the wording of TG-9 in `intent/2026-09-29-deterministic-evidence-spec.md`. On pass, the tier gate prints that the maintainer's merge is the human sign-off, that it bypasses the default-branch ruleset, and that the ruleset requires no status checks, so CI cannot block the merge. The line still starts with `Human sign-off: the maintainer's merge`.
- **RS-6.** No check, exit code, or protected path changes. `node --test scripts/ci/*.test.mjs` passes.
