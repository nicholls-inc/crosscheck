# Intent: Harden the Docker runs of `dafny_evidence`

Task: ER-1.7. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-dafny-evidence-record-spec.md` (DE-6 and DE-9 amended).

## Problem statement
`dafny_evidence` runs `dafny verify`, `dafny audit` and `dafny --version` in the Dafny image with no network, a memory and CPU limit, and the work tree mounted read-only. The container still runs as root, keeps Docker's default capability set, can gain privileges through a setuid binary, and can start processes without bound. The rerun command the record carries has the same gaps and none of the limits. A record's rerun is meant to be run by an auditor on their own machine, against a work tree they did not write, so the command it hands them should not run the toolchain as root.

## Proposed outcome
- Every Dafny run of `dafny_evidence`, and both `docker run` commands in the rerun command, pass `--cap-drop=ALL`, `--security-opt=no-new-privileges`, `--pids-limit=512` and `--user=65534:65534`.
- One list in `crosscheck/mcp-server/src/docker.ts` holds those flags, and both the tool's runs and the rerun command read it, so the two cannot drift apart.
- DE-6 and DE-9 of the spec name the flags, and the spec records the probe that chose the numbers.
- `docs/TASKS.md` marks ER-1.7 `done` with this file as its record.

## Probe, 2026-10-07
Against `crosscheck-dafny:latest` (Dafny `4.11.0+fcb2042d`), amd64 under emulation on an 11-CPU arm64 Docker host:

| Run | Result |
|---|---|
| `verify`, `audit` and `--version` of a one-lemma file with all four flags | exit 0, "1 verified, 0 errors", "0 findings", the version |
| Inside the container with all four flags | `uid=65534(nobody)`, every capability set 0, `NoNewPrivs: 1`, `pids.max` 256 (the probe's limit) |
| `verify` of a 60-lemma file, default cores, `pids.peak` | 48 |
| The same with `--cpus=1 --memory=512m`, as the tool runs it | 23 |
| The same with `--cores 64` | 186 |
| The same with `--pids-limit=20` | exit 1 |

So 512 leaves more than twice the peak of a 64-way run, and a run that reaches the limit fails rather than passes. Two runs with `--cores 64` hung at 0% CPU until killed, one with the four flags and one with none of them, so the hang comes from the emulated many-core run and not from these flags.

## Affected users and systems
- A caller of `dafny_evidence` gets the same record, with a longer rerun command. A run that needs more than 512 processes or threads now fails, and the tool refuses.
- An auditor who runs a record's rerun command runs Dafny as `nobody` with no capabilities.
- `dafny_verify`, `dafny_compile` and the Lean tools do not change. `dafny_compile` writes into its mount, and on a Linux host `nobody` cannot write to the server's private temp directory, so hardening those tools needs its own design.

## Constraints
- The image does not change, so a record's trusted base and the rerun's image name keep their meaning, and an image built before this change still runs the new command.
- No new dependency.

## Open questions
None that block the spec. Whether `nobody` can read the work tree depends on its file modes on a Linux host, and the spec flags it.
