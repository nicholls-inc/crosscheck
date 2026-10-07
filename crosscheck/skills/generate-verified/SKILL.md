---
name: generate-verified
add-mode: bootstrap
description: >-
  Generate a Dafny implementation body that satisfies a verified spec. Iteratively
  adds proof hints, loop invariants, and lemmas until the verifier accepts.
  With `evidence: <path.dfy>`, commits the verified program there and emits an
  evidence record with dafny_evidence.
  Use after /spec-iterate produces an approved spec. Triggers: "implement the spec",
  "generate verified code", "prove the implementation".
argument-hint: "[optional: Dafny spec to implement] [evidence: <path.dfy>] [requirement: <path[#anchor]>]"
---

# /generate-verified — Verified Dafny Implementation

## Description

Generate a Dafny implementation that satisfies a verified spec. Iteratively add proof hints, loop invariants, and lemmas until the verifier accepts the code.

## Instructions

You are a formal verification expert. The user has an approved Dafny specification (from `/spec-iterate` or provided directly). Your job is to write an implementation body that Dafny's verifier accepts.

### Step 1: Review the Spec

Read the spec carefully. Identify:
- What methods/functions need implementation bodies
- What the requires/ensures clauses demand
- What data structures are involved (arrays, sequences, datatypes)
- Whether ghost state or lemmas will likely be needed

### Step 2: Generate the Implementation

Replace placeholder bodies (`assume false;` or `...`) with actual Dafny code. Key strategies:
- **Start simple**: Write the most straightforward implementation first
- **Add loop invariants**: Every `while` loop needs invariants that:
  1. Are true before the loop starts
  2. Are preserved by each iteration
  3. Together with the negated loop guard, imply the postcondition
- **Add assertions**: Strategic `assert` statements help the verifier at intermediate points
- **Use calc blocks**: For complex arithmetic proofs
- **Add lemmas**: Factor out reusable proof obligations into separate lemmas

### Step 3: Verify

Call `dafny_verify` with the full program (spec + implementation).

If verification fails, analyze the errors and apply these repair strategies:

| Error Type | Repair Strategy |
|---|---|
| "postcondition might not hold" | Strengthen loop invariants or add assertions before the return |
| "loop invariant might not be maintained" | The invariant is too strong or the loop body has a bug — weaken or fix |
| "loop invariant might not hold on entry" | The invariant doesn't match initial state — adjust initialization or invariant |
| "decreases clause might not decrease" | Fix the termination measure or restructure the recursion |
| "index out of range" | Add bounds checks or strengthen preconditions |
| "assertion might not hold" | The assertion is wrong or needs intermediate lemma support |
| "cannot prove termination" | Add explicit `decreases` clause |

#### Interpret Difficulty Metrics

After each `dafny_verify` call, check the response for a `difficulty` field. If present, interpret the metrics:

- **Solver time**: If `solverTimeMs > 10000` (10s), flag as computationally expensive and suggest simplifying the spec or breaking into smaller lemmas
- **Resource count**: If `resourceCount > 500000`, warn about high resource usage and proof fragility across Dafny versions
- **Proof hints**: If `proofHintCount > 5`, note moderate/high proof complexity
- **Empty lemma bodies**: If `emptyLemmaBodyCount > 0`, flag that these may indicate trivially true properties — review needed
- **Trivial proof**: If `trivialProof` is true AND the spec has meaningful postconditions, note that property-based testing would have sufficed

If the `difficulty` field is absent (older server version), skip this section gracefully.

**Maximum 5 verification attempts.** On each attempt:
1. Show the current errors
2. Explain your repair strategy
3. Apply the fix
4. Re-verify

After 5 unsuccessful attempts, do not ask the user a chat-blocking question. Emit a structured failure artifact at `.crosscheck/work/dafny/<spec-id>/generate-verified-failure.md` containing the best version, per-attempt error logs, diagnosis, and a triage block at PR review:

```markdown
**Triage (mark exactly one):**
- [ ] Simplify the spec — <one-line on what to weaken>
- [ ] Restructure the implementation — <one-line on the algorithmic change needed>
- [ ] Abandon — the spec/impl combination is too hard to discharge in this form
```

