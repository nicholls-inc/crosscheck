# Task queue

This file is the ordered queue of work on this repository. The order of the rows is the order of work. To take a task, follow "Pick up the next task" in [`assurance/DEVELOPMENT-FRAMEWORK.md`](assurance/DEVELOPMENT-FRAMEWORK.md).

## Columns

- **Task.** `<roadmap item ID>.<n>`. The item is in [`assurance/ROADMAP.md`](assurance/ROADMAP.md) and governs the task. An ID is never reused.
- **Status.** One of three values:
  - `todo`: nobody has finished the task.
  - `blocked`: the task waits for something outside the queue, such as a decision by a person or a fix elsewhere. The row says what unblocks it.
  - `done`: the pull request that completed the task is merged.

  The queue has no "in progress" status. A pushed branch named `task/<task ID>` shows that an agent holds the task.
- **Depends on.** Tasks that must be `done` first.
- **Issue.** The GitHub issue where the discussion is, if one exists.
- **Record.** The path of the intent file, once the task has one.

## Rules

- One task is one pull request. If a task is too large for one pull request, its pull request replaces the row with smaller rows, points every `Depends on` that named the old row at the new rows, and does nothing else.
- The pull request that completes a task sets its row to `done` and fills in the record.
- Add a row under an existing roadmap item. If no item fits, the work needs a new roadmap item first, and only the maintainer approves one.
- The queue grants no authority. A change to a protected surface cites a roadmap item.

## Queue

