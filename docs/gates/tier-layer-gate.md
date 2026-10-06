# Gate: Tier-Layer Gate (CI Failure)

## What this gate protects

Not every change carries the same risk. A typo fix and a change to the logic that decides whether other changes are safe are not the same kind of event, and they should not require the same paperwork. Crosscheck sorts every pull request into one of three **tiers** — routine, standard, or critical — and each tier requires a different **artefact** (a committed document proving the right groundwork was done) before a human reviewer is asked to approve anything. The full definition lives in `docs/assurance/TIER-LAYER-MAP.md`; this gate is what enforces it.

The **tier-gate CI job** (`scripts/ci/tier-gate.mjs`) runs on every pull request and checks two things: that a tier was declared, and that the artefact its tier requires is actually present. It runs deterministically, in CI — the same way for every PR, with no judgement calls — and it runs **before the review gate opens**. A failing tier gate means review has not started yet, not that a reviewer raised a concern.

## How to declare a tier

A pull request declares its tier in one of two ways: a `Tier: N` line in the PR body (e.g. `Tier: 2`), or a `tier:N` label on the PR. The `Tier:` line must start the line, optionally after an indent. A line that starts with a list or quote marker (`- `, `* `, `+ `, `> `) declares nothing, and so does `Tier:` in the middle of a sentence, so quoted or list-item text cannot set the tier. The first `Tier:` line is the declaration and must read exactly `Tier: 1`, `Tier: 2` or `Tier: 3`. If it says anything else, such as `Tier: 4` or `Tier: 1 (routine)`, the gate fails, even when a later line is valid. A `Tier:` line and a `tier:N` label must agree, and so must two labels; if they disagree, the gate fails. If neither is present, the gate fails immediately and asks you to add one.

## Which tier applies

- **Tier 1 — routine.** Documentation, tests, and non-behavioural code (formatting, renames, build plumbing that changes no output). Requires a reference to the governing `intent.md` — the "why we're doing this" document from the Plan stage — cited by path in the PR body or added under `intent/`.
- **Tier 2 — standard.** Behavioural code changes: anything that alters what the software does for a user or caller. It requires a spec, which is a design document generated from the intent that flags open concerns instead of silently resolving them. Either of these satisfies the requirement:
  - the pull request changes a root `spec.md`;
  - the PR body has a `Spec: <path>` line citing a file in the repository. For CGV, this can be the changed Lean file or fixture `expected.json`.
- **Tier 3 — critical/protected.** Protected surfaces (see below), gate logic, hooks, CI enforcement, and invariants: anything that changes how the project decides whether other changes are safe. It requires a plan, detailed enough that someone who never saw the discussion could implement the change from it alone. Either of these satisfies the requirement:
  - the pull request changes a root `plan.md`;
  - the PR body has a `Plan: <path>` line citing a file in the repository.

  For every protected file in the diff, it also requires a **governance-note block** naming that file. The block must be in a note that this pull request adds or changes. For a CGV proof surface, the PR body also needs a `## Protected-surface change` section.

**A citation names a file in the repository.** An `Intent:`, `Spec:` or `Plan:` line starts the line, optionally after an indent and one list or quote marker. Its path must resolve, through any symlinks, to a regular file inside the repository, so a directory, `../x`, or a symlink that points outside the repository does not count. If the PR body has several lines for one keyword, one valid line is enough.

**Artefacts left over from earlier changes do not count.** A root `plan.md` or `spec.md`, or a governance note, counts only if this pull request changes it, or, for a plan or spec, if the PR body cites it by path. Otherwise the file left behind by the last change would satisfy every later one.

**No LLM verdict is required or read.** Tier 3 used to require an `intent-check` attestation. That record is an LLM's judgement, and this repository counts only deterministic checks and human judgement as evidence. You can still run `/intent-check` locally, as a second opinion.

## Why a protected path forces Tier 3

`.claude/rules/protected-surfaces.md` lists paths — skill definitions, agent prompts, invariants, hooks, CI enforcement scripts, and the rules file itself — that define how Crosscheck decides what is correct. If a diff touches any of these, the gate imposes a **floor of Tier 3** regardless of what the PR declares. Declaring Tier 1 or Tier 2 on such a diff is not a judgement call the gate defers to you on — it is a straightforward failure, because a change to how safety is decided cannot be waved through as routine. You may always declare a tier *above* the floor; you may never declare one below it.

## Incidents are checked after the merge, not by this gate

The tier gate reads no incident reference. A pull request that fixes an incident passes or fails this gate on its tier and artefacts alone, whether or not it adds an eval.

A separate workflow, `incident-eval-check.yml` (the Incident Eval Check), checks incidents. It applies when a pull request has the `incident` label, or a line in its body or in one of its commit messages that holds only `Fixes-Incident:` and one id, optionally indented. The trigger in the middle of a line, after a list or quote marker, or followed by more than one word does not count, so a description that quotes it does not fire the check. It then needs two files that contain the incident id:

- an **eval** under `evals/` that reproduces the incident and stays in the suite, so the same failure cannot recur unnoticed;
- a candidate invariant under `docs/invariants/` or `crosscheck/docs/invariants/`.

The check runs only after the pull request is merged. It reports on the merge and cannot block it. No check asks for the eval or the invariant before the merge, so add both in the pull request that fixes the incident. Stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md` lists its exit codes.

## What each decision means

- **Approving (declaring the correct tier and supplying its artefacts)**: the pull request now carries an auditable record matching its actual risk level, and the tier gate passes, opening the review gate.
- **Declining (leaving the tier or artefacts as they are)**: the change stays blocked. Nothing is merged and no reviewer is asked to look at it until the tier and artefacts are corrected.

The ruleset on the default branch requires no status checks, so GitHub does not stop a merge while this check is red. "Blocked" is the maintainer's rule rather than a setting: the maintainer does not merge a pull request whose tier gate is red. When the gate passes, it also lists which CI job holds the deterministic evidence for each changed file. A file that no job checks, prose included, is listed as "not yet reached", with the property that blocks a check and the open question. The list is in `docs/assurance/TIER-LAYER-MAP.md`.

## How long this takes

For Tier 1 and Tier 2, usually a few minutes — citing or writing a short intent or spec document. For Tier 3, expect tens of minutes, because a genuine plan and, for protected files, a governance-note block from `/protected-surface-amend` both take real drafting time.

## Who to ask if unsure

The Crosscheck maintainers, via a GitHub issue on this repository.
