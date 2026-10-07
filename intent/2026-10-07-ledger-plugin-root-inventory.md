# Intent: A Crosscheck plugin root holds at least one skill and one agent

Task: PB-1.44. Governing roadmap item: PB-1. Found in PB-1.41 (`intent/2026-10-07-ledger-plugin-root-marker.md`).

## Problem statement
LL-11 marks a plugin root by its manifest alone. Measured on `origin/main` at 60e1a43 with the conformance binary built from `crosscheck/conformance`:

- A directory that holds only `.claude-plugin/plugin.json` with `{"name":"crosscheck"}` prints `skills discovered : 0`, `agents discovered : 0` and `RESULT: PASS`, exit 0. Copying the manifest is enough to make a run vouch for nothing.
- A manifest `{"Name":"crosscheck"}` prints `RESULT: PASS`, exit 0, because `json.Unmarshal` matches the key `name` without regard to case. `claude plugin validate` (Claude Code 2.1.292) rejects the same manifest with `name: Invalid input: expected string, received undefined`. So the oracle accepts a root that Claude Code would not load as a plugin.
- A manifest `{}` fails LL-11 with `names "", want "crosscheck"`, and a `plugin.json` that is a directory fails LL-11 with `read ...: is a directory`. No test names either case.

The task row asked what the minimum inventory of a Crosscheck tree is: at least one skill and one agent, or the skills the README names. The checks the oracle runs settle it:

- The AUTO checks scan two kinds of artefact, skills (`skills/<name>/SKILL.md`) and agents (`agents/<name>.md`). With none of a kind, every structural and routing check on that kind passes vacuously. At least one of each is the least inventory under which each AUTO check has something to check.
- "The skills the README names" adds nothing the oracle lacks. AUTO 2 (phantom) already fails every `/name` that the doc set names and the tree lacks. And it gives no floor: a tree with no README names nothing, so it would still pass empty.
- Every released Crosscheck tree meets the floor. The installed copies 2.6.0, 2.7.0 and 2.8.0 each hold 30 skills and 5 agents, and so does `crosscheck/` today (`TestGoldenRealTree`).
- An exact inventory (30 and 5) would fail the run each time a skill is added or removed. `TestGoldenRealTree` already pins the exact counts of the real tree, where a change to them is reviewed.

## Proposed outcome
- New rule LL-12. When LL-11 passes (the root is a directory whose manifest names `crosscheck`) and the scan finds no skill or no agent, `analyze` appends one error, `[root] plugin root <root> is not a Crosscheck plugin tree: it holds <n> skills and <m> agents, want at least one skill (skills/<name>/SKILL.md) and one agent (agents/<name>.md)`. A skill directory without a `SKILL.md` is not a skill, as discovery already treats it.
- LL-11 reads the key `name` exactly, as Claude Code does. A manifest whose only name key is `Name` fails LL-11 with `has no "name" key`. A `name` that is not a string fails LL-11.
- `TestLedgerLoadRoot` pins the LL-11 cases `{}`, a `plugin.json` that is a directory, a `"Name"` key and a non-string `name`, and the LL-12 cases of a manifest-only root, a root with no agent, a root with no skill and a root whose only skill directory has no `SKILL.md`.
- The spec, the `main.go` header and `crosscheck/conformance/README.md` state LL-12 and the exact key match.

## Affected users and systems
- Anyone who runs the conformance oracle on a directory that copies the Crosscheck manifest without the plugin's skills and agents. The run now fails.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.
- The `conformance` CI job runs on the real tree (30 skills, 5 agents), so it still passes.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- LL-1 to LL-10 keep their rules and messages. LL-11 keeps its message prefix.
- A root that fails LL-11 does not also fail LL-12, so each wrong root still fails with one root error.

## Open questions
None. The measurements above answer the task row's question.

Not yet reached: a tree with one skill and one agent copied next to the manifest passes, whatever else is missing. The property that blocks it is a check that ties the tree to a released Crosscheck inventory, and the open question is whether such a check can be written without pinning a count that changes with every release.
