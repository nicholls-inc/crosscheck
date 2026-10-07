# Crosscheck Development Framework

Governing roadmap item: **PB-1** (`docs/assurance/ROADMAP.md`).

This repository develops both of its tools, Crosscheck and CGV (`cgv/`), with
the artefact chain from the AI-native SDLC playbook. Every stage commits an
artefact that the next stage reads. Together, the intent, the spec, the plan,
the diff, the tests and the review findings are the audit trail. No stage
depends on conversation context that has since scrolled away.

Three kinds of control appear below:
- **Skills are advisory.** They make the right behaviour likely.
- **Hooks and CI are deterministic.** They make the right behaviour checked
  every time.
- **Human approval is concentrated at named gates.** Each gate has an
  explainer under `docs/gates/`.

Only the last two count as evidence (`docs/VISION.md`). No CI job calls an LLM.
The ruleset on the default branch requires no status checks, so CI checks cannot
block a merge. The maintainer's merge, which bypasses the ruleset's approval
rule, is the human sign-off, and the maintainer does not merge while a check is
red.

## The chain

| Stage | Artefact committed | Where | Triggered by |
|---|---|---|---|
| 1. Plan | `intent/<slug>.md` | repo-root `intent/` | a person deciding to make a change |
| 2. Design | `spec.md` | beside the intent, or `crosscheck/docs/` for module specs | accepted intent committed |
| 3. Build | `plan.md`, then diff + tests | `plan.md` at repo root | accepted spec committed |
| 4. Test | verification logs, attestations | `.assurance/` | diff pushed |
| 5. Deploy | PR body with tier, review findings, approval records | GitHub PR | branch pushed |
| 6. Maintain | incident record + eval + candidate invariant | `evals/`, `docs/invariants/` or `crosscheck/docs/invariants/` | production or dogfood failure |

Stage 6 feeds a fresh `intent/<slug>.md` back into stage 1. That loop is the
framework; each production incident gets an eval, and the eval stays in the
suite as a regression test.

### 1. Plan — `intent/<slug>.md`

Fields: problem statement, proposed outcome, affected users and systems,
constraints, open questions. `/informal-spec` is the skill that extracts this
precisely; it ends at a hard human sign-off gate
(`docs/gates/informal-spec-sign-off.md`) which writes
`Human sign-off: YYYY-MM-DD` into the file. No sign-off, no stage 2.

### 2. Design — `spec.md`

Generated from the accepted intent. A spec must **flag** concerns rather than
silently resolve them. Invariants are drafted from the spec by
`/draft-invariants`, whose red-pen gate
(`docs/gates/draft-invariants-red-pen.md`) lets a human strike, reword or add
invariants *before* any test is generated. Adequacy of the spec is probed by
`/audit-spec-coverage` (spec section → invariant coverage matrices),
`/audit-invariant-consistency` (contradictions within, across, and against the
spec) and `/spec-adversary` (up to three missing invariants). Each emits capped,
prioritised findings with a four-path triage block a human resolves.

For repos onboarding from scratch, `/assurance-init` scaffolds
`docs/assurance/`, the horizon directories and `.claude/rules/`.

### 3. Build — `plan.md`, diff, tests

The bar for `plan.md`: *an engineer who has never seen the conversation could
implement the change from the plan alone.* It lists the files that change, the
order of work, the risks, and the proof or tests that will show it worked.
`lowry` drives the red-to-green loop against the ratified invariant contract; it
refuses to reach green by weakening an invariant and instead stops and emits a
drift packet (`docs/gates/lowry-drift-packet.md`).

Edits to protected surfaces (`SKILL.md`, `agents/*.md`, invariant docs,
`docs/assurance/**`, `.claude/rules/**`, `.claude/hooks/**`, `evals/**`) require
a governance note from `/protected-surface-amend`. That skill refuses
unconditionally without a governing roadmap item
(`docs/gates/protected-surface-roadmap-refusal.md`), and the PreToolUse hook in
`.claude/hooks/` blocks the write unless the note is new on this branch, not
already on the default branch (`docs/gates/protected-surface-hook.md`).

### 4. Test — verification and alignment

Evidence at this stage comes from deterministic CI jobs:
- **Crosscheck:** `ci.yml` runs `npm test`, which includes the property tests
  that cover the invariant docs.
