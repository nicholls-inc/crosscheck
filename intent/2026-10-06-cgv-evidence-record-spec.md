# Spec: CGV emits an evidence record

Intent: `intent/2026-10-06-cgv-evidence-record.md`. Governing roadmap item: ER-1. Task: ER-1.2.

This spec adds the requirements CR-1 to CR-8 for `crosscheck-contracts contracts check`. The record they describe is version 1 of the format in `intent/2026-10-06-evidence-record-spec.md`, and every record CGV writes must satisfy EV-1 to EV-12 of that spec.

## Requirements

- **CR-1. The option.** `contracts check` takes `--evidence-record PATH`. Without it, the command's output and exit codes do not change.
- **CR-2. Only exit 0 leaves a record.** When the run exits 0, CGV writes the record to PATH, replacing any file there. When a run that gets past argument parsing exits with any other code, including a refusal under CR-3, CGV removes any file at PATH and writes none. An invocation that the argument parser rejects (exit 2, before any run starts) leaves a file at PATH untouched. If the record cannot be written, the run exits 2 with a message that names PATH.
- **CR-3. The commit.** Before extraction, CGV runs git in the directory of the checked path (the path itself, or its parent for a single file). It exits 2, with a message on stderr and nothing on stdout, and does not run the checker, when:
  - the path is not inside a git work tree;
  - `git status --porcelain --untracked-files=all` lists any change under the checked path or the `--overrides` file;
  - `git ls-files --others --ignored --exclude-standard` lists a `.py` file under the checked path that the run would analyse, that is, one the extractor's file walk finds and no `--exclude` glob removes. An ignored file the run does not read, such as a `.log` file, a `.py` file under `__pycache__/`, or a `.py` file an `--exclude` glob removes, does not refuse the run;
  - `git ls-files --others --ignored --exclude-standard` lists the `--overrides` file, so the commit does not pin an input the run reads;
  - the `--overrides` file is outside that work tree.

  Otherwise `commit` is the output of `git rev-parse HEAD`, which is 40 lowercase hex characters.

  After the checker exits 0, and immediately before writing the record, CGV runs the same `git status` and ignored-file checks again and reads `git rev-parse HEAD` again. It exits 2 and writes no record when either check now fails or HEAD differs from `commit`. So the order is: remove any file at PATH, check, extract, run the checker, check again, write.
- **CR-4. One claim.** The record has one claim:
  - `id` is `cgv-data-paths`;
  - `requirement` is `null`;
  - `strength` is `proved`, and `basis` is `{"theorems": ["ContractGraph.runChecker_sound_all"]}`;
  - `statement` says that at every hop of every checked data path from a function to a model field, the source's guarantees imply the target's requirements, for the contracts the CGV extractor derived from the checked path at that commit. It gives the number of warnings and says that a warning marks a requirement CGV could not show, which the theorem does not cover.
- **CR-5. The trusted base.** In this order, with these versions:

  | `component` | `version` |
  |---|---|
  | `Lean 4 kernel` | the toolchain in `cgv/prover/lean-toolchain` when the CLI was built, such as `leanprover/lean4:v4.28.0` |
  | `Lean axioms propext, Classical.choice and Quot.sound` | the same toolchain |
  | `Lean compiler and runtime that built contract-graph-checker` | the same toolchain |
  | `contract-graph-checker binary` | `sha256:` and the lowercase hex SHA-256 of the checker file that the run executed |
  | `CGV Rust extractor and command line` | the CGV build commit (CR-6) |
  | `CGV Translation.lean` | the CGV build commit |
  | `CGV BehaviorModel.lean` | the CGV build commit |
  | `Docstring requires: and ensures: contracts in the checked project, tagged ASSUMED and checked by no tool: N` | the project commit. Present only when N, the number of contracts the extractor tagged `ASSUMED`, is at least 1 |

- **CR-6. The CGV build commit.** `cargo build` records `git rev-parse HEAD` of the CGV source, with the suffix `-dirty` when `git status --porcelain` lists a change under `cgv/`. When git cannot answer, the version is `unknown`.
- **CR-7. The rerun.** `rerun.exit_code` is 0. `rerun.command` runs from the root of the project's work tree:

  ```
  crosscheck-contracts contracts check <path> --django-version <v> [--overrides <path>] [--exclude <glob>]... [--allow-parse-errors] [--max-states <n>] [--max-states-per-edge <n>] --lean-checker <checker>
  ```

  Each `<path>` is relative to the work-tree root (`.` for the root itself). `<checker>` is the absolute path of the checker the run executed. An option appears only when the run was given it, except `--django-version`, which always appears. Each argument is a word of POSIX shell. An argument with any character outside `A-Za-z0-9_./=:@%+-` is single-quoted, with each `'` written as `'\''`.
