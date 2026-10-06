# Spec: The evidence record format, version 1

Intent: `intent/2026-10-06-evidence-record.md`. Governing roadmap item: ER-1. Task: ER-1.1.

This spec defines version 1 of the evidence record and the rules EV-1 to EV-13 that a checker applies to it. ER-1.2 (CGV emits a record), ER-1.3 (one Crosscheck pipeline emits a record) and ER-1.4 (a deterministic checker) implement it. This task changes no code.

## Terms

- **Record.** One JSON document about one commit. It lists claims.
- **Claim.** One statement about the code at that commit, with its strength, the facts that strength needs, its trusted base, and the command that reruns it.
- **Strength.** One of the four strengths in rule 7 of `docs/VISION.md`:
  - `proved`: a machine-checked proof, relative to the trusted base;
  - `tested`: checked on a stated number of inputs;
  - `observed`: seen in the world, for example by an acceptance test against a real system or by production monitoring;
  - `judged`: decided by a named person.
- **Trusted base.** The components whose correctness the claim assumes and does not check, each with a pinned version. For a CGV proof these are the Lean kernel, the Rust extractor, and `Translation.lean`.
- **Rerun.** A shell command that a reader runs from the root of the repository at the record's commit, and the exit code it must produce for the claim to stand.

## The shape

A record is a JSON object with exactly three fields.

| Field | Type | Meaning |
|---|---|---|
| `format` | string | Always `"evidence-record/1"`. |
| `commit` | string | The full 40-character lowercase hex SHA of the commit the claims are about. |
| `claims` | array of claims | At least one claim. |

A claim is a JSON object with exactly seven fields.

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Names the claim within the record. Lowercase letters, digits and `-`, starting with a letter or digit. |
| `statement` | string | What is claimed, in plain language, for a reader who will not open the code. |
| `requirement` | string or `null` | The approved requirement the claim traces to, as a repository path with an optional `#anchor`. `null` says the claim traces to no approved requirement yet. The field is required, so that a missing trace is stated rather than left out. |
| `strength` | string | `"proved"`, `"tested"`, `"observed"` or `"judged"`. |
| `basis` | object | The facts the strength needs. Its fields depend on `strength`; see below. |
| `trusted_base` | array of components | At least one component. |
| `rerun` | object | `{"command": <string>, "exit_code": <integer>}`. |

`basis` has exactly the fields its strength names.

| `strength` | `basis` fields |
|---|---|
| `proved` | `theorems`: a non-empty array of the fully qualified names of the theorems that prove the statement. |
| `tested` | `cases`: a positive integer, the number of inputs checked. `seed`: a string that reproduces the sampled inputs, or `null` when the inputs are fixed and not sampled. |
| `observed` | `source`: a non-empty string that says what was observed, where, and when. |
| `judged` | `judge`: the name of the person who judged. `date`: the date of the judgment, as `YYYY-MM-DD`. |

A trusted-base component is `{"component": <string>, "version": <string>}`. The version pins the component: a tag, a release, a commit SHA, or an image digest.

The record has no field for an overall verdict. Each claim keeps its own strength. How strengths combine into one verdict is an open question of `docs/VISION.md` (see "Concerns flagged").

## Rules

A checker that implements this spec reads one record and applies EV-1 to EV-12. EV-13 states its interface.

