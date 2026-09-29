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

## Resolved questions
- **Which roadmap item governs this move?** MR-1 in `docs/assurance/ROADMAP.md`. It was added with the history import and names this intent. Follow-up work that changes the tier map or the tier gate is governed by PB-1, which owns both. The CGV CI work below needs a new roadmap item.

## Open questions
- **Does CGV adopt the full development framework, or only the tier declaration?**
  Proposed answer: CGV adopts the full chain, with CGV-native evidence at each tier. Reasoning:
  - "Tier declaration only" does not hold up. The tier gate runs on every pull request and checks each tier's artefacts, so a declaration without artefacts fails the gate. Making the gate skip `cgv/` would leave CGV's protected surfaces with no deterministic check at all.
  - The full framework as written does not fit CGV:
    - A Tier 2 prose `spec.md` duplicates the Lean theorem statements, which are the stronger spec, and the two can drift apart.
    - `/intent-check` round-trips invariant prose against TypeScript or Go property tests. It cannot read a Lean theorem.
    - `/invariant-coverage-scaffold` does not support Rust yet.
    - The soundness theorem statements are protected by rule, but they are not in the machine-readable path list, so no hook or Tier 3 floor catches a change to them.
  - CGV has no CI. `ci.yml` runs only on `crosscheck/**`, so `cargo test`, `lake build`, and `scripts/check-fixtures.sh` never run on a pull request. Adopting the framework without CI would add paperwork without adding any deterministic evidence.
  - Proposed shape:
    1. Every CGV change has an intent. This costs little and records the design.
    2. At Tier 2, the spec may be the changed Lean statements plus the changed `test_fixtures/*/expected.json` files, cited by a `Spec:` line in the pull request.
    3. At Tier 3, the evidence is a passing `lake build` and a statement manifest check. The manifest check fails whenever a protected theorem statement changes. That change then forces Tier 3 and the **Protected-surface change** section.
    4. The first step is a new CI workflow that runs `cargo test`, `lake build ContractGraph ContractGraph.Main`, and `scripts/check-fixtures.sh` on `cgv/**`.
- **The tier gate accepts any `intent-check` attestation file. Under the vision, an LLM verdict cannot be evidence. How should Tier 3 change?**
  Proposed answer, for discussion. The current gate has three separate defects:
  - *Stale artefacts pass.* `tier-gate.mjs` searches the whole tree for the file name. The committed `crosscheck/mcp-server/.assurance/intent-check-attestation.json` (from #134, April 2026) therefore satisfies every Tier 3 pull request. The same happens with the root `plan.md` (from #246). Neither the verdict nor the content hash is read.
  - *The attestation is LLM judgement.* Its `verdict` comes from a back-translator and a diff-checker, with a confidence figure of 91% in the committed file. Even a fresh attestation with a checked hash is still LLM judgement.
  - *There is no human approval check.* The gate never confirms that a named human approved the pull request.

  Proposal:
  1. Remove the attestation from Tier 3's required artefacts. `/intent-check` keeps running as an advisory signal: it posts its result as a pull request comment and feeds the false-positive tracker. The gate never reads it.
  2. Replace it with deterministic evidence for each class of protected surface, run in CI on the pull request's head commit:
     - For invariant docs and property tests, the invariant coverage check and the covering tests pass.
     - For Dafny and Lean specs, `dafny verify` or `lake build` passes.
     - For CGV theorem statements, the statement manifest check passes.
     - For `SKILL.md`, agent files, rules, hooks, and CI, no deterministic behaviour check exists yet. The gate says "not yet reached: human review is the only evidence" and does not pretend otherwise.
  3. Require human approval deterministically. A CODEOWNERS entry for the protected paths, plus branch protection that requires a code-owner review, makes a named human's approval a merge condition. That approval is the human judgement the vision counts.
  4. Bind artefacts to the pull request. `plan.md` and governance notes count only if the pull request adds or changes them, and the gate reads their content rather than only checking that they exist.

  This changes `docs/assurance/TIER-LAYER-MAP.md`, `scripts/ci/tier-gate.mjs`, and the tier-gate workflow. All three are Class A protected surfaces, so the change goes through `/protected-surface-amend` under PB-1.
