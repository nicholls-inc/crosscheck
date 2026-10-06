# Gate: Incident Eval Check (CI Failure After a Merge)

## What this gate protects

An **incident** is a failure of this repository's tools seen in production or in dogfooding, such as a gate that passed a change it should have failed. Every incident should leave two things behind, so that the same failure cannot recur unnoticed:

- an **eval** under `evals/`: a written scenario, or an executable one, that reproduces the incident and stays in the suite as a regression test (see `evals/README.md`);
- a **candidate invariant** under `docs/invariants/` or `crosscheck/docs/invariants/`: a stated rule that the incident showed the code must keep. It is a candidate because a human still decides whether it becomes part of the module's contract.

The **Incident Eval Check** (`scripts/ci/incident-eval-check.mjs`, run by `.github/workflows/incident-eval-check.yml`) looks for both after a pull request that references an incident is merged.

## When it runs, and when it applies

The workflow runs on `pull_request` `closed`, and only when the pull request was merged. It reports on a merge that has already happened, so it cannot block one. A pull request closed without a merge is skipped.

On every merged pull request, the check first reads the pull request's commits, from `refs/pull/<number>/head`, in the range `origin/<base>..<head>`. It then applies when either of these holds:

- the pull request has the `incident` label, in any case;
- its body, or one of its commit messages, contains `Fixes-Incident: <id>`, in any case. The id is the first word after the colon, with one trailing `.`, `,` or `;` dropped. The body is read first, and the first match wins.

The text is matched anywhere in a line, so a description that only quotes it also counts. Write the trigger without its colon when you mean to talk about it.

## What it needs

When it applies, the check walks the checkout of the merged commit and needs:

- a file under `evals/` whose path or content contains the id;
- a file under `docs/invariants/` or `crosscheck/docs/invariants/` whose content contains the id.

Both are plain substring matches, so a file that names `INC-12` also matches `INC-1`.

## Exit codes

- **0.** No incident reference (it prints `no incident reference — skipped`), or both files exist (it prints a `PASS` line).
- **1.** The eval or the invariant is missing, or the `incident` label is set and no id is found. It prints the gate message and one line per missing item.
- **2.** It could not read the pull request's commits: a malformed input, a git error, or an empty range. The range is empty when the pull request's head is already on the base branch, as after a merge commit. This can happen on any merged pull request, with or without an incident reference. A squash merge or a GitHub rebase merge leaves the commits readable, because GitHub keeps `refs/pull/<number>/head` and a rebase merge writes new commit SHAs. The repository's ruleset allows only squash merges.

The full rules are IE-1 to IE-7 in `intent/2026-09-30-incident-eval-range-spec.md`, with IE-2 and IE-8 revised in `intent/2026-10-06-incident-eval-surfaces-spec.md`.

## What each decision means

- **Approving (adding both artefacts)**: open a follow-up pull request that adds the eval and the candidate invariant. Put the same incident reference in its body, so that the check runs on it too and its merge shows a green result. The incident becomes a permanent regression check and a documented invariant.
- **Declining (leaving them out)**: the run on the merge stays red. The merged change leaves the incident with no regression check and no invariant. The ruleset on the default branch requires no status checks, so nothing else happens.

A follow-up pull request with no incident reference of its own is skipped, and re-running the original run is expected to check out the original merge again (inferred from GitHub's event documentation, not run), so neither is expected to turn the original run green.

## What the check does not catch

- It does not run the eval. It only finds a file that contains the id. An eval that no longer reproduces anything still passes, which is why the tier gate reports changes under `evals/` as "not yet reached".
- It does not judge the invariant. Any file in the invariant directories that contains the id counts.
- It does not run before the merge. No check asks for the eval or the invariant while the pull request is open, so add both in the pull request that fixes the incident.

## How long this takes

Writing the eval and the invariant is the work: often an hour. To see what the check saw for a merged pull request `<n>`, run from a clone of the repository:

```bash
git fetch origin
PR_NUMBER=<n> BASE_REF=main \
HEAD_SHA="$(gh pr view <n> --json headRefOid -q .headRefOid)" \
PR_BODY="$(gh pr view <n> --json body -q .body)" \
PR_LABELS="$(gh pr view <n> --json labels -q '[.labels[].name] | join(",")')" \
node scripts/ci/incident-eval-check.mjs; echo "exit $?"
```

It reads the eval and the invariant from your working tree, not from the merged commit, so run it on `main` to see whether a follow-up has landed.

## Who to ask if unsure

The Crosscheck maintainers, via a GitHub issue on this repository.
