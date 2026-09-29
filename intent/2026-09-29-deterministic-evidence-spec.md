# Spec: Only deterministic checks count as evidence in CI

Intent: `intent/2026-09-29-deterministic-evidence.md`. Decisions: `intent/2026-09-29-crosscheck-monorepo.md`. Governing roadmap item: PB-1.

Each requirement has an ID. The tests cite these IDs.

## Tier gate (`scripts/ci/tier-gate.mjs`)

Inputs are unchanged: `PR_BODY`, `PR_LABELS`, `CHANGED_FILES`, `BASE_REF`, and `CROSSCHECK_PROTECTED_RULES`. A path *changed in this pull request* is one listed in `CHANGED_FILES` that still exists in the working tree. A deleted file does not count as an artefact.

- **TG-1. Declaration and floor are unchanged.** A tier comes from a `Tier: N` line or a `tier:N` label. If the diff touches a protected path, the floor is Tier 3. A declaration below the floor fails.
- **TG-2. Tier 1 is unchanged.** Tier 1 passes when the pull request changes an intent file under `intent/` (other than `README.md` or `TEMPLATE.md`), or when an `Intent: <path>` line cites an existing file. A citation line starts with the keyword, optionally indented or after a list or quote marker (`- `, `* `, `+ `, `> `); the keyword in the middle of a line does not count. The same holds for `Spec:` and `Plan:`.
- **TG-1a. Renames.** The changed-file list is computed with `--no-renames`, so a protected file moved to another path counts as a change to the protected path. `tier-gate.yml` holds this; the node tests take the list as input and do not cover it.
- **TG-3. Tier 2 spec.** A root `spec.md` counts only if this pull request changes it. A `Spec: <path>` line that cites an existing file also counts. For CGV, the cited file may be the changed Lean file or fixture.
- **TG-4. Tier 3 plan.** A root `plan.md` counts only if this pull request changes it. A `Plan: <path>` line that cites an existing file also counts.
- **TG-5. Tier 3 governance notes.** A note counts only if this pull request changes it and its path matches `.assurance/protected-surface-amend/*.md` or `.assurance/add-session-*/**.md`. Every changed protected file must be named in at least one counting note. A note that is already in the tree but not changed does not count.
- **TG-6. No attestation.** The gate never reads `intent-check-attestation.json` files and never requires one. Whether one exists does not affect the result.
- **TG-7. CGV proof surfaces.** If this pull request changes `cgv/prover/ContractGraph/BehaviorModel.lean`, `cgv/prover/protected-statements.txt`, or its generator `cgv/prover/scripts/ProtectedStatements.lean`, the PR body must contain a Markdown heading (level 2 or deeper) whose text is `Protected-surface change`, case-insensitive.
- **TG-8. Evidence report.** On pass, the gate prints one line for each class of changed file. Each line names the CI workflow that holds the deterministic evidence for that class, or says `not yet reached: human review is the only evidence`. The report is information only and never changes the result. The classes, with the first match winning:

  | Changed path | Evidence |
  |---|---|
  | `cgv/**` | `CGV CI` workflow (`cargo test`, `lake build`, fixtures, statement manifest and axiom check) |
  | `crosscheck/mcp-server/**`, `crosscheck/docs/invariants/**` | `CI` workflow (`npm test`, including the property tests) |
  | `scripts/ci/**` | `Tier Gate` workflow (`node --test scripts/ci/*.test.mjs`) |
  | `evals/**` | `Incident Eval Check` workflow |
  | `crosscheck/skills/**`, `crosscheck/agents/**`, `.claude/**`, `docs/assurance/**`, `.github/workflows/**` | not yet reached: human review is the only evidence |
  | anything else | none required at this tier |

  The skill, agent, rule, hook, and workflow rows are "not yet reached" because none of these files has a deterministic behaviour check. The blocking property is that their behaviour is prompt text, or the gate definition itself. The open question is what a replayable behavioural eval of a prompt artefact would look like.
- **TG-9. Human sign-off statement.** On pass, the gate prints that the maintainer's merge is the human sign-off, and that CI does not enforce it because the repository has no branch protection.
- **TG-10. Failure message.** Failures keep the fixed three-sentence gate message and the link to `docs/gates/tier-layer-gate.md`.

## CGV statement manifest

- **SM-1.** `cgv/prover/scripts/ProtectedStatements.lean` prints, in a fixed order:
  - the type of every theorem listed in the CGV table in `.claude/rules/protected-surfaces.md`;
  - the type and a structural value hash of `constraintImplies`, `IsDataPath`, and `stepwiseSound`, and of every non-theorem `ContractGraph.*` constant those three reach through types or values. Inductive types include their constructors.
