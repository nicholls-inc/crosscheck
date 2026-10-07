# Intent: `dafny_evidence` checks that `requirement` names a tracked file, and refuses a theorem named twice

Task: ER-1.11. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-dafny-evidence-record-spec.md` (DE-1 and DE-4 amended, and a concern on the anchor added).

## Problem statement
Two inputs of the MCP tool `dafny_evidence` are checked less than the record they feed promises.

- **`requirement`.** The evidence record spec defines a claim's `requirement` as "a repository path with an optional `#anchor`", and the tool's input description in `crosscheck/mcp-server/src/index.ts` says the same. DE-1 checks only that the string is not blank. So `"see the ticket"`, `"/etc/passwd"`, `"../other-repo/req.md"` and `"docs/missing.md"` all pass, and the record then names a requirement path that no reader can open at `commit`.
- **`theorems`.** DE-1 accepts a list that names one theorem twice, such as `["AbsNonneg", "AbsNonneg"]`. The record's `basis.theorems` then repeats the name, which reads as two proofs where there is one.

## Proposed outcome
- DE-1 refuses a `requirement` string whose path part, the trimmed text before the first `#`, is empty, absolute, uses `\`, or has a `..`, `.` or empty segment, and one with a `#` and nothing after it.
- DE-4 runs its checks on the requirement's path as well, before any Dafny run: the path must be a tracked regular file in the work tree, not a symbolic link and not reached through one. With DE-3's clean tree, that is the file at `commit`.
- DE-1 refuses each theorem name that appears more than once, with one error per name.
- The input description of `requirement` says it must name a tracked file.
- The record does not change for an input that passes. `null` stays the way to say a claim traces to no requirement.
- `docs/TASKS.md` marks ER-1.11 `done` with this file as its record.

The task offered a second option, describing the field as free text. The evidence record spec settles it: the format already defines `requirement` as a repository path, so a free-text description would make the tool's input disagree with the format it emits.

## Affected users and systems
- A caller that passes a `requirement` naming no tracked file, or a duplicate theorem name, is refused. No skill calls `dafny_evidence` yet (ER-1.6), so no pipeline in this repository changes.
- The evidence record format, `scripts/check-evidence-record.mjs`, CGV's record, `dafny_verify`, `dafny_compile` and the Lean tools do not change.

## Constraints
- No new dependency. The check reuses DE-4's git and file checks, so a requirement path and `file` are held to one rule.
- The anchor is not checked against the file. Whether `#abs` names a heading depends on the renderer, and the format does not fix an anchor syntax. The spec flags that as not yet reached.

## Open questions
None that block the spec.
