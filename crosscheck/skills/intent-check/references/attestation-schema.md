# Attestation schema — JSON format and SHA-256 computation

`/intent-check` writes `.assurance/intent-check-attestation.json` on every run. The attestation records what the two-LLM pipeline said, and about which file contents.

## What the attestation is for

The attestation is an advisory record. Its verdict is an LLM's judgement, and rule 1 of Crosscheck's vision (`docs/VISION.md` in the Crosscheck repository) says no guarantee rests on the judgement of an LLM. So the record is never evidence and never a required artefact. No pre-commit hook, CI job, merge rule or reviewer may require the file to exist, require its verdict to be `pass`, or accept it as authority for a change.

It serves two readers:

- **A reviewer** who wants to see what the pipeline said, without rerunning it. The content hash tells them whether the files changed after the run.
- **A human classifying the FP tracker**, who needs the verdict context to fill `human_verdict`.

Earlier versions of this document described a pre-commit hook that rejected a commit unless the attestation recorded `pass`. That hook made an LLM verdict a commit gate, so it is gone. A repository that installed it should remove it through `/protected-surface-amend`.

## Schema

```json
{
  "protected_files":  ["<sorted-paths-relative-to-repo-root>"],
  "content_hash":     "<lowercase-hex-sha256>",
  "verdict":          "pass" | "fail",
  "checked_at":       "<RFC3339 timestamp>",
  "pipeline_output":  {
    "back_translation": "<Section 1 + Section 2 verbatim>",
    "diff_result": {
      "match":              true | false,
      "mismatch_reason":    "<string>",
      "mismatch_category":  "<enum>",
      "confidence_pct":     0-100,
      "confidence_basis":   "<enum>"
    }
  }
}
```

| Field                            | Required | Description                                                                                                                                                       |
|----------------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `protected_files`                | yes      | Alphabetically sorted list of repo-relative paths touched by this pipeline run. Forms the input to the content hash.                                              |
| `content_hash`                   | yes      | Lowercase hex SHA-256 of `concat(read_bytes(f) for f in protected_files)` with **no** delimiter between files. Order matches `protected_files`.                   |
| `verdict`                        | yes      | `pass` if the diff-checker returned `match=true` AND `confidence_pct>=80`; otherwise `fail`. Low-confidence matches are not passes.                                |
| `checked_at`                     | yes      | RFC3339 timestamp in UTC (e.g. `2026-04-24T14:32:10Z`). Lets a reader tell whether the record is older than the files it names.                            |
| `pipeline_output.back_translation` | yes    | Section 1 + Section 2 from the blind back-translator, verbatim. Human-readable so a reviewer can see what the pipeline saw — see "Why `pipeline_output` is a field".     |
| `pipeline_output.diff_result`    | yes      | Full JSON object from the diff-checker after semantic validation (not the raw pre-validation output).                                                            |

Readers should tolerate unknown or extra fields for forward compatibility, but new fields SHOULD NOT be added without updating this doc and the skill's verification checklist.

## Example

```json
{
  "protected_files": [
    "docs/invariants/queue.md",
    "internal/queue/queue.go",
    "internal/queue/queue_invariants_prop_test.go"
  ],
  "content_hash": "9c3b1f1e2d1b4a3a6c4b9e5d2c1b4a3a6c4b9e5d2c1b4a3a6c4b9e5d2c1b4a3a",
  "verdict": "pass",
  "checked_at": "2026-04-24T14:32:10Z",
  "pipeline_output": {
    "back_translation": "### Section 1: Behavioural guarantees\nThe test enforces that enqueue followed by dequeue preserves payload ordering under single-writer concurrency… \n\n### Section 2: Design rationale comments\nqueue_invariants_prop_test.go:452-457\n> Clock values are zeroed before equality comparison because wall-clock drift between reference run and crash run is a test-infrastructure artefact.",
    "diff_result": {
      "match": true,
      "mismatch_reason": "",
      "mismatch_category": "clean_match",
      "confidence_pct": 92,
      "confidence_basis": "rationale-found"
    }
  }
}
```

## SHA-256 computation (exact)

The hash binds the attestation to the specific byte contents of the protected files at the time of the run. After any later edit to one of those files, the record describes contents that no longer exist. The record does not have to be refreshed, because nothing waits for it.

`add-orchestrator` and `/draft-invariants` reuse this algorithm, by this section's heading, for the ADD session marker's `hash_value`. Keep the algorithm stable.

Algorithm:

```python
import hashlib

def content_hash(repo_root: str, protected_files: list[str]) -> str:
    h = hashlib.sha256()
    for rel_path in sorted(protected_files):  # sort BEFORE hashing, always
        with open(f"{repo_root}/{rel_path}", "rb") as f:
            h.update(f.read())  # NO delimiter, NO newline padding
    return h.hexdigest()
```

Invariants of the algorithm:

1. Sort `protected_files` alphabetically using locale-independent ordering (bytewise). The sorted list is what goes into the attestation AND into the hash. The sort is what makes the hash independent of the order files were discovered in.
2. Concatenate raw file bytes with **no delimiter**. Adding a delimiter (newline, null byte, filename prefix) would be fine, but the choice must stay fixed — every reader that recomputes the hash must use the same algorithm. We pick "no delimiter" for simplicity; the file boundaries are implicit in the manifest (`protected_files`) and the hash only needs to be unique per (file-set, byte-content) pair.
3. Hex-encode the digest in lowercase. Case matters for equality comparison.

## Why `pipeline_output` is a field

The hash and the verdict alone say only that a run happened. `pipeline_output` says what the run saw:

- **Human review.** When a reviewer wants to know why the pipeline returned its verdict, they can read the back-translation and the diff result without rerunning the pipeline.
- **Tracker reconciliation.** When a human fills the `human_verdict` column in the FP tracker, they need the verdict context. The attestation and the tracker row together give them enough to decide.

## Interaction with `/protected-surface-amend`

If a reviewer decides that a `fail` verdict reflects an intentional spec evolution rather than a drift, they amend the invariant prose (or the covering test) through `/protected-surface-amend`. The governance note cites the roadmap item and the human decision, never the attestation. Nobody has to rerun `/intent-check` to get a `pass` before the change merges, and hand-editing the attestation serves no purpose, because nothing reads it as authority.