- **SM-2.** The script fails with a non-zero exit if a listed name is missing, or if a listed theorem is not a theorem. This covers a rename or deletion.
- **SM-3.** The output is byte-identical across runs on the same sources and the same toolchain.
- **SM-4.** Changing only a proof leaves the output unchanged. Changing a listed statement, a listed definition, or a definition they reach changes the output.
- **SM-5.** `cgv/prover/protected-statements.txt` is the committed output. CI fails when a fresh run differs from it. The manifest is a protected path, so updating it forces Tier 3 (TG-1), a governance note (TG-5), and the protected-surface change section (TG-7). The generator is a protected path for the same reason: it decides what the manifest records.
- **SM-6.** The script fails with a non-zero exit if a listed theorem or definition depends on an axiom other than `propext`, `Classical.choice` and `Quot.sound`. `lake build` accepts `sorry` with a warning, so without SM-6 a `sorry`, or a new axiom that closes a proof, would leave the manifest and every CI step unchanged. A declaration the kernel never checked is not yet reached: `set_option debug.skipKernelTC true` with `addDecl` adds a theorem that reports no axioms. The property that blocks it is a kernel replay of the built environment, and the open question is whether a replay (`lean4checker`, `Environment.replay`) fits CGV CI's time budget.

The manifest does not catch changes outside what the listed definitions reach. Examples are the checker (`runChecker`, `checkEdge`) and `Translation.lean`. That is intended: the theorems prove the checker against the statements, and the translation is untrusted by design (see `cgv/CLAUDE.md`, Trust model). A Lean toolchain bump may change the pretty-printed text or the hashes, and so forces a manifest update and Tier 3 review. That is acceptable, because a toolchain bump changes the trusted kernel.

## CGV CI (`.github/workflows/cgv-ci.yml`)

- **CI-1.** Runs on pull requests and on pushes to `main` that change `cgv/**` or the workflow file itself.
- **CI-2.** In `cgv/`, runs `cargo test` and `cargo build --release`, after CI-3: the end-to-end Rust test runs the checker binary that `lake build` produces, and skips when it is missing.
- **CI-3.** Installs the toolchain pinned in `cgv/prover/lean-toolchain`, then runs `lake build` for the default targets in `cgv/prover`. These are the proofs, the checker, and the `#guard` tests.
- **CI-4.** Runs `scripts/check-fixtures.sh`.
- **CI-5.** Regenerates the statement manifest and fails on any difference from the committed file, printing the diff, or when the generator fails (SM-2, SM-6).
- **CI-6.** Calls no LLM and uses no secret.

## Workflows removed

- **WR-1.** `.github/workflows/spec-audit.yml` is deleted, because it ran `/audit-spec-coverage` and `/audit-invariant-consistency` through an LLM.
- **WR-2.** `.github/workflows/protected-surface-check.yml` is deleted. It ran `/intent-check` through an LLM, and its detection job never checked governance notes. The tier gate's TG-5 already does that.
- **WR-3.** No remaining workflow references `ANTHROPIC_API_KEY` or `claude-code-action`.

## Documents

- **DOC-1.** `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, and `.claude/rules/protected-surfaces.md` describe TG-1 to TG-9, the CGV evidence, the workflow list after WR-1 and WR-2, and the sign-off without branch protection.
- **DOC-2.** `docs/gates/intent-check-verdict.md` and `docs/gates/audit-spec-coverage-triage.md` no longer describe CI runs. They describe the local, advisory runs.
- **DOC-3.** `.claude/rules/protected-surfaces.md` adds `cgv/prover/protected-statements.txt` and `cgv/prover/scripts/ProtectedStatements.lean` to the machine-readable list, and describes the manifest in the CGV section.
- **DOC-4.** `cgv/CLAUDE.md` describes the manifest and CGV CI. `cgv/README.md` does too, if its trust model covers theorem statements.
- **DOC-5.** The PB-1 scope in `docs/assurance/ROADMAP.md` names the workflows that remain, and states that the framework covers CGV.

## Concerns flagged, not resolved here

- **No merge blocking.** Without branch protection, every requirement above is information for the maintainer, not a merge condition. "The merge is the sign-off" depends on the maintainer reading red checks.
- **Advice the product gives other repos.** Crosscheck's own skills (`/assurance-init`, `/assurance-status`, `/draft-invariants`, `/intent-check`, `add-orchestrator`) still tell target repositories that Tier 3 needs an intent-check attestation. That advice conflicts with the vision, but changing it changes skill behaviour, which this spec excludes. It needs its own intent.
- **The stale attestation stays committed.** The attestation from #134 remains under `crosscheck/mcp-server/.assurance/` as a historical record. TG-6 makes it inert.
