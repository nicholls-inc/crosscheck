# Intent: The Dafny pipeline emits an evidence record

Task: ER-1.3. Governing roadmap item: ER-1.

## Problem statement
`intent/2026-10-06-evidence-record-spec.md` defines version 1 of the evidence record. No Crosscheck pipeline writes one. ER-1's acceptance asks for one Crosscheck pipeline to emit a record in the format, and the spec leaves ER-1.3 to choose which.

Today a passing `dafny_verify` returns `{ success: true, errors, warnings, rawOutput, difficulty }`. That result does not name the commit it describes, the theorems behind it, the trusted base, or a command that reruns it. It also verifies a source string passed in by the caller, so nothing ties the result to a file at a commit.

`dafny_verify` success also does not mean every declaration was proved. A probe on 2026-10-06 against `crosscheck-dafny:latest` (Dafny `4.11.0+fcb2042d`) showed:

| Program | `dafny verify` exit | `dafny audit` |
|---|---|---|
| A lemma with a body that proves it | 0, "1 verified, 0 errors" | 0 findings |
| `lemma {:axiom} Bad(x: int) ensures x > 0` with no body | 0, "0 verified, 0 errors" | exit 0, "1 findings" |
| `assume x > 0;` with no `{:axiom}` | 2, warnings fail the run | |
| A bodiless lemma with no `{:axiom}` | 2 | |
| `{:verify false}` | 2 | 1 finding |
| `{:extern}` with an `ensures` | 2 | 1 finding |
| A lemma whose postcondition fails | 4 | |

So an `{:axiom}` declaration passes `dafny verify` with exit 0, and `dafny audit` reports it but also exits 0. A `proved` claim that rests only on the verify exit code could rest on an axiom.

## Proposed outcome
- A new MCP tool, `dafny_evidence`, takes a git repository, the path of a committed `.dfy` file, a plain-language statement, the requirement it traces to or `null`, and the names of the theorems that prove the statement. It runs `dafny verify` and `dafny audit` on the file as committed, and emits a version 1 evidence record with one `proved` claim when verification passes, the audit has no findings, and every named theorem is declared in the file. Otherwise it refuses and says why.
- The record's trusted base names the Dafny version that the image reports, the Z3 solver that ships in the same Dafny release, and the local image's ID. Its rerun command runs the same two Docker commands from the repository root, and exits 0 only when both pass.
- The tool writes the record to a path when asked, and always returns it.
- The spec, `intent/2026-10-06-dafny-evidence-record-spec.md`, states the inputs, the refusals and the record the tool builds.
- `docs/TASKS.md` marks ER-1.3 `done` with this file as its record.

## Affected users and systems
- A Claude Code user running the Dafny pipeline (`/generate-verified`, `/spec-iterate`) gets a record that an engineer or auditor can read and rerun, without reading the skill that produced the result.
- The MCP server in `crosscheck/mcp-server/` gains a tool. The six existing tools do not change.
- ER-1.4's checker gets a real record to check.
- `CLAUDE.md` and `crosscheck/README.md` list the MCP tools, and both gain the new one.

## Constraints
- The record must satisfy EV-1 to EV-12 of the evidence record spec.
- No LLM decides any part of the claim's strength. The caller, usually an LLM, drafts the statement and names the theorems. The tool decides `proved` only from Dafny's exit code, the audit's finding count and the theorem names that the file declares. Rule 1 of `docs/VISION.md` makes the statement a draft that a person reviews, and the record cannot show that a person did. The spec flags it.
- The skills that drive the Dafny pipeline are Class A protected surfaces. This task does not edit them, so no skill calls the tool yet. A follow-up issue covers wiring `/generate-verified` to call it.
- No new dependency.

## Open questions
None that block the spec. Where a record lives is left open by the evidence record spec. This tool writes it to a path that the caller chooses and returns it, which does not settle the question. The spec flags the rest.
