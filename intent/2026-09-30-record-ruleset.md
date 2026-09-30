# Intent: Record the ruleset on the default branch

## Problem statement
Six live documents say the repository has no branch protection because it is private and has no GitHub Pro. Both parts are now false.

- The repository is public.
- Since 2026-09-30 the default branch has an active ruleset, `default` (id 24251306). It blocks deletion and force-push. It requires a pull request with one approving review, code-owner review, and resolved review threads. It dismisses stale approvals on push and allows only squash merges. Organisation admins and the repository admin role may bypass it for pull requests.
- The ruleset requires no status checks. CI still cannot block a merge.
- The maintainer is the only person who can approve, and GitHub does not let an author approve their own pull request. Every merge therefore bypasses the ruleset. The repository has no CODEOWNERS file, so the code-owner rule names no one.

The statements are in `CLAUDE.md`, `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md`, and the pass message of `scripts/ci/tier-gate.mjs` (TG-9). Agents and gates read these files, so the wrong statement shapes what they say about sign-off.

## Proposed outcome
- Each of the six places states the ruleset, that it requires no status checks, and that the maintainer's merge bypasses it and is the human sign-off.
- Approval by someone other than the author is described as not yet reached, with the property that blocks it: a second person with write access.

## Affected users and systems
- The maintainer, and every agent that reads `CLAUDE.md` or the framework documents.
- The tier gate's pass message. Its exit codes and checks do not change.

## Constraints
- Historical records keep their wording: earlier intents, specs, plans, merged governance notes, and the ADD archive record what was true when they were written.
- No gate changes behaviour. Only the TG-9 message text changes, and its test still matches.

## Open questions
None. The maintainer directed this change on 2026-09-30.