- **CR-8. Tests.** `cgv/tests/e2e_evidence.rs` runs the built CLI in a scratch git repository with a stand-in checker, and covers:
  - exit 0 writes a record whose fields are exactly those of EV-1 and EV-5, whose `commit` is the scratch repository's `HEAD`, and whose claim, basis and trusted base match CR-4 and CR-5;
  - the rerun command, run with `sh -c` from the work-tree root with the CLI on `PATH`, exits 0;
  - an argument with a space and a `'` survives the round trip through the rerun command;
  - exit 1 removes a record left at PATH by an earlier run and writes none;
  - an untracked file under the checked path, a modified tracked file, and a path outside any git repository each exit 2 without running the checker, and leave no record;
  - an ignored `.py` file under the checked path exits 2 without running the checker and leaves no record, while an ignored `.log` file, an ignored `.py` file under `__pycache__/`, an ignored `.py` file an `--exclude` glob removes, and an ignored `.py` file outside the checked path do not refuse the run;
  - an ignored `--overrides` file exits 2 without running the checker and leaves no record, while a tracked one does not refuse the run;
  - a file added under the checked path while the checker runs, and a commit made while the checker runs, each exit 2 and leave no record;
  - a docstring `ensures:` clause adds the `ASSUMED` component with its count, and a project without one has no such component;
  - a run without `--evidence-record` creates no file.

## Concerns flagged, not resolved here

- **The binary and the sources are pinned separately.** The checker binary is pinned by its hash. `Translation.lean` and `BehaviorModel.lean` are pinned to the commit of the CLI, which is the commit of the checker's sources only when both were built from one checkout. The property that blocks a stronger pin is that the checker does not report the commit it was built from. The open question is whether the checker should print its build commit, so that the record can name it.
- **The rerun names a local path.** `<checker>` is an absolute path on the machine that ran CGV, so an auditor on another machine edits the command. Rerunning from pinned inputs on any machine is not yet reached. The property that blocks it is a pinned, published checker binary. The open question is how CGV publishes one.
- **The second check is not atomic with the write.** CR-3 checks the checkout again just before the record is written, which closes the window that spans extraction and the checker run. A change made between that check and the rename that writes the record is not yet reached. The property that blocks it is that git offers no lock that holds the work tree still. The open question is whether the record should carry a hash of the files the extractor read, so that a reader can check them against the commit.
- **Inputs the commit does not pin.** A symlink that the extractor's file walk follows, to a directory or `.py` file outside the work tree, is analysed but not pinned: git tracks only the link. A file that git is told to ignore changes for (`assume-unchanged`, `skip-worktree`) or a submodule setting that hides changes can also read as clean. Both are not yet reached. The property that blocks them is that `git status` does not cover content the extractor reads through a link or behind a flag. The open question is whether the record should carry a hash of the files the extractor read, which would also answer the atomicity concern above.
- **Skipped files are not in the record.** With `--allow-parse-errors` the extractor skips a file that does not parse and prints a warning on stderr. The record still says `proved`, and its rerun command carries the flag, but the skipped files are not named. Naming them is not yet reached. The property that blocks it is that the extractor does not return the skipped files to `main`. The open question is whether the record should refuse a run that skipped a file or state the count in the claim's statement.
- **The build commit can be stale.** `cgv/build.rs` reads the commit and the `-dirty` mark once, and Cargo reruns it only when `src`, `prover/ContractGraph`, `Cargo.toml`, `build.rs` or HEAD change. A change elsewhere under `cgv/` can leave the recorded mark out of date, and a `-dirty` or `unknown` build commit is still recorded with strength `proved`. Detecting it is not yet reached. The property that blocks it is that Cargo cannot rerun a build script on every `cgv/` file without also rerunning on build output. The open question is whether the record should refuse a `-dirty` or `unknown` build commit, which would stop development builds from writing records.
- **No rerun reproduces the hash.** EV-13 forbids the record checker from running commands, and the rerun reproduces only the exit code. Nothing checks that the checker at `<checker>` still has the recorded hash.
- **The theorem's other qualifiers are in prose.** The claim's statement and `cgv/README.md` ("What exit 0 promises") give them. The record has no field for them.
