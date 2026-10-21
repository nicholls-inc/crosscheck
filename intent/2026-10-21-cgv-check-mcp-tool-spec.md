# Spec: The MCP tool `cgv_check`

Intent: `intent/2026-10-21-cgv-check-mcp-tool.md`. Governing roadmap item: ER-1. Task: ER-1.20.

`cgv_check` runs CGV's `contracts check` on a committed Python project and returns CGV's findings JSON and CGV's evidence record. The record is the file CGV writes under CR-1 to CR-8 of `intent/2026-10-06-cgv-evidence-record-spec.md`. The tool does not build, edit or strengthen it.

## Requirements

- **CK-1. Inputs.** The zod schema in `crosscheck/mcp-server/src/tools/cgvCheck.ts` is the input shape:
  - `repoPath`, a string: an absolute path inside a git work tree.
  - `path`, a string: the Python project to check, relative to the work tree's top level with `/` separators. It is `.` for the top level, or a path with no empty, `.` or `..` segment and no `\`.
  - `baseline`, an optional string: a git revision. It must not be blank, hold white space, or start with `-`.

  No input may hold a NUL byte, which `execFile` cannot pass and would otherwise reject the whole call.

  The tool refuses each input that breaks these rules, with one error per broken rule, before it runs git or CGV.
- **CK-2. The binaries.** The CLI is the file that `CGV_CONTRACTS` names, or else the first `crosscheck-contracts` on `PATH`. The checker is the file that `CGV_CHECKER` names, or else the first `contract-graph-checker` on `PATH`. A variable that is set must name an executable file. When it does not, the tool refuses and names the variable, and it does not fall back to `PATH`. When a binary is not found, the tool refuses with one error per missing binary, and each ends with this command, which builds both from a fresh clone and prints the two variables to set:

  ```
  git clone https://github.com/nicholls-inc/crosscheck && cd crosscheck/cgv && cargo build --release && (cd prover && lake build contract-graph-checker) && echo "CGV_CONTRACTS=$PWD/target/release/crosscheck-contracts CGV_CHECKER=$PWD/prover/.lake/build/bin/contract-graph-checker"
  ```

  The error also says that both variables must be set in the environment Claude Code starts in, or both binaries put on `PATH`, and that the build needs `cargo` and `elan`.
- **CK-3. The commit.** As DE-2 and DE-3 of `intent/2026-10-06-dafny-evidence-record-spec.md`: the tool reads the top level with `git rev-parse --show-toplevel` and `commit` with `git rev-parse HEAD`, and refuses when either fails. It refuses, with one error per line, when `git status --porcelain --untracked-files=all` lists any change anywhere in the work tree, or `git ls-files -v` marks a file `assume-unchanged` or `skip-worktree`.
- **CK-4. The project.** For `path` other than `.`, `git ls-tree <commit> -- <path>` must list a tree, or a regular file (mode `100644` or `100755`) whose name ends in `.py`. The tool refuses `not committed: <path>` when it lists nothing, and refuses a symbolic link, a submodule and any other file.
- **CK-5. The baseline run.** When `baseline` is given:
  - the tool resolves it with `git rev-parse --verify --quiet --end-of-options <baseline>^{commit}` and refuses when that fails;
  - CK-4's check, run at the baseline commit, must pass, and the tool refuses when `path` does not exist there;
  - it checks the baseline commit out with `git -c core.hooksPath=/dev/null worktree add --detach <tmp>/baseline-tree <baseline commit>`, and refuses when that fails. This is the checkout that made HEAD's work tree, with the same smudge filters and end-of-line rules, so the two runs read the same bytes for the same committed content. It does not use `git archive`, which applies `.gitattributes` `export-ignore` and `export-subst` and so drops or rewrites baseline files that HEAD's work tree keeps. No hook runs on this checkout;
  - it runs `crosscheck-contracts contracts check <tmp>/baseline-tree/<path> --lean-checker <checker> --output-db <tmp>/baseline.sqlite --write-baseline <tmp>/baseline.json` from `<tmp>/baseline-tree`.

  The run must exit 0 or 1 and write the baseline file. Otherwise the tool refuses with the exit code and CGV's stderr.

  On every path out of this step, success, refusal or a thrown error, the tool runs `git worktree remove --force <tmp>/baseline-tree`, deletes that directory if it remains,, so the repository lists no extra work tree afterwards. It does not run `git worktree prune`, which is repository-wide and would also drop the entries of the caller's other worktrees whose directories are temporarily missing. `remove` deregisters the checkout, and fails harmlessly when `worktree add` registered nothing. A process killed between `add` and this cleanup leaves one entry, which `git worktree prune` removes when the caller runs it.
- **CK-6. The record run.** The tool runs `crosscheck-contracts contracts check <top level>/<path> --lean-checker <checker> --output-db <tmp>/head.sqlite --evidence-record <tmp>/record.json` from the top level. Its stdout must parse as a JSON object, or the tool refuses.
  - Exit 0: the file at `<tmp>/record.json` must parse as a JSON object whose `format` is `evidence-record/1` and whose `commit` is `commit`. That object is the record. The tool refuses when the file is missing or fails either test.
  - Exit 1: the record is `null`. CGV writes no record for a run with an error (CR-2).
  - Any other exit, a timeout or a signal: the tool refuses with the exit and CGV's stderr. CGV's own refusals under CR-3 are among them.
