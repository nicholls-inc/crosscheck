# Plan: Record the ruleset on the default branch

Intent: `intent/2026-09-30-record-ruleset.md`
Spec: `intent/2026-09-30-record-ruleset-spec.md` (RS-1 to RS-6)
Governing roadmap item: PB-1. Tier: 3.

This plan is not the root `plan.md`, because an open pull request (#52) rewrites that file. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/record-ruleset-2026-09-30.md`, naming the four protected files below.
2. Rewrite the sign-off statement in each file in the table (RS-1 to RS-4).
3. Change the TG-9 line in `scripts/ci/tier-gate.mjs` (RS-5).
4. Run `node --test scripts/ci/*.test.mjs`, then grep the live documents for "no branch protection" and "private".

## Files

| File | Protected | Change |
|---|---|---|
| `.claude/rules/protected-surfaces.md` | yes (`.claude/rules/**`) | amendment pattern, step 1 (RS-1, RS-3, RS-4) |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | opening section and stage 5 (RS-1, RS-2) |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | opening section and "Evidence and sign-off" (RS-1 to RS-3) |
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | TG-9 pass line (RS-5) |
| `CLAUDE.md` | no | "Protected surfaces" section (RS-1 to RS-3) |
| `docs/gates/tier-layer-gate.md` | no | "What approving means" (RS-1, RS-2) |
| `.assurance/protected-surface-amend/record-ruleset-2026-09-30.md` | no | new governance note |
| `intent/2026-09-30-record-ruleset*.md` | no | stage artefacts |

## Risks

- **The ruleset changes again.** The documents then drift in the same way. The facts are stated once in `.claude/rules/protected-surfaces.md` and summarised elsewhere, so a later change touches one full statement and five short ones.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes. The TG-9 test matches the unchanged prefix.
- `grep -rn -i "no branch protection\|is private" CLAUDE.md .claude/rules docs/assurance docs/gates scripts/ci` returns nothing.
- The ruleset facts come from `gh api repos/nicholls-inc/crosscheck/rulesets/24251306` and `gh repo view --json visibility`, both read on 2026-09-30.
