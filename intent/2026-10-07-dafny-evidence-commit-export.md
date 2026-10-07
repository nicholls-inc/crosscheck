# Intent: `dafny_evidence` runs Dafny on the blobs at `commit`, not on the work tree

Task: ER-1.10. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-dafny-evidence-record-spec.md` (DE-3, DE-6, DE-12 and DE-13 amended, DE-14 added, and the concerns on reverted changes and on `nobody` reading the tree rewritten).

## Problem statement
`dafny_evidence` mounts the root of the work tree into the Dafny container, read-only, and checks before and after the runs that the tree matches `commit` (DE-3, DE-13). Two gaps follow.

- **A change made and reverted while Dafny runs is not seen.** DE-13 compares the state before and after the runs. An edit made after DE-3's check and undone before DE-13's passes both, and Dafny may have read the edited file. The record then says `commit` is proved when Dafny checked something else.
- **The whole repository is in the container.** Dafny needs only `file` and the files DE-12 reaches through `include`, but every file in the tree, including ignored ones such as `.env`, is readable inside the container.

The task asks whether to mount a `git archive` copy of `commit` instead, at the cost of copying the tree on every run, and to implement it if the answer is yes.

## Decision
Yes, with a narrower copy than `git archive` of the whole tree. The tool already knows every file Dafny reads: DE-12 states that `file` and its includes are the only files Dafny 4.11.0 reads in this invocation. So the tool reads each of those files as its blob at `commit`, scans the blob for includes, writes the same bytes into a fresh temporary directory, and mounts that directory at `/work`. The copy is the include closure, not the tree, so its cost grows with the proof, not with the repository. On the fixtures it is one or two files of under 100 bytes.

`git archive` itself is not used, because it applies `.gitattributes` export filters (`export-subst`, `export-ignore`, end-of-line conversion), so its output can differ from the blob. `git cat-file blob` returns the blob's bytes with no filter.

## Proposed outcome
- The tool reads `file` and each include as the blob at `commit`, through `git ls-tree` and `git cat-file blob`, and refuses a path whose entry at `commit` is not a regular file (mode `100644` or `100755`). The include scan reads those bytes, not the work tree.
- Before the first Dafny run, the tool writes exactly those files, with the scanned bytes, into a new temporary directory, with directories mode `0755` and files mode `0644`, and every Dafny run mounts that directory read-only at `/work`. The directory is removed when the tool returns.
- A change to the work tree during the runs, reverted or not, cannot change what Dafny reads. DE-13 stays, so a change that is not reverted still refuses, as before.
- The rerun command (DE-9) does not change. It still mounts the auditor's checkout of `commit`.
- `docs/TASKS.md` marks ER-1.10 `done` with this file as its record.

## Affected users and systems
- A caller of `dafny_evidence` gets the same record for the same inputs. The Dafny arguments do not change, so the recorded fixtures of ER-1.9 still replay.
- On a Linux host, a tree that `nobody` cannot read no longer fails the tool's runs, because the export's modes are set. The rerun command still mounts the auditor's tree, so the concern stays for it.
- `dafny_verify`, `dafny_compile`, the Lean tools, the evidence record format and its checker do not change. No skill calls `dafny_evidence` yet (ER-1.6).

## Constraints
- No new dependency. The export uses `git` and `node:fs`.
- DE-3, DE-4 and DE-12's work-tree checks stay. They no longer decide what Dafny reads, but they keep the record's rerun command, which mounts a checkout, true of the tree the caller holds, and removing them is a separate decision.
- The temporary directory uses the `dafny-` prefix, so `dafny_cleanup` removes one left by a crash.

## Open questions
None that block the spec. Whether the rerun command should also build its mount from `commit` rather than the auditor's checkout is a separate question, recorded as ER-1.18.