Stop after writing the artifact. An orchestrator can re-dispatch with the relaxed input.

### Step 4: Post-Generation Checks

After generating verified code, check for these patterns and warn:

| Detected Pattern | Alert |
|---|---|
| `real` type usage | "Dafny `real` compiles to `_dafny.BigRational` in Python—you may want to replace with native `float` (losing formal precision guarantees)." |
| `seq<char>` / string operations | "Go backend has string/seq ambiguity at runtime. Test string operations carefully in extracted code." |
| Identifiers with underscores | "Go: Dafny identifiers starting with `_` may conflict with Go's file-naming rules. Renaming recommended." |
| Generics / type parameters | "Go: generic type parameters compile via type erasure to `interface{}`. Type assertions may be needed in extracted code." |

### Step 5: Present the Result

If verification succeeds, present:
- The full verified Dafny program
- Summary of what was proven
- Any proof artifacts added (lemmas, ghost variables) with explanations
- A difficulty summary (if the `difficulty` field was present in the `dafny_verify` response):

**Proof Difficulty Summary:**
| Metric | Value | Assessment |
|--------|-------|------------|
| Solver time | {X}ms | Low (<2s) / Medium (2-10s) / High (>10s) |
| Resource count | {N} | Low (<100K) / Medium (100K-500K) / High (>500K) |
| Proof hints needed | {N} | Minimal (0) / Moderate (1-5) / Heavy (>5) |
| Empty lemma bodies | {N} | OK (0) / Review needed (>0) |
| Overall | Trivial/Moderate/Complex | — |

If all 5 attempts fail, the structured failure artifact from Step 3 is the deliverable. Do not paste the best version into chat as the primary output; the artifact path is the handoff.

### Step 6: Write Verified Artifact

Persist the verified implementation to `.crosscheck/work/dafny/<spec-id>/impl.dfy` per the persistence convention. The downstream `/extract-code` and `/check-regressions` skills consume this file directly.

### Step 7: Emit an Evidence Record

This step commits the verified program and asks the MCP tool `dafny_evidence` for an evidence record (`evidence-record/1`) with one `proved` claim. The spec is `intent/2026-10-06-generate-verified-evidence-spec.md` (GV-1 to GV-9) in the Crosscheck repository. The tool's own rules (DE-1 to DE-13 in `intent/2026-10-06-dafny-evidence-record-spec.md`) decide whether a record is emitted. This step only puts the repository in the state the tool asks for, and reports what the tool returns.

Run this step only when the invocation names `evidence: <path>` and verification succeeded in Step 3. Otherwise skip it, make no commit, write no file, and record the reason for Step 8: `No evidence record: no evidence path was named.` or `No evidence record: verification did not succeed.` The file in `.crosscheck/work/` cannot be the committed file the tool needs, because `.crosscheck/` is gitignored by convention, so the caller names where the repository keeps it.

All commands run at the work tree's top level (`git rev-parse --show-toplevel`). When a command below says "stop", skip the rest of this step and give Step 8 the reason and the command's output.

