---
description: Pick up the next task, open a PR, and review-triage it until pr-swarm approves HEAD and CI is green
argument-hint: "[pr=<n>] [decisions=\"<text>\"] [dry-run] [passes=<n>]"
---

# Ship next task

Arguments: $ARGUMENTS

You coordinate. Every phase runs in an `Agent` call with `subagent_type: "general-purpose"` and the model named below, and you wait for it to finish before the next phase. Those subagents can dispatch their own subagents, which poteto-mode and pr-swarm need for their model tiers. Subagents started by a Workflow script cannot, so do not turn this into one. Do the gate yourself. Never merge, approve a PR, arm auto-merge yourself, or force-push.

Parse the arguments. `pr=<n>` resumes at Review. `decisions="..."` needs `pr`. `dry-run` needs `pr` and adds `--dry-run` to review-triage. `passes=<n>` caps review passes (default 3). Set `pass = 1`.

Every subagent prompt ends with: "Local runs: gh and git need `dangerouslyDisableSandbox: true` on the Bash call. There is no human to ask."

## 1. Build (opus, skipped when `pr` is given)

Prompt: "Invoke the poteto-mode skill with args `pick up next task`. Follow the pick-up procedure this repository's CLAUDE.md names, in a worktree, and run the matched playbook through its Opening a PR step. Stop once the PR is open. Do not run review-triage, pr-swarm, babysit, or any PR-status loop. A product or preference call no experiment can settle ends the run as `blocked`. No task ready is `no_task`. Return JSON `{status: pr_opened|no_task|blocked, task_id, pr_number, pr_url, summary, blocked_reason}`."

`status` other than `pr_opened` ends the run with that outcome.

## 2. Decide (opus, only with `decisions`)

Prompt: "Invoke the poteto-mode skill. On PR #<pr>, apply these maintainer decisions to the open review threads they answer: <decisions>. Work in a worktree on the PR's head branch. Update the spec and docs each decision changes, run the repo's tests, commit, push to that branch, then reply on and resolve each thread a decision answers. Return JSON `{head_sha, summary}`."

## 3. Review (sonnet)

Prompt: "Invoke the review-triage skill with args `<pr> --unattended` (plus ` --dry-run` when set) and follow it to its structured result. Work in a worktree on the PR's head branch. From its final JSON, return `{rounds_used, loop_exhausted, new_head_sha, verdict, deferred: [{id, location, gate, reason}], commits, invariant_violations, degradations, summary}`, copying degradations verbatim."

## 4. Gate (you)

Run `sh .claude/scripts/pr-gate.sh <pr>` with `dangerouslyDisableSandbox: true`. It waits up to 15 minutes for pending checks and prints JSON. `done` is true only when the PR is open, pr-swarm's latest verdict was given for the current HEAD and is `APPROVE` or `APPROVE_WITH_NITS`, and at least one check ran with none pending or failed. Take that value as given. Do not re-derive it.

## 5. Next step, first matching row wins

| Condition | Action |
| --- | --- |
| gate `state` is not `OPEN` | Stop. `closed` |
| review `invariant_violations` is non-empty | Stop. `invariant_violation` |
| a review degradation says the Agent tool was missing or not available, or the review ran router-only or inline | Stop. `degraded_review`. The verdict came from one model and does not count |
| a `deferred` entry has `gate` `ladder` or `error` | Stop. `needs_human` |
| dry run | Stop. `dry_run_complete` |
| gate `checks_failed` is non-empty | Fix CI (sonnet): "Invoke the poteto-mode skill and run its Bug fix playbook on PR #<pr>'s failing checks: <names>. Reproduce each locally first, fix the root cause, run the repo's tests, commit, and push to the PR's head branch in a worktree. Return JSON `{head_sha, summary}`." Then `pass += 1` and go to 3 |
| gate `done` | Stop. `ready_for_human_review` |
| `pass` equals the cap | Stop. `exhausted` |
| otherwise | `pass += 1`, go to 3 |

## 6. Report

Say the outcome and the PR link. Then:
- `ready_for_human_review`: the verdict and HEAD SHA, and any `gate: human` threads the maintainer should read before merging.
- `needs_human`: each `ladder` or `error` deferral as a plain question with its options, and the command to answer it, `/ship-next-task pr=<n> decisions="..."`.
- Any other stop: what stopped it and the gate JSON.

An approving verdict from pr-swarm is an LLM's judgement. It makes the PR ready for the maintainer's review. It is not evidence that the change is correct.
