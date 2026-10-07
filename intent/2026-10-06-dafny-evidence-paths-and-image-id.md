# Intent: Tighten `dafny_evidence` output and include paths, and pin the rerun to the image ID

Task: ER-1.8. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-dafny-evidence-record-spec.md` (DE-9, DE-11 and DE-12 amended, and the concern "The rerun names a tag, not a digest" rewritten).

## Problem statement
Three gaps remain in `dafny_evidence` after ER-1.3 and ER-1.7.

- **Output path.** DE-11 lets a record be written to any new path inside the work tree outside `.git`. That includes `.github/workflows/x.yml`, `.husky/pre-commit` and `.claude/settings.local.json`, where a tool or a hook reads the file as configuration, and a name with no `.json` that no reader treats as a record.
- **Include path.** DE-12 accepts an include whose path does not end in `.dfy`. On 2026-10-06 Dafny 4.11.0 verified `include "Lib.txt"` as Dafny source, and parsed `include "Lib.doo"` as source too. So a proof can rest on a file that neither a reviewer nor a `*.dfy` search sees as Dafny.
- **Rerun image.** DE-9's rerun command names the image by its tag, `crosscheck-dafny:latest`. The trusted base records the image ID, but the command does not use it. Anyone who rebuilds or retags the image runs a different toolchain under the same command, and nothing tells them. Rule 3 of `docs/VISION.md` asks that a result rerun from pinned inputs, and a tag is not a pinned input.

## Probe, 2026-10-06
Docker 29.4.0 on an arm64 host, overlay2 storage, against `crosscheck-dafny:latest` (Dafny `4.11.0+fcb2042d`):

| Run | Result |
|---|---|
| `docker run` with the sandbox flags and the image ID `sha256:ccd363af...` in place of the tag, `--version` | exit 0, the version |
| `docker run` with an ID no local image has | exit 125, "No such image", and no pull |
| A `FROM scratch` image: read its ID, `docker save`, `docker rmi`, `docker load`, read the ID again | the same ID |
| The same Dockerfile built again with `--no-cache` | a different ID |
| `verify` of a file with `include "Lib.txt"`, where `Lib.txt` holds a predicate the file uses | "1 verified, 0 errors" |
| `verify` of a file with `include "Lib.doo"`, where `Lib.doo` is not Dafny | a parse error in `Lib.doo` |

So a rerun that names the ID runs exactly the recorded image or exits 125 before Dafny starts. `docker save` and `docker load` carry the ID to another machine. A rebuild, even of an unchanged Dockerfile, does not.

## Proposed outcome
- `dafny_evidence` refuses an `outputPath` that does not end in `.json`, and one that passes through a directory whose name starts with `.`, before any Dafny run and again before the write. The second rule replaces the `.git` rule, which it covers.
- `dafny_evidence` refuses an include whose path does not end in `.dfy`.
- The rerun command names the image by the ID that the trusted base records. An auditor on another machine runs that command after `docker load` of the image that the record's author saved. An auditor who builds the image instead gets exit 125, and must edit the command to name their own image, which is then a different trusted base, and they know it.
- The spec's concern on the rerun image says what an auditor runs, and that a rerun with no help from the author is not yet reached.
- `docs/TASKS.md` marks ER-1.8 `done` with this file as its record.

## Affected users and systems
- A caller of `dafny_evidence` that passed an output path under a dot directory or without `.json`, or a file with a non-`.dfy` include, is refused. No tracked spec in this repository does either.
- An auditor who reruns a record on the machine that wrote it sees no change. On another machine the command fails with exit 125 until they load the author's image.
- `dafny_verify`, `dafny_compile`, the Lean tools and the evidence record format do not change. The trusted base entries do not change.

## Constraints
- No new dependency, and the image does not change.
- A failure must fail the rerun, not pass it: an unknown ID exits non-zero.

## Open questions
None that block the spec. A rerun that needs no help from the author needs a published image whose build pins its base, which is the toolchain pinning question the evidence record spec leaves open. The spec flags it.