| Task | Status | What | Depends on | Issue | Record |
|---|---|---|---|---|---|
| PB-1.1 | done | Add this queue, the roadmap items for the vision, and the pick-up procedure | | | `intent/2026-09-30-task-queue.md` |
| PB-1.2 | done | Fix the `Incident Eval Check` workflow. It failed on the four squash-merged pull requests #43 to #46 with "Invalid revision range" | | | `intent/2026-09-30-incident-eval-range.md` |
| PB-1.3 | done | Stop merged governance notes from unlocking the protected-surface hook. The notes on `main` allow edits to 29 of 57 protected files | | | `intent/2026-09-30-merged-notes-unlock.md` |
| PB-1.4 | done | Tier gate: anchor the `Tier:` line, and label unchecked code "not yet reached" | | #50 | `intent/2026-09-30-tier-anchor.md` |
| PB-1.5 | done | Tier gate: accept any citation line, require a regular file in the repository, and add negative tests | | #49 | `intent/2026-09-30-citation-rule.md` |
| PB-1.6 | done | Add a script that prints the next task by the rules of the pick-up procedure, and check this file in CI: each task ID names a roadmap item, each dependency exists, each status is valid, and a pull request sets to `done` only the row in its `Task:` line. Add a test that races two claims against a scratch remote | | | `intent/2026-10-01-queue-check.md` |
| PB-1.7 | done | Tier Gate workflow: pass the base ref to `run:` as an environment variable, not a `${{ }}` expression, and stop writing the changed files through a fixed `EOF` heredoc delimiter, which a file named `EOF` ends early | | | `intent/2026-10-01-tier-gate-workflow.md` |
| PB-1.8 | done | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` says `incident-eval-check.yml` fails on an incident record without an eval. State its real trigger (the `incident` label or an incident line in the body or a commit), its exit 2, and that it runs after the merge and cannot block it | | | `intent/2026-10-06-incident-eval-doc.md` |
| PB-1.9 | done | Add pre-commit hooks, as the roadmap's dual-track principle asks, for the parts of the tier gate and the task queue check that need no PR body. Each runs in under 5 seconds and prints the command that fixes the failure | | | `intent/2026-10-06-pre-commit-hooks.md` |
| PB-1.10 | done | Tier gate: read changed file names unquoted. `git diff --name-only` quotes a path with a byte outside printable ASCII, so `docs/assurance/é.md` reaches the gate as `"docs/assurance/\303\251.md"` and matches no protected glob | | | `intent/2026-10-06-unquoted-paths.md` |
| PB-1.11 | done | `docs/gates/tier-layer-gate.md`, under "What the incident-eval check adds", says the tier gate expects an incident's eval "before the gate will pass". The tier gate reads no incident reference, and `incident-eval-check.yml` runs after the merge. State what the Incident Eval Check needs and when it runs | | | `intent/2026-10-06-tier-gate-incident-doc.md` |
| PB-1.12 | todo | Make the rest of the repository describe the Incident Eval Check as `DEVELOPMENT-FRAMEWORK.md` stage 5 now does. Its failure message in `scripts/ci/incident-eval-check.mjs` says the change "stays blocked", though it prints after the merge, and links to the gate index rather than an explainer, so add one under `docs/gates/`. `docs/assurance/TIER-LAYER-MAP.md` names the check as the evidence for `evals/**`. `evals/README.md` asks for a candidate invariant only "where applicable". Stage 6 and the stage table of `DEVELOPMENT-FRAMEWORK.md` say "incident record + eval" with no invariant. IE-2 says a rebase merge leaves an empty range, but GitHub's rebase merge writes new commit SHAs and no test covers it | | | |
| PB-1.13 | todo | `.husky/commit-msg` reads `git diff --cached --name-only`, which quotes a path with a byte outside printable ASCII. The quoted name ends in `"`, so a `SKILL.md` or `agents/*.md` under such a directory escapes the commit-type check. Read the names with `-z` | | | |
| PB-1.14 | todo | `.claude/hooks/protected-surface-guard.mjs` compiles a glob's `**` to `.*`, which stops at a newline, so the hook lets an edit to `docs/assurance/n<newline>l.md` through. The tier gate compiles with the `s` flag since PB-1.10 | | | |
| PB-1.15 | todo | Tier gate fails closed when `CHANGED_FILES_PATH` is unset or the file is missing. `readChangedFiles` in `scripts/ci/tier-gate.mjs` returns an empty list when the variable is unset, so a manual run or a workflow edit that drops or misspells it passes at Tier 1. A missing file crashes with a stack trace rather than a message. Exit 1 with a message naming the variable in both cases, and add a test for each | | | |
| PB-1.16 | todo | The Incident Eval Check matches its trigger text anywhere in a line, so prose that quotes the trigger fires it. The run for #62, whose body and commits described the check, failed after the merge and named the incident `<id>` followed by a backtick. Decide which lines count as an incident reference, and add a test for quoted prose | | | |
| VA-1.1 | todo | Skills and agents stop presenting the `intent-check` attestation as a required artefact | | | |
| VA-1.2 | done | Replace "out of scope", "not addressed" and "best-effort" in `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` with "not yet reached", the blocking property, and the open question | | | `intent/2026-10-06-not-yet-reached-docs.md` |
| VA-1.4 | todo | `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` give Layer 5 the confidence "Probabilistic (~96%)" from `/intent-check`, and say Layers 4 to 6 "prove" the spec is right. Rule 1 of the vision makes `/intent-check` a search tool. State Layer 5 as search, and say what each of Layers 4 to 6 proves, tests or only searches | | | |
| VA-1.5 | todo | Skills and agents call Layer 6 or a class of code "best-effort", "out of scope" or "not addressed": `crosscheck/agents/hellebuyck.md` and `SKILL.md` files such as `audit-spec-coverage`, `audit-invariant-consistency`, `assurance-layer-audit`, `spec-adversary` and `drt-oracle`. The row owns every match in `crosscheck/agents` and `crosscheck/skills` that describes a class of code or a layer with one of the three phrases. A match that means something else, such as a best-effort grep, needs no change, including `intent-check/SKILL.md` (VA-1.1 owns only the attestation wording there). Say "not yet reached", the blocking property, and the open question. These are Class A protected surfaces, so the pull request needs a `/crosscheck:protected-surface-amend` note, and the `feat(crosscheck)` or `fix(crosscheck)` commit type | | | |
| VA-1.6 | done | Do the same as VA-1.2 for the rest of the docs under `crosscheck/docs/` that call a class of code or a layer "out of scope", "not addressed" or "best-effort", starting with "What this hierarchy is not good for" and the Layer 2, 3 and 6 sections of `crosscheck/docs/research/assurance-hierarchy.md`. Leave dated research records and retrospectives as written, and say so. Files under `crosscheck/docs/invariants/` are Class B protected surfaces, so editing one needs a `/crosscheck:protected-surface-amend` note | | | `intent/2026-10-06-not-yet-reached-research-docs.md` |
| ER-1.1 | done | Write the intent and the spec for the evidence record format | | | `intent/2026-10-06-evidence-record.md` |
| ER-1.2 | done | CGV emits an evidence record | ER-1.1 | | `intent/2026-10-06-cgv-evidence-record.md` |
| ER-1.3 | done | One Crosscheck pipeline emits an evidence record | ER-1.1 | #80, #81 | `intent/2026-10-06-dafny-evidence-record.md` |
| ER-1.4 | done | Add a deterministic checker for evidence records | ER-1.1 | | `intent/2026-10-06-evidence-record-checker.md` |
| ER-1.5 | todo | Run `scripts/check-evidence-record.test.mjs` in CI. No workflow runs it, because the tier gate's step runs only `scripts/ci/*.test.mjs`. Adding a step edits `.github/workflows/**`, a Class A protected surface, so the pull request needs a `/crosscheck:protected-surface-amend` note | ER-1.4 | | |
| ER-1.6 | todo | Wire `/generate-verified` to call the MCP tool `dafny_evidence`, so the Dafny pipeline a user runs emits an evidence record. Settle when the skill commits the `.dfy` file, since the tool refuses a dirty work tree (DE-3), and where the record is written. `SKILL.md` is a Class A protected surface, so the pull request needs a `/crosscheck:protected-surface-amend` note citing ER-1 and a `feat(crosscheck)` commit | ER-1.3 | #80 | |
| ER-1.7 | todo | Harden the `docker run` flags of `dafny_evidence`'s Dafny runs and of the rerun command it writes: `--cap-drop=ALL`, `--security-opt=no-new-privileges`, a `--pids-limit`, and a non-root user. Amend DE-6 and DE-9 of `intent/2026-10-06-dafny-evidence-record-spec.md` | ER-1.3 | | |
| ER-1.8 | todo | `dafny_evidence`: refuse an `outputPath` that does not end in `.json` or that passes through a directory whose name starts with `.`, amending DE-11; refuse an include whose path does not end in `.dfy`, amending DE-12; name the image ID in the rerun command, not the tag, amending DE-9. The spec's "rerun names a tag" concern says an auditor on another machine gets a different ID, so the row must settle what that auditor runs | ER-1.3 | | |
| ER-1.9 | todo | Tests for `dafny_evidence`: run `scripts/check-evidence-record.mjs` on a record the tool emits (DE-10), and evidence DE-5 to DE-8 against real Dafny output in a suite `npm test` runs, not only in the e2e suite it skips | ER-1.3 | | |
| ER-1.10 | todo | Decide whether `dafny_evidence` mounts a `git archive` copy of `commit` instead of the work tree. That would stop mounting the whole repository into the container (DE-6) and close the DE-13 gap of an edit made and reverted while Dafny runs, at the cost of copying the tree on every run. Record the decision in the spec, and implement it if it is yes | ER-1.3 | | |
| CG-1.1 | done | CGV README: say what the tool is for relative to type checkers, and qualify the exit 0 claim | | #10 | `intent/2026-10-06-cgv-readme-type-checkers.md` |
| CG-1.7 | todo | Accept `int` where `float` is required, and `int` or `float` where `complex` is required (PEP 484 numeric tower). 35 of the 53 triaged false positives. The `type` check is `st = tt` in `constraintImplies` (`cgv/prover/ContractGraph/Checker.lean`), a protected definition, so widening it changes `protected-statements.txt` and needs Tier 3, a governance note and a rule in `BehaviorModel.lean`. The intent decides first whether an extractor-side change avoids that. Add a fixture with an `ok.py` | | #5 | |
| CG-1.8 | todo | Treat a parameter typed `object` as accepting None, and add fixtures that pin the same for `Any` and an unannotated parameter. `annotation_facts` in `cgv/src/dataclass_extractor.rs` gives every plain name except `Any` the nullability `Some(false)`. Add a fixture with an `ok.py` | | #5 | |
| CG-1.9 | todo | Extend narrowing in `cgv/src/flow.rs` to four patterns: reassignment after an early return (`v = None; ...; if not v: return; v = int(v); f(v)`), a caller's truthiness guard on an attribute, `x in {literals}`, and `k in d` before `d.get(k)`. Add a fixture with an `ok.py` for each | | #5 | |
| CG-1.10 | todo | Model `Model.model_validate(...)` and `model_validate_json(...)` as a validation boundary rather than a typed write: pydantic lax mode coerces `"true"` to `bool` and rejects `None` on purpose. The write is in `cgv/src/edge_discovery.rs` and the constructor facts in `cgv/src/value_analysis.rs`. `BehaviorModel.lean` says only that pydantic validates at construction, so the intent decides whether it needs a rule for lax coercion. Add a fixture with an `ok.py` | | #5 | |
| CG-1.11 | todo | Treat a call to a `NoReturn` function (`assert_never`, a function annotated `NoReturn` or `Never`, `sys.exit`) as an exit, and an exhaustive `match` over every member of an `Enum` as having no fall-through. `terminates` and `always_exits` in `cgv/src/flow.rs` treat `case _ as x: assert_never(x)` as falling through. Add a fixture with an `ok.py` | | #5 | |
| CG-1.3 | todo | Close the extractor gaps that turn real nullability bugs into warnings | | #6 | |
| CG-1.4 | todo | Hide missing-guarantee warnings by default, and report them as coverage for each module | | #9 | |
| CG-1.5 | todo | Baseline mode: report only the findings that a change introduces | | #7 | |
| CG-1.6 | todo | Give each error a checkable witness: a concrete value or a generated failing test | | #8 | |
| TB-1.1 | todo | CGV CI: replay the Lean kernel, so that a declaration that skipped the kernel cannot pass the axiom check | | #47 | |
| TB-1.2 | todo | CGV: reject `implemented_by` and `extern` on constants that the soundness theorems reach | | #48 | |
| TB-1.3 | todo | CGV manifest: hash the definitions that protected statements mention, and check the name lists against the rules table | | #51 | |
| TB-1.4 | todo | Write the intent and the plan for proving extraction, and split the work into rows | | #16 | |
| AD-1.1 | todo | Review issues #19 to #41 against `docs/VISION.md` and record one decision for each. Refine: rewrite the issue against the vision, name the rule or item it serves, and add a row under that item. Drop: close the issue with the reason. Start no work on any of them in this task | | #27 | |