- **CK-7. The comparison run.** When `baseline` is given, the tool then runs `crosscheck-contracts contracts check <top level>/<path> --lean-checker <checker> --output-db <tmp>/diff.sqlite --baseline <tmp>/baseline.json` from the top level. It must exit 0 or 1 and print a JSON object, or the tool refuses.
- **CK-8. Nothing moved.** After the last CGV run the tool reads HEAD and the work tree state again, as DE-13 does. It refuses when HEAD is not `commit` or CK-3's checks now list a change.
- **CK-9. The result.** On success the tool returns `{ success: true, commit, baseline, exitCode, findings, record }`:
  - `commit` is HEAD, and `baseline` is the resolved baseline commit or `null`;
  - `findings` is the JSON object of CK-7's run when a baseline is given, and of CK-6's run otherwise, parsed and not changed. Fields CGV adds later, such as `counterexample` in #133, pass through;
  - `exitCode` is that run's exit code, 0 or 1;
  - `record` is CK-6's record or `null`. So with a baseline, `exitCode` can be 0 while `record` is `null`: no error is new, and an error the baseline holds still stops CGV from writing a record.
- **CK-10. A refusal.** Every refusal returns `{ success: false, errors }` with at least one error. The tool returns a refusal as data at the MCP layer and does not throw.
- **CK-11. Limits.** Each CGV run is stopped after 10 minutes and refused as a timeout. CGV's stdout and stderr may each be up to 512 MiB.
- **CK-12. Cleanup.** The temporary directory, which holds the baseline checkout, the databases, the baseline and the record, has the prefix `cgv-` and is removed on every path out of the tool once it exists. The tool writes nothing inside the work tree. While the baseline checkout exists, git records it under the repository's `worktrees` directory, and CK-5's cleanup removes that entry.
- **CK-13. Tests.**
  - Unit tests cover CK-1 and CK-2.
  - MCP-layer tests call the tool through `createServer` with a stand-in CLI. They cover the result of CK-9 with and without a baseline, and one refusal for each of CK-1 to CK-8. One commits a `.gitattributes` that marks a `.py` file `export-ignore` and another `export-subst`, and checks that the baseline run sees both files as committed. Others check that no extra work tree and no baseline checkout remain after a baseline run that completes and one that fails.
  - An integration test runs the real CGV binaries on fixtures from `cgv/test_fixtures/`. It checks that the record of a clean project passes `scripts/check-evidence-record.mjs` and that its rerun command exits 0. It checks that a project with an error returns exit 1 and no record, that a baseline with the same errors returns exit 0, and one without them exit 1, and that a baseline whose `.gitattributes` marks the fixture's `models.py` `export-ignore` still holds its errors, so the result is exit 0 with no new error. It is skipped, with a message that names the variables and the build command, when the binaries are not found. The mcp-server CI job does not build or receive the CGV binaries (the CGV CI job builds them but runs no mcp-server tests), so this test runs locally only and always skips in CI. Run it from the repository root with `(cd cgv && cargo build --release && cd prover && lake build contract-graph-checker) && cd crosscheck/mcp-server && npx vitest run src/__tests__/integration/cgvCheck.cgv.integration.test.ts`. ER-1.22 adds it to CI.

## Concerns flagged, not resolved here

- **CGV runs on the host.** The Dafny runs of `dafny_evidence` are in a container with no network (DE-6). CGV's extractor parses Python with a Rust parser and does not run it, and the checker reads a SQLite file, so neither executes project code. Running CGV in a container is not yet reached. The property that blocks it is that no CGV image exists. The open question is whether the record's trusted base should then name the image.
- **git reads its environment.** The tool's git calls, and CGV's, inherit `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`, as ER-1.13 says of `dafny_evidence`. A set variable makes CK-3, CK-4 and CK-8 read another repository. Clearing them for both tools is ER-1.13's work.
- **The second check is not atomic.** CK-8 runs after the comparison run, and CGV's own recheck (CR-3) runs before it writes the record. A change made and reverted between CGV's recheck and CK-8 is not seen. The property that blocks it is that git offers no lock on the work tree. The open question is the same as CR's: a hash of the files the extractor read.
- **The options are CGV's defaults.** `--overrides`, `--exclude`, `--django-version`, `--max-states` and `--max-states-per-edge` are not inputs, so a project that needs one cannot be checked through the tool. Passing them through is not yet reached. Each must reach both runs and the export in the same form, and `--overrides` must also be checked as committed.
- **The record is not written into the repository.** `dafny_evidence` writes its record when asked (DE-11). This tool returns it. Where a CGV record lives in a pull request is ER-1.21's call.
- **The rerun names local paths.** The record's rerun command names the checker by its absolute path on the machine that ran the tool (CR spec concern). The tool does not change it.