- **CGV:** `cgv-ci.yml` runs `cargo test`, `lake build` (the soundness proofs
  and the `#guard` tests), the fixture comparison, and the statement-manifest
  check. That check fails when a protected theorem statement, or a definition
  reachable from `constraintImplies`, `IsDataPath` or `stepwiseSound`, changes
  without `cgv/prover/protected-statements.txt` changing too. It also fails when
  a protected theorem or definition depends on `sorry` or on a non-standard
  axiom. A kernel replay (`leanchecker`) then fails when a declaration of the
  built environment does not re-check, such as one added with the kernel check
  switched off. It does not cover everything: CI-9 in
  `intent/2026-09-29-deterministic-evidence-spec.md` lists the four limits
  (`unsafe` and `partial` constants, the kernel that built the files, the
  `ContractGraphTest` library, and `implemented_by` and `extern`).
- **The tier gate:** `tier-gate.yml` runs the gate's own tests.

`/intent-check`, `/audit-spec-coverage`, `/audit-invariant-consistency` and
`/spec-adversary` are advisory. They use an LLM, so they run locally or under an
orchestrator, never in CI, and no gate reads their output.
- `/intent-check` runs the round-trip triple (invariant prose, covering test,
  code diff) and appends to the false-positive tracker.
- It refuses when the rolling false-positive rate reaches the kill threshold
  (`docs/gates/intent-check-kill-criterion.md`).
- A failing verdict is a prompt to look harder: fix the code, fix the test, or
  amend the invariant (`docs/gates/intent-check-verdict.md`).

`/assurance-probe` measures test strength on rotation, not per PR.

### 5. Deploy — the PR

The PR body declares `Tier: N` (see `docs/assurance/TIER-LAYER-MAP.md`) and
carries the review findings produced against `REVIEW.md`: separate passes for
bugs and logic, security, and compliance with the spec and plan; findings marked
Important or Nit; at most five nits reported and the rest summarised as a count;
generated paths excluded. These CI workflows report on the merge. None of them
calls an LLM, and none can block a merge, because the ruleset on the default
branch requires no status checks:

- `tier-gate.yml` fails in any of these cases (`docs/gates/tier-layer-gate.md`):
  - the declared tier lacks its required artefacts;
  - a changed protected path is not named in a governance note that the PR
    adds or changes;
  - a CGV proof surface changes without a `## Protected-surface change`
    section in the PR body.

  Any protected path forces a Tier 3 floor. On a pass, the gate lists which job
  holds the evidence for each changed file.
- `ci.yml` (Crosscheck) and `cgv-ci.yml` (CGV) are the tests, builds and proofs
  described in stage 4.
- `incident-eval-check.yml` runs only after a pull request is merged, so it
  reports on the merge and cannot block it. On every merged pull request it
  first reads the pull request's commits, and exits 2 if it cannot, for
  example after a merge commit, which puts them on the base branch where the
  check cannot tell them apart from the base branch's own. A pull request with
  no incident reference can therefore still exit 2 (IE-2 in
  `intent/2026-09-30-incident-eval-range-spec.md`).

  The check then applies when the pull request has the `incident` label, or
  an incident line in its body or in one of its commit messages (IE-5). An
  incident line holds only `Fixes-Incident:`, in any case, and one id,
  optionally indented (IE-9 in `intent/2026-10-06-incident-line-spec.md`). The
  trigger in the middle of a line, after a list or quote marker, or followed by
  more than one word is not an incident line, so prose that quotes it does not
  fire the check. Write one incident line per incident. It exits 1 in either
  of these cases:
  - no eval under `evals/` contains the incident id in its path or content, or
    no candidate invariant under `docs/invariants/` or
    `crosscheck/docs/invariants/` contains it in its content. Both tests are
    plain substring matches, so a file naming `INC-12` also matches `INC-1`;
  - the label is set and no id is found.

  Its explainer is `docs/gates/incident-eval-check.md`.
- `task-queue.yml` fails when a row of `docs/TASKS.md` names no roadmap item,
  repeats a task ID, has a status other than `todo`, `blocked` or `done`, or
  depends on a task that is not in the queue. It also fails when the pull
  request sets to `done` any row other than the one its `Task:` line names
  (`docs/gates/task-queue-check.md`).
- `semantic-pr.yml` checks that the PR title is a conventional commit.

