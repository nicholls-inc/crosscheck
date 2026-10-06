# Spec: `dafny_evidence`, an evidence record from the Dafny pipeline

Intent: `intent/2026-10-06-dafny-evidence-record.md`. Governing roadmap item: ER-1. Task: ER-1.3.

This spec defines the MCP tool `dafny_evidence`. It runs `dafny verify` and `dafny audit` on a committed Dafny file and emits a version 1 evidence record (`intent/2026-10-06-evidence-record-spec.md`) with one `proved` claim, or refuses. Rules DE-1 to DE-11 state what it does.

## Input

| Field | Type | Meaning |
|---|---|---|
| `repoPath` | string | An absolute path inside a git work tree. The tool works from the work tree's top level, `git rev-parse --show-toplevel`, called "the root" below. |
| `file` | string | The path of the `.dfy` file, relative to the root, with `/` separators. |
| `statement` | string | What is claimed, in plain language. It becomes the claim's `statement`. |
| `requirement` | string or `null` | The requirement the claim traces to, or `null`. It becomes the claim's `requirement`. |
| `theorems` | array of strings | The names of the lemmas, methods or functions whose contracts prove the statement. A name may be qualified by its modules, as `M.Name`. It becomes `basis.theorems`. |
| `outputPath` | string, optional | Where to write the record. A relative path is resolved against the root. |

## Output

`{ "success": boolean, "errors": string[], "record": object or null, "writtenTo": string or null }`.

`success` is `true` only when the tool built the record and, if `outputPath` was given, wrote it. `errors` is empty exactly when `success` is `true`. `record` is the record when one was built, and `null` when the tool refused before building one. `writtenTo` is the absolute path written, or `null`.

## Rules

- **DE-1. Inputs.** The tool refuses when `repoPath` is not absolute, when `file` is absolute, has a `..`, `.` or empty segment, uses `\`, or does not end in `.dfy`, when `statement` is empty after trimming white space, when `requirement` is a string that is empty after trimming, when `theorems` is empty, or when a theorem name does not match `^[A-Za-z_][A-Za-z0-9_'?]*(\.[A-Za-z_][A-Za-z0-9_'?]*)*$`. Each problem is one error.
- **DE-2. A git work tree.** The tool refuses when `git rev-parse --show-toplevel` or `git rev-parse HEAD` fails in `repoPath`. The record's `commit` is the output of `git rev-parse HEAD`.
- **DE-3. A clean tree.** The tool refuses when `git status --porcelain` fails or prints anything in the root, and names each path it printed. That covers staged, unstaged and untracked files, and leaves out ignored ones. So the files Dafny reads are the files at `commit`, including any file that `file` includes.
- **DE-4. A committed file.** The tool refuses when `git ls-files --error-unmatch -- <file>` fails in the root.
- **DE-5. The theorems are declared.** The tool refuses when a theorem's last segment is not declared in the file, or a qualifier before it is not declared as a module. A name `N` is declared when the file has `lemma`, `method`, `function` or `predicate`, white space, any number of attributes `{:...}` each followed by white space, and then `N` not followed by a letter, digit, `_`, `'` or `?`. A qualifier `Q` is declared when the same holds with `module` in place of the four keywords. Variants such as `ghost function`, `twostate lemma` and `least predicate` match, because they end in one of the four keywords.
- **DE-6. Verify.** The tool runs `dafny verify /work/<file>` in the Dafny image (`DAFNY_DOCKER_IMAGE`, default `crosscheck-dafny:latest`) with the root mounted at `/work` and no network. It refuses unless the run exits 0 and does not time out. Dafny 4.11.0 fails the run on warnings, so `assume`, a bodiless lemma with no `{:axiom}`, `{:verify false}` and an `{:extern}` with an `ensures` each refuse here.
- **DE-7. Audit.** The tool runs `dafny audit /work/<file>` in the same way. It refuses unless the run exits 0 and its standard output and standard error together contain `Dafny auditor completed with 0 findings`. This is the check that catches `{:axiom}`, which DE-6 passes.
- **DE-8. Trusted base.** The tool runs `dafny --version` in the image and takes its trimmed standard output as `<dafny version>`. It refuses unless that matches `^\d+\.\d+\.\d+\S*$`. It reads the image ID with `docker image inspect --format {{.Id}} <image>`, and refuses unless the trimmed output is non-empty. The trusted base is, in this order:
  1. `{"component": "Dafny verifier", "version": "<dafny version>"}`;
  2. `{"component": "Z3 solver shipped with the Dafny release", "version": "Dafny <dafny version>"}`;
  3. `{"component": "Dafny Docker image <image>", "version": "<image ID>"}`.
