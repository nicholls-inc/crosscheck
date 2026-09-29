# Intent: Move Crosscheck into one repository with CGV

## Problem statement
Crosscheck and the contract graph verifier (CGV) now share one vision, written in `docs/VISION.md`. They live in two repositories. Crosscheck is one directory of `nicholls-inc/claude-code-marketplace`, next to unrelated plugins, and CGV is `nicholls-inc/contract-graph-verifier`. A change that spans both tools, such as a shared evidence-record format, would need two coordinated pull requests. It would also need two sets of governance rules and two Claude projects.

## Proposed outcome
One repository holds Crosscheck, CGV, the shared vision, and Crosscheck's development framework. Crosscheck keeps its history from 2026-03-06, when it was renamed from `formal-verify`, onward. The marketplace installs the plugin from this repository with a `git-subdir` source, so plugin users download only `crosscheck/`. The repository is renamed to `nicholls-inc/crosscheck`.

## Affected users and systems
- Users of the `crosscheck@nicholls` plugin. The install command stays the same. The plugin source moves.
- Crosscheck's CI, release-please versioning (`crosscheck-v*` tags, now 2.7.0), and protected-surface gates. They now run in this repository.
- CGV contributors. Every CGV pull request now falls under the tier gate and the development framework.
- The marketplace repository, which loses `crosscheck/` and the governance files that only Crosscheck used.

## Constraints
- No skill, agent, MCP tool, invariant, or threshold changes behaviour.
- Paths inside `crosscheck/` keep working. Crosscheck-owned root paths keep the layout they had in the marketplace, so the 16 skill links to `docs/gates/` still resolve.
- The history import must be merged with a merge commit. A squash merge discards the history.
- `.claude/hooks/` and `.claude/settings.json` are not imported by an agent. The maintainer adds them.


## Decisions (maintainer, 2026-09-29)
- **Which roadmap item governs this move?** MR-1 in `docs/assurance/ROADMAP.md`. It was added with the history import and names this intent. The follow-up work below changes the tier map, the tier gate, and CI, which PB-1 owns, so PB-1 governs it.
- **Does CGV adopt the full development framework, or only the tier declaration?** The full framework. The repository applies its own system to itself. Why "tier declaration only" does not hold up:
  - The tier gate runs on every pull request and checks each tier's artefacts, so a declaration without artefacts fails the gate.
  - Making the gate skip `cgv/` would leave CGV's protected surfaces with no deterministic check.

  CGV's evidence differs from Crosscheck's at each tier:
  1. Every CGV change has an intent.
  2. At Tier 2, the spec may be the changed Lean statements plus the changed `test_fixtures/*/expected.json` files, cited by a `Spec:` line.
  3. At Tier 3, the evidence is a passing `lake build` plus a statement manifest check. The manifest check fails when a protected theorem statement, or a definition it relies on, changes without the committed manifest changing too. The manifest is a protected path, so changing it forces Tier 3 and the **Protected-surface change** section.
  4. CGV gets CI: `cargo test`, `lake build`, `scripts/check-fixtures.sh`, and the manifest check, run on `cgv/**`.
- **How should Tier 3 change?** The gate stops reading the `intent-check` attestation. It had three defects:
  - It accepted any attestation file in the tree, including the committed one from #134. The root `plan.md` from #246 passed the same way.
  - It never read the attestation's verdict or content hash.
  - The verdict is LLM judgement in any case.

  The new Tier 3:
  1. The gate does not read the attestation. `/intent-check` stays available as a local advisory tool.
  2. Deterministic evidence comes from CI jobs for each class of protected surface: the Crosscheck tests, CGV's `lake build` and manifest check, and the tier gate's own tests. For `SKILL.md`, agent files, rules, hooks, and CI definitions, no deterministic behaviour check exists yet. The gate reports this as "not yet reached", and human review is the only evidence.
  3. A plan or governance note found by searching the tree counts only if this pull request adds or changes it. A file cited by a `Plan:` or `Spec:` line still counts, because the author names it and the reviewer can read it.
  4. For a CGV proof surface, the pull request body must contain a `## Protected-surface change` section.

## Constraints on the follow-up (maintainer, 2026-09-29)
- **No LLM runs in CI.** There is no budget for it. `spec-audit.yml` and the `intent-check` job in `protected-surface-check.yml` are removed. The audit skills and `/intent-check` run locally, as advisory tools.
- **No branch protection.** The repository is private, and the account has no GitHub Pro. No CI check can therefore block a merge. The maintainer's merge is the human sign-off. Enforcing that sign-off in CI is not yet reached: the blocking property is a merge condition that GitHub enforces, which needs branch protection. The CI checks inform that merge. They do not replace it.