1. **Check the path.** `<path>` is relative to the top level, ends in `.dfy`, uses `/`, has no `..`, `.` or empty segment, and uses only `A-Z a-z 0-9 _ . / -`. Otherwise stop. The tool refuses such a path anyway (DE-1).
2. **Ignore `.crosscheck/`.** Run `git check-ignore -q .crosscheck/work/dafny/<spec-id>/impl.dfy`. If it exits non-zero, write `.crosscheck/.gitignore` with the single line `*`, which ignores everything under `.crosscheck/` including itself, and run the check again. If it still fails, stop. Do not edit the repository's own `.gitignore`.
3. **Require a clean tree.** Run `git status --porcelain --untracked-files=all`. If it prints anything, stop and list every path it printed. Do not commit, stash, discard or ignore a change this run did not make. The tool refuses a dirty tree (DE-3), and stopping here avoids a commit made for nothing.
4. **Commit the program.** Write the exact bytes of `.crosscheck/work/dafny/<spec-id>/impl.dfy` to `<path>`, creating its directory, and run `git add -- <path>`. If `git add` fails, for example because `<path>` is ignored, stop with git's output. If `git diff --cached --quiet` exits 0, HEAD already holds this program, so make no commit. Otherwise run `git commit -m "chore: add <path>, verified by /generate-verified"`. Never pass `--no-verify` or `--amend`. If the commit fails, stop with git's output, and leave `<path>` staged for the caller.
5. **Name the theorems.** List the fully qualified name of each method, function and lemma that carries the `requires` and `ensures` clauses signed off in the spec: `M.C.Name` for `Name` in class `C` of module `M`, and `Name` alone at the top level of the file. This is the name Dafny's verification log prints. Do not list helper lemmas added in Step 2, since they prove the contracts and do not state them.
6. **Draft the statement.** In plain language, for a reader who will not open the code, say what the contracts of those theorems guarantee, and nothing more. This is your draft. A person must check it against the contracts (rule 1 of the Crosscheck vision), and the record cannot show that anyone did.
7. **Call the tool.** Call `dafny_evidence` with:
   - `repoPath`: the top level, as an absolute path;
   - `file`: `<path>`;
   - `statement`: the draft from 6;
   - `requirement`: the invocation's `requirement:` value, or `null`;
   - `theorems`: the names from 5;
   - `outputPath`: `.crosscheck/work/dafny/<spec-id>/evidence.json`. That path is ignored, so the record leaves the tree clean, and a later run overwrites it.
8. **Handle a refusal.** If the result has `success: false`, give Step 8 every entry of `errors` as the tool wrote it. If an error names a theorem (an invalid name, or a name not in Dafny's log), you may correct the names and call once more. Never edit the program, the spec or a contract to get a record. A refusal is a finding, not an obstacle.

### Step 8: Present the Evidence Summary

Present an Evidence Summary, not a checklist:

```
## Evidence Summary (agent-verified during this run)

- Implementation verifies via dafny_verify — all postconditions discharged.
- Proof complexity (from difficulty metrics, if present): solver <Xms>, resource <N>, hints <K>, empty lemma bodies <N> — overall <Trivial|Moderate|Complex>.
- Empty lemma bodies flagged: <list with line refs, or "none">.
- Target-language pitfalls detected in Step 4: <list with file:line refs, or "none">.
- Implementation written to .crosscheck/work/dafny/<spec-id>/impl.dfy.
- Evidence record: <writtenTo>, claim <id>, strength <strength>, for commit <commit> (<path> committed by this run | <path> already at HEAD). The record describes that commit, not the work tree. Copy it to wherever the change ships it: a commit, the pull request or a CI artefact.
  Or: No evidence record: <reason from Step 7, with each tool error or path verbatim>.
- .crosscheck/.gitignore written: <yes | no>.

## Decisions for Review (human owns these at PR time, if any)

- [ ] Evidence-record statement (only if a record was emitted): does "<statement>" say what the contracts of <theorems> say, and nothing more? The statement is an agent draft, and the record's `proved` covers the contracts, not the statement.
- [ ] Empty-lemma-body finding (only if flagged): are the trivial properties intentional, or do they indicate over-claimed postconditions?
- [ ] Trivial-proof finding (only if flagged + spec has meaningful postconditions): would `/lightweight-verify` have sufficed for this property?
```

The "Consider using `/lightweight-verify`" aside on trivial proofs is moved into the Decisions block as a structured question, not a fire-and-forget chat aside — it is only surfaced when the metrics actually indicate triviality on a non-trivial-looking spec.

## Arguments

Optionally, the Dafny spec to implement. If not provided, assumes the spec was established in the current conversation via `/spec-iterate`.

- `evidence: <path>`: where to commit the verified program, relative to the work tree's top level, ending in `.dfy`. Naming it turns on Step 7, which makes one commit in the repository and emits an evidence record. Without it, the skill makes no commit and emits no record.
- `requirement: <path[#anchor]>`: the requirement the record's claim traces to. Without it, the claim's `requirement` is `null`. It has an effect only with `evidence:`.

Examples: `/generate-verified`, `/generate-verified evidence: verified/max.dfy`, `/generate-verified evidence: specs/max.dfy requirement: docs/requirements.md#max`
