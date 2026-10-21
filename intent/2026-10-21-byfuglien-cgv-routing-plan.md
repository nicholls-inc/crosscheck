# Plan: `byfuglien` routes a Python data-model change to `cgv_check`

Intent: `intent/2026-10-21-byfuglien-cgv-routing.md`. Spec: `intent/2026-10-21-byfuglien-cgv-routing-spec.md`.

## Files that change

| File | Where | Change |
|---|---|---|
| `crosscheck/agents/byfuglien.md` | Skills section, Task Classification table, Phase 3, Phase 4 | BC-1 to BC-8: a CGV tool entry, a classification row, a "CGV check" procedure, a CGV quality gate |
| `.assurance/protected-surface-amend/byfuglien-cgv-routing-2026-10-21.md` | new | governance note naming the agent file |
| `docs/TASKS.md` | ER-1.21 row | `done`, record |
| `crosscheck/JOURNAL.md` | new entry | |

## Order of work

1. Commit the intent, spec and plan.
2. Write the governance note, so the PreToolUse hook allows the edit.
3. Edit the agent.
4. Update the ER-1.21 row and `crosscheck/JOURNAL.md`.
5. Run the checks below.

## Risks

- Overclaiming CGV's result. The report wording is taken from the CK-9 result shape, CGV's record (CR spec) and `cgv/README.md` "What exit 0 promises", and the qualifiers travel with every result.
- A refusal read as a pass. BC-3 names every failure path, and the Phase 4 gate checks it.
- Conflating the baseline run with the record run. BC-4 and BC-5 keep exit 0 under a baseline apart from the record's claim.
- A dirty work tree is common in a local session, so the route often reports "not run". That is the intended behaviour (intent decision 6).

## Proof

- `go vet ./...`, `go test ./...` and `go run . ..` in `crosscheck/conformance` pass: the agent keeps its frontmatter and routes to no phantom skill.
- `grep -n "cgv_check" crosscheck/agents/byfuglien.md` shows the classification row, the procedure and the gate.
- `grep -n -i "verified" crosscheck/agents/byfuglien.md` shows no line that applies the word to a CGV result.
- `node --test scripts/ci/*.test.mjs`, `node scripts/ci/skill-references.mjs`, `BASE_REF=main PR_BODY='Task: ER-1.21' node scripts/ci/task-queue.mjs check`, and `node scripts/ci/tier-gate.mjs` with the PR body pass.