- **EV-1. Closed object.** The record is a JSON object whose fields are exactly `format`, `commit` and `claims`. A missing field or any other field is an error. A misspelled field such as `"strenght"` therefore fails rather than reading as absent.
- **EV-2. Format.** `format` is the string `"evidence-record/1"`.
- **EV-3. Commit.** `commit` matches `^[0-9a-f]{40}$`.
- **EV-4. Claims.** `claims` is a non-empty array.
- **EV-5. Closed claim.** Each claim is an object whose fields are exactly the seven in the table above.
- **EV-6. Identifier.** `id` matches `^[a-z0-9][a-z0-9-]*$`, and no two claims in a record share an `id`.
- **EV-7. Statement.** `statement` is a string that is not empty after trimming white space.
- **EV-8. Requirement.** `requirement` is `null`, or a string that is not empty after trimming white space.
- **EV-9. Strength.** `strength` is present and is one of `"proved"`, `"tested"`, `"observed"` and `"judged"`. Any other value is an error, including `"verified"`, which rule 7 of the vision rules out.
- **EV-10. Basis.** `basis` is an object whose fields are exactly those of its strength, with these types:
  - `theorems` is a non-empty array of strings, none of them empty;
  - `cases` is an integer of at least 1, and `seed` is a non-empty string or `null`;
  - `source` is a non-empty string;
  - `judge` is a non-empty string, and `date` matches `^\d{4}-\d{2}-\d{2}$` and is a real calendar date.
