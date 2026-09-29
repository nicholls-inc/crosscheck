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

## Open questions
- Which roadmap item governs this move? No item in `docs/assurance/ROADMAP.md` covers it yet.
- Does CGV adopt the full development framework, or only the tier declaration?
- The tier gate accepts any `intent-check` attestation file. Under the vision, an LLM verdict cannot be evidence. How should Tier 3 change?
