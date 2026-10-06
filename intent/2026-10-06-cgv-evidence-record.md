# Intent: CGV emits an evidence record

Task: ER-1.2. Governing roadmap item: ER-1.

## Problem statement
`intent/2026-10-06-evidence-record-spec.md` defines version 1 of the evidence record. CGV does not write one. A run of `crosscheck-contracts contracts check` prints the checker's JSON and an exit code. Exit 0 rests on `ContractGraph.runChecker_sound_all`, but the output does not name the theorem, the commit it checked, the trusted base, or the command that reruns it. A reader of a pull request cannot tell from CGV's output that exit 0 is a proof relative to an extractor nobody has proved.

The format spec leaves five decisions to this task: where the record lives, whether `BehaviorModel.lean` stays in the trusted base, how the contracts tagged `ASSUMED` are stated, whether the permitted axioms join the trusted base, and how a record names the commit it describes.

## Proposed outcome
- `contracts check` takes `--evidence-record PATH`. After a run that exits 0, PATH holds a version 1 record with one `proved` claim about the checked project. After a run with any other exit code, PATH holds nothing, so a stale record cannot outlive a failed run.
- The record names the commit of the checked project, read from git. CGV refuses to write a record, and exits 2 before extraction, when the project is not in a git repository or has uncommitted or untracked changes under the checked path or the overrides file. A record about a commit must describe the files at that commit.
- The claim's rerun command is the `contracts check` command, with the same options, run from the repository root.
- The five decisions, each stated in `intent/2026-10-06-cgv-evidence-record-spec.md`:
  - The record is a file the caller names. CI artefact, later commit or pull request: the caller chooses where it goes.
  - `BehaviorModel.lean` stays in the trusted base, as the format spec's worked example has it. An understated trusted base is the unsafe error.
  - Contracts tagged `ASSUMED` (docstring `requires:` and `ensures:` clauses) join the trusted base as one component that gives their count, pinned to the project commit, whenever the run has any.
  - The axioms `propext`, `Classical.choice` and `Quot.sound` join the trusted base as one component.
  - The checker binary joins the trusted base pinned by its SHA-256, since the CLI cannot otherwise tell which Lean sources the binary was built from.
- `docs/TASKS.md` marks ER-1.2 `done` with this file as its record. `cgv/README.md` documents the option.

## Affected users and systems
- Anyone who reads a pull request that ran CGV. The record states what exit 0 proves, relative to what, and how to rerun it.
- ER-1.4, whose checker will read these records. The record follows EV-1 to EV-12.
- `cgv/src/main.rs`, a new `cgv/src/evidence.rs`, a new `cgv/build.rs`, `cgv/Cargo.toml` (the `sha2` crate), `cgv/tests/`, `cgv/README.md`.

## Constraints
- No change to the checker, the theorems, `BehaviorModel.lean` or the manifest. No protected surface changes, so the change is Tier 2.
- The default output does not change. Without `--evidence-record`, stdout, stderr and exit codes are as before.
- The record states only what CGV knows. Where it cannot pin a component, it says so in the version, for example `unknown` or a `-dirty` suffix, rather than guess.
- Rule 1 of the vision: no part of the record comes from an LLM.

## Open questions
None that block the spec. The concerns that stay open are in the spec under "Concerns flagged".