- **EV-11. Trusted base.** `trusted_base` is a non-empty array. Each element is an object whose fields are exactly `component` and `version`, both non-empty strings. No two elements share a `component`.
- **EV-12. Rerun.** `rerun` is present and is an object whose fields are exactly `command` and `exit_code`. `command` is a string that is not empty after trimming white space. `exit_code` is an integer from 0 to 255.
- **EV-13. Checker interface.** The checker takes the path of a record. It reads no network, calls no LLM, and runs no command from the record.
  - It exits 0 when the record satisfies EV-1 to EV-12.
  - It exits 1 when the record breaks a rule. It reports every broken rule, not only the first. Each problem is one line that names the rule, the claim `id` where there is one (or the claim's index when the `id` itself is broken), and the field.
  - It exits 2 when it cannot read the file or the file is not JSON.

EV-9 and EV-12 are the two rejections the roadmap's acceptance for ER-1 names: a claim with no strength, and a claim with no rerun command.

## Tests the checker must have

ER-1.4 adds the checker. Its tests include, at least, one record per case below, each asserting the exit code and the literal rule name in the output:

- a valid record with one claim of each strength exits 0;
- a claim with no `strength` exits 1 and names EV-5 and EV-9;
- a claim with `"strength": "verified"` exits 1 and names EV-9;
- a claim with no `rerun`, and a claim whose `rerun.command` is `"  "`, each exit 1 and name EV-12;
- a `tested` claim whose `basis` has `theorems` instead of `cases` and `seed` exits 1 and names EV-10;
- a `judged` claim with `"date": "2026-02-30"` exits 1 and names EV-10;
- two claims with the same `id` exit 1 and name EV-6;
- a record with an extra top-level field exits 1 and names EV-1;
- `"format": "evidence-record/2"` exits 1 and names EV-2;
- an empty `claims` array exits 1 and names EV-4;
- a record with three broken claims reports all three;
- a file that is not JSON exits 2.

## Worked examples

These are targets for ER-1.2 and ER-1.3, not output of either tool today. Commands, SHAs and paths marked `<...>` are placeholders.

### CGV: exit 0 on a project

`runChecker_sound_all` in `cgv/prover/ContractGraph/Main.lean` proves that exit 0 implies every data path is sound with respect to the translated contracts. The proof says nothing about whether the extractor read the Python correctly, so the extractor and the translation are in the trusted base. `BehaviorModel.lean` is not a premise of the theorem today (`cgv/README.md`, "Trust model"), so it is not listed.

```json
{
  "format": "evidence-record/1",
  "commit": "<40-hex SHA of the project checked>",
  "claims": [
    {
      "id": "cgv-data-paths",
      "statement": "Every data path from a function to a model field in the project satisfies the field's declared constraints, as the CGV extractor read them.",
      "requirement": null,
      "strength": "proved",
      "basis": { "theorems": ["ContractGraph.runChecker_sound_all"] },
      "trusted_base": [
        { "component": "Lean 4 kernel", "version": "leanprover/lean4:v4.28.0" },
        { "component": "CGV Rust extractor", "version": "<CGV commit SHA>" },
        { "component": "CGV Translation.lean", "version": "<CGV commit SHA>" }
      ],
      "rerun": {
        "command": "crosscheck-contracts contracts check . --lean-checker <path to contract-graph-checker>",
        "exit_code": 0
      }
    }
  ]
}
```

### Crosscheck: a passing `/drt-oracle` run

`/drt-oracle` runs a Lean model as an oracle against an implementation on sampled inputs, and its report already records the seed and the number of inputs (`crosscheck/skills/drt-oracle/SKILL.md`). A pass is `tested`, not `proved`. The skill's own text says that the absence of divergences is evidence, not proof.

```json
{
  "format": "evidence-record/1",
  "commit": "<40-hex SHA>",
  "claims": [
    {
      "id": "drt-tariff-price",
      "statement": "tariff.price in src/tariff.py returns the same value as the Lean model Tariff.price on every generated input.",
      "requirement": null,
      "strength": "tested",
      "basis": { "cases": 1000, "seed": "<seed from the report>" },
      "trusted_base": [
        { "component": "Lean 4", "version": "leanprover/lean4:v4.10.0" },
        { "component": "Mathlib", "version": "v4.10.0" },
        { "component": "Lean model Tariff.price", "version": "<commit SHA>" },
        { "component": "DRT input generator", "version": "<commit SHA>" }
      ],
      "rerun": {
        "command": "<the reproducer command from the report, pinned to the seed>",
        "exit_code": 0
      }
    }
  ]
}
```

ER-1.3 chooses which Crosscheck pipeline emits first. A `dafny_verify` pass is the other candidate, and would give a `proved` claim whose trusted base names Dafny 4.11.0 and its Z3.

## Concerns flagged, not resolved here

- **Combining strengths.** The record keeps each claim's strength and has no overall verdict. How a `proved` claim, a `tested` claim and a `judged` claim combine into one verdict on a pull request is an open question of `docs/VISION.md`. A future version of the format may add a verdict once the question has an answer. Version 1 does not guess one.
- **The checker checks shape, not truth.** EV-13 forbids the checker from running the rerun commands. A record whose commands fail still passes EV-1 to EV-12. Rerunning every claim, which is the auditor's job in the vision, is not yet reached. The property that blocks it is a pinned, sandboxed environment for each command. The open question is how to pin the toolchains: the Crosscheck Docker images use the `ubuntu:22.04` tag with no digest, and the Lean image installs elan from an unpinned branch.
- **A judge must be a person, and a rerun must not call an LLM.** Rule 1 of the vision requires both. No rule above can decide either from the record. The checker accepts any non-empty `judge` and any command. Review is the only check today.
- **A record cannot sit in the commit it describes.** `commit` names a SHA, and a file inside that commit cannot know the commit's own SHA. Where a record lives (a CI artefact, a later commit, the pull request) is left to ER-1.2 and ER-1.3. The format does not depend on it.
- **CGV's contract levels are not claim strengths.** CGV tags each contract `PROVED`, `TESTED`, `EXTRACTED` or `ASSUMED` (`cgv/src/db.rs`). A docstring `ensures:` clause is `ASSUMED`, so the proved claim above rests on contracts that no tool checked. ER-1.2 decides how to state that in the record, for example by adding the assumed contracts to the trusted base or by writing a separate claim for them. This spec does not map one vocabulary onto the other.
- **Versions are free text.** EV-11 requires a non-empty `version` but does not check that it pins anything. `"latest"` passes.
- **Requirements are mostly `null` today.** No claim either tool can make traces to an approved requirement yet. RQ-1 governs that work. EV-8 checks the field's shape and not that the path exists.
- **Words in a statement are not checked.** EV-9 keeps `"verified"` out of `strength`. A `statement` that says "verified" passes.
- **Duplicate JSON keys.** Most JSON parsers keep the last of two keys with the same name. A record that writes `strength` twice passes with the second value. A checker that rejects duplicate keys needs its own parser, and the spec does not require one.