Two of these checks also run before the commit, as the roadmap's dual-track
principle asks. `npm install` at the repository root installs
`.husky/pre-commit`, which runs `node scripts/ci/pre-commit.mjs` on the
commit as staged. It runs only the rules that need no PR body:
- when the commit stages a protected path, every protected path that the
  branch changes must be named in a governance note that the branch changes
  (the tier gate's governance-note rule);
- when the commit stages `docs/TASKS.md` or `docs/assurance/ROADMAP.md`, the
  queue must pass the task queue check's rules, except the `Task:` line rule.

Each failure prints the command that fixes it. The hook never fetches and
reads `origin/main` as last fetched. `git commit --no-verify` skips it, so CI
stays the check that every pull request passes through
(PC-1 to PC-8 in `intent/2026-10-06-pre-commit-hooks-spec.md`).

### 6. Maintain — incidents and evals

The pull request that fixes an incident adds an incident record and its eval
under `evals/`, and a candidate invariant under `docs/invariants/` or
`crosscheck/docs/invariants/`, each naming the incident id. After that pull
request merges, the Incident Eval Check (stage 5) looks for the eval and the
invariant (`docs/gates/incident-eval-check.md`). `auditor` runs read-only
consolidation passes and renders settled / active / drifted per artefact for
human adjudication (`docs/gates/auditor-verdicts.md`); it never edits what it
audits.

## Pick up the next task

`docs/TASKS.md` is the ordered queue of tasks. `docs/assurance/ROADMAP.md` holds
the items that govern them. When the maintainer says "pick up next task", do
these steps in order.

1. **Choose.** Run `git fetch origin`, then read the queue with
   `git show origin/main:docs/TASKS.md`. List the claimed tasks with
   `git ls-remote --heads origin 'task/*'`. The next task is the first row that
   meets all three conditions:
   - its status is `todo`;
   - every task it depends on is `done`;
   - no branch named `task/<task ID>` exists.

   After `git fetch origin`, `node scripts/ci/task-queue.mjs next` applies
   these three conditions and prints the ID of the next task. When no row meets
   them, it exits 1 and prints each row that is not ready, with the reason.
2. **Claim.** Create a branch named exactly `task/<task ID>` from `origin/main`,
   give it an empty commit that no other agent can produce, and push it only if
   the branch does not exist yet:

   ```bash
   nonce=$(uuidgen) && [ -n "$nonce" ] || { echo "no nonce: stop"; exit 1; }
   git switch -c task/<task ID> --no-track origin/main
   git commit --allow-empty -m "chore: claim task/<task ID> ($nonce)"
   git push origin --force-with-lease=refs/heads/task/<task ID>: \
     HEAD:refs/heads/task/<task ID>
   git ls-remote origin refs/heads/task/<task ID>   # must print HEAD's SHA
   ```

   The lease with nothing after the colon makes the push fail if the branch
   exists, and the unique commit means two agents never push the same SHA. Both
   are needed: a second push of a branch cut from `origin/main` with no commit
   of its own prints "Everything up-to-date" and succeeds. If `uuidgen` prints
   nothing, stop: without the nonce, two agents with one identity can make the
   same commit in the same second. If the push fails, or `ls-remote` prints a
   SHA that is not your `HEAD`, another agent holds the task. Run
   `git switch --detach origin/main && git branch -D task/<task ID>` and return
   to step 1. `--no-track` keeps the branch from tracking `main`, so push later
   commits with `git push origin task/<task ID>`.
3. **Read.** Read `docs/VISION.md`, the roadmap item that the task ID names, and
   the linked issue. Then read the journals that `AGENTS.md` tells you to read.
4. **Run the chain.** Start at stage 1 above with `intent/<yyyy-mm-dd>-<slug>.md`.
   Find the tier in `TIER-LAYER-MAP.md` and commit the artefacts that the tier
   requires before the diff.
5. **Record.** In the same pull request, set the row of the task to `done` and
   put the path of the intent in its record. Add a `Task: <task ID>` line to the
   pull request body.

Stop and report to the maintainer in each of these cases. Do not work around
them.

- The intent has an open question.
- No row meets the conditions in step 1. Report which rows are `blocked` or
  claimed, and what unblocks each.
- The task needs a change to a protected surface that no roadmap item covers.

If you stop after step 2, release the claim with
`git push origin --delete task/<task ID>` and name the task in your report.
Otherwise the task stays claimed with nobody working on it. A claim branch that
nobody is working on is deleted by the maintainer.

Two rules keep the queue true.

- **New work becomes a row.** If you find work that the task does not cover, add
  a `todo` row under the roadmap item it belongs to, or open an issue. Do not do
  that work in the current pull request.
- **Only the maintainer changes the status of a roadmap item.** When the last
  task of an item is done, say so in the pull request body. The maintainer
  decides whether the item meets its acceptance.

The maintainer's merge makes the new status true on `main`. Until the merge, the
branch is the only sign that the task is in progress.

## Which agent runs which stretch

- `add-orchestrator` — spec → bulk-drafted invariants → batched audit → triaged
  findings → approved invariants (stages 1–2), with batched sign-offs.
- `byfuglien` — the implementation and verification chain (stages 3–4).
- `hellebuyck` — specification-chain assurance and governance scaffolding.
- `lowry` — the gated run-to-green loop (stage 3).
- `auditor` — stage 6 consolidation, read-only.

If you are unsure at any gate, ask the Crosscheck maintainers via a GitHub issue
on this repository.