- **DE-9. Rerun.** `rerun.exit_code` is 0, and `rerun.command` is, with `<i>` the image and `<f>` the path `/work/<file>`, each single-quoted for a POSIX shell:

  ```
  docker run --rm --network=none -v "$PWD":/work <i> verify <f> && docker run --rm --network=none -v "$PWD":/work <i> audit <f> 2>&1 | grep -q 'Dafny auditor completed with 0 findings'
  ```

  Run from the root at `commit`, it exits 0 exactly when DE-6 and DE-7 pass, so the tool and an auditor's rerun apply the same checks. Single quoting writes `'` as `'\''`.
- **DE-10. The record.** The record is `{"format": "evidence-record/1", "commit": <DE-2>, "claims": [<claim>]}`. The claim's fields are, in order, `id`, `statement`, `requirement`, `strength`, `basis`, `trusted_base` and `rerun`. `id` is `dafny-` followed by the file path without `.dfy`, lowercased, with each run of characters other than `a` to `z` and `0` to `9` replaced by one `-`, and with leading and trailing `-` removed. When that leaves nothing, `id` is `dafny`. `statement` is the input trimmed. `requirement` is the input, trimmed when it is a string. `strength` is `"proved"`. `basis` is `{"theorems": <theorems>}` in the order given. The record satisfies EV-1 to EV-12.
- **DE-11. Output file.** When `outputPath` is given, the tool writes the record as JSON with two-space indentation and a final newline. A write that fails makes `success` `false`, with the error, and still returns the record. Given the same inputs, commit and image, the tool writes the same bytes.

## Tests

The unit tests cover DE-1, DE-5, DE-9 and DE-10 as pure functions, including a file path with a `'` and a space in DE-9. An integration test makes a real git repository in a temporary directory, replaces Docker with a stub, and covers DE-2, DE-3, DE-4, DE-6, DE-7, DE-8 and DE-11: each refusal, and a success whose record is compared field by field with a literal. An end-to-end test, run by `npm run test:e2e` against the real image, emits a record for a proved lemma, runs its rerun command in the root and checks that it exits 0, and refuses an `{:axiom}` lemma under DE-7.

## Concerns flagged, not resolved here

- **The statement is the caller's.** The tool checks that each named theorem verifies and is declared. It does not check that the theorems' contracts say what `statement` says. Usually an LLM drafts both. Rule 1 of `docs/VISION.md` makes the statement a draft for a person to review, and the record cannot show that a person did. The property that blocks a check is a formal link between the statement and the contracts. The open question is the controlled English of rule 4.
- **DE-5 is lexical.** A declaration inside a comment or a string satisfies it. A parser-backed check, for example from Dafny's own symbol output, is not yet reached. The property that blocks it is a Dafny command that lists declarations in a stable format, and the open question is whether `dafny` offers one.
- **The rerun names a tag, not a digest.** The rerun command uses the image name, so an auditor who built `crosscheck-dafny:latest` later may run a different image. The trusted base records the local image ID, and the Dockerfile pins Dafny 4.11.0, but the base image is `ubuntu:22.04` with no digest. The evidence record spec flags the toolchain pinning, and it stays open.
- **Z3's own version is not read.** The Dafny release zip bundles Z3, and the record names the Dafny release rather than the Z3 build.
- **The Docker runtime and the host are not in the trusted base.** On an arm64 host the amd64 image runs under emulation.
- **Any warning refuses.** DE-6 inherits Dafny's default of failing on warnings, so a missing trigger or "unusual indentation" refuses as surely as an `assume`. That is the safe direction. On 2026-10-06 it refuses all three tracked specs under `crosscheck/mcp-server/specs/`, which also hold `assume false` and one verification error (issue #81).
- **A record written into the tree dirties it.** DE-3 then refuses the next run until the record is committed, ignored, or written outside the tree.
- **No skill calls the tool yet.** `/generate-verified` and `/spec-iterate` are Class A protected surfaces, and wiring them is a separate change, issue #80.
