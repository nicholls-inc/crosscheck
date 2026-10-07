# Intent: `/generate-verified` emits an evidence record

Task: ER-1.6. Governing roadmap item: ER-1. Issue: #80.

## Problem statement
ER-1's acceptance asks for one Crosscheck pipeline to emit an evidence record. ER-1.3 added the MCP tool `dafny_evidence`, which emits a record with one `proved` claim for a committed `.dfy` file, but no skill calls it. `/generate-verified` ends with a verified program in `.crosscheck/work/dafny/<spec-id>/impl.dfy` and an "Evidence Summary" written by the agent. That summary names no commit, no trusted base and no rerun command, and nothing in it is checked by a tool other than `dafny_verify` on a source string.

Issue #80 names three things the wiring must settle:

1. When the skill commits the `.dfy` file. `dafny_evidence` refuses a dirty work tree (DE-3) and an untracked file (DE-4).
2. Where the record is written. A record written into the tree dirties it, so the next run refuses.
3. Who writes `statement` and `theorems`. The tool decides `proved` from Dafny's exit code, the audit and the declared names. The statement is a draft that a person reviews (rule 1 of `docs/VISION.md`).

A fourth fact shapes all three. `crosscheck/docs/orchestrator-coordination.md` §3 says `.crosscheck/` is gitignored by convention, so `impl.dfy` can never be the tracked file that DE-4 asks for. If `.crosscheck/` is not ignored in a repository, the untracked `spec.dfy` and `impl.dfy` make DE-3 refuse every call.

## Proposed outcome
`/generate-verified` gains a step after a successful verification. When the invocation names a repository path for the verified program, the skill:

- makes sure `.crosscheck/` is ignored, by writing `.crosscheck/.gitignore` with the single pattern `*` when git does not already ignore it. That file ignores itself, so nothing is committed and the repository's own `.gitignore` is not edited;
- refuses to go on, and says why, when the work tree has any other change, rather than committing, stashing or discarding work it did not make;
- writes the verified program to the named path and commits that one path;
- drafts `statement` and `theorems` from the signed-off spec, and passes `requirement` through from the invocation or as `null`;
- calls `dafny_evidence` with `outputPath` `.crosscheck/work/dafny/<spec-id>/evidence.json`, which is ignored, so the tree stays clean and the next run overwrites the record;
- reports the record path, the commit and the claim in the Evidence Summary, and adds an unticked review item asking a person to check the statement against the contracts.

Without a named path the step does not run, and the Evidence Summary says that no record was emitted and why. The answers to the three questions:

1. The skill commits the program after `dafny_verify` accepts it and before `dafny_evidence`, and only when the caller names where it goes. The caller names it because the canonical location of a verified program is the repository's choice, and a commit in someone else's repository is a side effect the caller must ask for.
2. The record goes to `.crosscheck/work/dafny/<spec-id>/evidence.json`, beside the program it describes. The caller copies it to wherever the change ships it, as CGV's caller-named `--evidence-record` file is (ER-1.2).
3. The skill drafts both. The theorems are the declarations whose contracts the spec signed off. The statement is an LLM draft, and the Evidence Summary says so and leaves its review to a person.

`docs/TASKS.md` marks ER-1.6 `done` with this file as its record.

## Affected users and systems
- A user or orchestrator (`byfuglien`) running `/generate-verified` on a repository. With a named path they get a commit and a record that names the commit, the trusted base and a rerun command. Without one they get the same run as today, plus a line saying no record was emitted.
- `crosscheck/skills/generate-verified/SKILL.md`, a Class A protected surface. The change is Tier 3.
- `crosscheck/docs/skills.md` and `crosscheck/README.md`, which describe the skill.
- The `dafny_evidence` tool is unchanged.

## Constraints
- No change to `dafny_evidence` or its spec. The skill follows DE-1 to DE-13 as written.
- The skill never weakens the program or the spec to get a record. A refusal is reported, not worked around.
- Rule 1 of the vision: the statement is an LLM draft, and the record cannot show that a person reviewed it. The skill says so where a reader sees the record.
- The skill never commits a path other than the one named, and never passes `--no-verify`.

## Open questions
None that block the spec. The concerns that stay open are in `intent/2026-10-06-generate-verified-evidence-spec.md` under "Concerns flagged".
