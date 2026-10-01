# Intent: The tier gate accepts any citation line, and only a regular file in the repository

Task: PB-1.5. Governing roadmap item: PB-1. Issue: #49.

## Problem statement
The tier gate, `scripts/ci/tier-gate.mjs`, checks the `Intent:`, `Spec:` and `Plan:` citations in a PR body with `citedExisting`. It has three faults left over from #46.

1. Only the first citation line counts. A body with `Plan: TBD` followed by `Plan: intent/p.md` fails, although it cites a plan that exists. This can wrongly block a pull request, but never lets one through.
2. The existence check is too loose. `existsSync(join(cwd, path))` accepts a directory (`Plan: intent`), a path that leaves the repository (`Plan: ../x`), and a symlink in the repository that points at a file outside it. None of these is an artefact of the repository.
3. The citation tests are positive only. They have no `+ ` marker case, and no case that shows a near miss is not a citation: `-Plan:`, `>> Plan:`, `1. Plan:`, `- [ ] Plan:`.

Run against `main` at a424468, the gate fails `Plan: TBD` then `Plan: intent/p.md`, and passes `Plan: intent`, `Plan: ../outside.md` and a symlink to a file outside the repository.

## Proposed outcome
- The requirement is met when any citation line for the keyword cites a valid file. The order of the lines does not matter.
- A cited path is valid only when it names a regular file, and its real path, after symlinks, lies inside the repository.
- The tests cover the `+ ` marker, the near misses above, several citation lines, directories, paths outside the repository, and symlinks.
- `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md` and the TG spec state the rule.

## Affected users and systems
- Everyone who opens a pull request here: the gate reads the citations in their PR body.
- `scripts/ci/tier-gate.mjs` and `scripts/ci/tier-gate.test.mjs`.
- `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md`, and `intent/2026-09-29-deterministic-evidence-spec.md`.
- `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- The citation line format does not change: the keyword starts the line, after an optional indent and at most one list or quote marker.
- The `Tier:` declaration (TG-1), the protected floor, governance notes (TG-5), TG-6 to TG-9 and the evidence report do not change.
- No workflow changes. Workflow fixes are task PB-1.7.
- No dependency is added.

## Open questions
None. Issue #49 states the outcome.
