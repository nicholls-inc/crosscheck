# Spec: `/generate-verified` emits an evidence record

Intent: `intent/2026-10-06-generate-verified-evidence.md`. Governing roadmap item: ER-1. Task: ER-1.6. Issue: #80.

This spec adds a step to `/generate-verified` that commits the verified program and calls `dafny_evidence` (`intent/2026-10-06-dafny-evidence-record-spec.md`, rules DE-1 to DE-13). Rules GV-1 to GV-9 state what the skill does. The tool is unchanged, and every check that makes the record true stays in the tool. The skill only puts the repository in the state the tool asks for, and reports what the tool returned.

## Invocation

`/generate-verified` takes two new optional arguments.

| Argument | Meaning |
|---|---|
| `evidence: <path>` | The path, relative to the work tree's top level, where the verified program is committed. It must end in `.dfy` and satisfy DE-1's path rule: `/` separators, no `..`, `.` or empty segment, and only `A-Z a-z 0-9 _ . / -`. |
| `requirement: <path[#anchor]>` | The requirement the claim traces to. It becomes the claim's `requirement`. Without it, `requirement` is `null`. |

## Rules

- **GV-1. Opt-in.** The step runs only when the invocation names `evidence:`. Without it, the skill emits no record, makes no commit and writes no `.crosscheck/.gitignore`, and the Evidence Summary says `No evidence record: no evidence path was named.` The verified file in `.crosscheck/work/` cannot be the tracked file DE-4 asks for, because `.crosscheck/` is gitignored by convention (`crosscheck/docs/orchestrator-coordination.md` §3). Where a repository keeps a verified program, and whether the skill may commit there, are the caller's to say.
- **GV-2. Only a verified program.** The step runs only after `dafny_verify` accepted the program in Step 3 and the skill wrote it to `.crosscheck/work/dafny/<spec-id>/impl.dfy` in Step 6. After five failed attempts there is no commit and no record. The file committed under GV-5 is byte for byte that `impl.dfy`.
- **GV-3. `.crosscheck/` is ignored.** The skill runs `git check-ignore -q .crosscheck/work/dafny/<spec-id>/impl.dfy` at the top level. When it exits non-zero, the skill writes `.crosscheck/.gitignore` holding the single line `*`. That pattern ignores every file under `.crosscheck/`, the `.gitignore` included, so the repository's own `.gitignore` is not edited and nothing is committed. It runs the check again and stops the step, with the output, if it still fails.
- **GV-4. A clean tree, or no record.** The skill runs `git status --porcelain --untracked-files=all` at the top level after GV-3. When it prints anything, the step stops: no commit and no record, and the Evidence Summary lists each path it printed. The skill does not commit, stash, discard or ignore a change it did not make. DE-3 would refuse the call anyway, and stopping first avoids a commit made for nothing.
- **GV-5. Commit the one path.** The skill writes the program to the `evidence:` path, creating its directory, and runs `git add -- <path>`. When `git add` fails, for example because the path is ignored, the step stops and reports git's output. When `git diff --cached --quiet` then exits 0, the path at HEAD already holds the program and the skill makes no commit. Otherwise it runs `git commit -m "chore: add <path>, verified by /generate-verified"`. It never passes `--no-verify` or `--amend`. When the commit fails, for example on a hook, the step stops and reports git's output, and the path stays staged for the caller, who must commit or unstage it before a later run, since the clean-tree rule stops on a staged file. When `git add` fails, the report says the path was written but not staged.
- **GV-6. The theorems.** `theorems` lists the fully qualified names, as DE-5 reads them from Dafny's log, of the methods, functions and lemmas that carry the `requires` and `ensures` clauses signed off in the spec: `M.C.Name` for `Name` in class `C` of module `M`, and `Name` alone at the top level of the file. Helper lemmas and loop invariants added in Step 2 are not listed, since they prove the spec's contracts and do not state them.
- **GV-7. The statement.** `statement` restates, in plain language for a reader who will not open the code, what the contracts of the GV-6 theorems guarantee, and nothing more. It is an LLM draft. Rule 1 of `docs/VISION.md` leaves its review to a person, and the record has no field that shows a review (the dafny-evidence spec's second concern).
- **GV-8. The call.** The skill calls `dafny_evidence` with `repoPath` the top level, `file` the `evidence:` path, `statement` from GV-7, `requirement` from the invocation or `null`, `theorems` from GV-6, and `outputPath` `.crosscheck/work/dafny/<spec-id>/evidence.json`. That path is ignored under GV-3, so the record does not dirty the tree, and a later run overwrites it under DE-11. When the call returns `success: false`, the skill reports every error as the tool gave it. It may call again once, only after an error that names a theorem under DE-1 or DE-5, with corrected names. When the corrected set differs from the set in the statement, the skill redrafts the statement so it covers exactly the theorems it passes, and says in the summary that the set changed. It never edits the program, the spec or a contract to get a record.
- **GV-9. The report.** The Evidence Summary names the record path, the commit, the claim `id` and its `strength`, or the reason no record was emitted. When a record was emitted, the "Decisions for Review" block lists, unticked, for a person: whether the statement says what the contracts of the named theorems say. The summary says the record describes the commit, not the work tree, and that the caller copies it to wherever the change ships it.

## Verification

A `SKILL.md` has no unit tests. The step is checked by following it by hand on a scratch repository against the real Dafny image, and by checking the record with `scripts/check-evidence-record.mjs`:

- with `.crosscheck/` not ignored, `git status` lists the work files; after GV-3 it lists nothing;
- after GV-5 and GV-8, `dafny_evidence` returns a record for the commit, the tree is still clean, the checker exits 0, and the record's rerun command exits 0 under `sh`;
- a second call overwrites the record and succeeds;
- with an unrelated untracked file, GV-4 stops before the commit, and `dafny_evidence` called anyway refuses under DE-3, naming the same path;
- a theorem named without its module refuses under DE-5.

## Concerns flagged, not resolved here

- **The statement is unreviewed in the record.** GV-9 asks a person to check it, but the record carries no sign that anyone did. This is the dafny-evidence spec's second concern. A record that marks a statement as reviewed is not yet reached. The property that blocks it is a review field in `evidence-record/1`, and the open question is how the format records a person's review.
- **The record lives in an ignored file.** A change does not ship its record unless the caller copies it into a commit, a pull request or a CI artefact. Committing the record by default is not yet reached. The property that blocks it is that a committed record dirties the tree for the next run (DE-3) and describes the commit before it. The open question is whether records belong in the repository, beside the change, or outside it.
- **The opt-in leaves the default run without a record.** A run with no `evidence:` path emits nothing, so ER-1's acceptance is met only when a caller asks. A default path is not yet reached. The property that blocks it is a location for verified programs that every repository shares, and the open question is whether the plugin should set one.
- **`/spec-iterate` emits no record.** Issue #80 names it too. Its spec has no implementation, so its contracts verify only with bodies `/generate-verified` writes later, and a record of the spec alone would claim the contracts are consistent, not that code meets them. It stays with issue #80.
