# Plan: Only deterministic checks count as evidence in CI

Intent: `intent/2026-09-29-deterministic-evidence.md`
Spec: `intent/2026-09-29-deterministic-evidence-spec.md` (requirement IDs TG-*, SM-*, CI-*, WR-*, DOC-*)
Governing roadmap item: PB-1. Tier: 3.

This file replaces the root `plan.md` from #246, which remains in git history.

## Order of work

1. **Governance note first.** Write `.assurance/protected-surface-amend/deterministic-evidence-2026-09-29.md`, naming every protected file in the table below. Until it exists, the PreToolUse hook blocks each edit.
2. **Statement manifest (SM-1 to SM-5).**
   - Add `cgv/prover/scripts/ProtectedStatements.lean`.
   - Run `lake build ContractGraph ContractGraph.Main`, then `lake env lean --run scripts/ProtectedStatements.lean > protected-statements.txt` in `cgv/prover`.
   - Commit `cgv/prover/protected-statements.txt`.
3. **Tier gate (TG-1 to TG-10).** In `scripts/ci/tier-gate.mjs`:
   - Add a `changedAndPresent(path)` helper.
   - Bind root `spec.md`, root `plan.md`, and governance notes to it.
   - Drop the attestation check.
   - Add the TG-7 heading check and the TG-8 and TG-9 report lines.
   - Export `evaluate(inputs)` so tests can call it without `process.exit`. The CLI wrapper keeps the same output and exit codes.
4. **Tier gate tests.**
   - Add `scripts/ci/tier-gate.test.mjs`, using `node:test`. Each test builds a temporary repository directory and calls `evaluate`. At least one test covers each TG requirement.
   - In `tier-gate.yml`, run `node --test scripts/ci/*.test.mjs` before the gate.
5. **CGV CI (CI-1 to CI-6).** Add `.github/workflows/cgv-ci.yml` with one job:
   - checkout;
   - Rust toolchain with a cache;
   - `cargo test` and `cargo build --release`;
   - elan from its release binary, using the toolchain in `lean-toolchain`, with a cache of `cgv/prover/.lake`;
   - `lake build`;
   - `scripts/check-fixtures.sh`;
   - the manifest diff.
6. **Remove LLM workflows (WR-1 to WR-3).** Delete `spec-audit.yml` and `protected-surface-check.yml`, then grep that no workflow references `ANTHROPIC_API_KEY` or `claude-code-action`.
7. **Documents (DOC-1 to DOC-5).** Update the files in the table. Then update the monorepo intent's pointer to the governing item.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | TG-1 to TG-10 |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | new tests |
| `.github/workflows/tier-gate.yml` | yes | run the tests |
| `.github/workflows/cgv-ci.yml` | yes | new |
| `.github/workflows/spec-audit.yml` | yes | deleted |
| `.github/workflows/protected-surface-check.yml` | yes | deleted |
| `.claude/rules/protected-surfaces.md` | yes | manifest path, CGV manifest text, sign-off without branch protection |
| `docs/assurance/TIER-LAYER-MAP.md` | yes | Tier 3 artefacts, CGV evidence, evidence classes |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes | workflow list, stage 4, CGV |
| `docs/assurance/ROADMAP.md` | yes | PB-1 scope |
| `cgv/prover/protected-statements.txt` | yes, after this change (DOC-3) | new |
| `cgv/prover/scripts/ProtectedStatements.lean` | no | new |
| `docs/gates/tier-layer-gate.md`, `docs/gates/intent-check-verdict.md`, `docs/gates/audit-spec-coverage-triage.md` | no | DOC-1, DOC-2 |
| `cgv/CLAUDE.md`, `cgv/README.md` | no | DOC-4 |
| `intent/2026-09-29-deterministic-evidence*.md`, `plan.md` | no | stage artefacts |

No `SKILL.md` or `agents/*.md` file changes.

## Risks

- **The gate rejects legitimate pull requests.** A plan or spec committed in an earlier pull request no longer counts implicitly. Mitigation: a `Plan:` or `Spec:` citation still counts (TG-3, TG-4), and the failure message says so.
- **The gate's own pull request.** This pull request changes the gate. CI runs the new gate from the pull request's head, so the new rules judge this pull request. It must pass them: it changes `plan.md`, adds a governance note naming every protected file, and has a protected-surface change section.
- **The manifest trips on a Lean toolchain bump.** This is intended (spec, SM section), but it forces a Tier 3 pull request for every bump.
- **CI minutes.** The CGV job runs only on `cgv/**`. The measured local cost is about 30 seconds for `lake build`, plus a Rust release build. On a private repository these minutes count against the free allowance.
- **Private repository, no branch protection.** No check blocks a merge. The spec flags this rather than resolving it.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, with at least one test for each TG requirement.
- Running the gate locally against this branch's diff and PR body passes, and prints the evidence report.
- The manifest sensitivity run (SM-3, SM-4) has been done:
  - two runs are byte-identical;
  - a proof-only edit (`:= rfl` to `:= by rfl` in `incompleteWith_exitCode`) leaves the manifest unchanged;
  - weakening that statement to `2 ≤ …` changes it;
  - editing the `ToString ConstraintKind` instance changes its value hash.
- `cargo test`, `cargo build --release`, `lake build`, `scripts/check-fixtures.sh`, and the manifest diff pass locally on this branch.
- `grep -rn "ANTHROPIC_API_KEY\|claude-code-action" .github/workflows` returns nothing.
