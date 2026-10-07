# Plan: Check every slash-reference in Crosscheck's Markdown

Intent: `intent/2026-10-07-slash-references.md`
Spec: `intent/2026-10-07-slash-references-spec.md` (SR-1 to SR-10)
Governing roadmap item: PB-1. Task: PB-1.18. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Data shape

One record per reference: `{ path, line, plugin, name, text }`. The checker builds it from an index snapshot `{ files: Map<path, text>, skills: Set<name>, agents: Set<name> }`. Resolution is a pure function of the record, the snapshot and the parsed allowlist, so the CLI, the pre-commit check and the tests share it. The catalogue is a pure function of `skills` and their `SKILL.md` text.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/slash-references-2026-10-07.md`, naming every protected file below.
2. Add `scripts/ci/skill-references.test.mjs` and the fixture `scripts/ci/fixtures/skill-references/`. Run the tests and see them fail, because the checker does not exist.
3. Add `scripts/ci/skill-references.mjs` (SR-1 to SR-7) and `crosscheck/slash-allowlist.txt`. Run the tests until they pass.
4. Run the checker on this repository. Fix each live reference that does not resolve in the text (spec, "Live text that named unbuilt skills"). Leave the three dated records of SR-2 as written. Run `node scripts/ci/skill-references.mjs --write` to generate `crosscheck/docs/skills.md`.
5. Add the check to `scripts/ci/pre-commit.mjs` (SR-8), copy the new script in `pre-commit.test.mjs`, and add its case.
6. Add `.github/workflows/skill-references.yml` (SR-9).
7. Add the explainer `docs/gates/skill-references.md`, its row in `docs/gates/README.md`, and the workflow in stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md`.
8. Mutate the checker (drop the allowlist lookup, accept any name, skip the catalogue comparison, widen the left boundary) and confirm a test fails for each. Restore with git.
9. Time the pre-commit hook on this repository with a staged `crosscheck/` Markdown file.
10. Set PB-1.18 to `done` in `docs/TASKS.md` with the intent as its record. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/skill-references.mjs` | yes (`scripts/ci/**`) | new, SR-1 to SR-7 |
| `scripts/ci/skill-references.test.mjs` | yes (`scripts/ci/**`) | new, SR-10 |
| `scripts/ci/fixtures/skill-references/**` | yes (`scripts/ci/**`) | new fixture with a broken reference |
| `scripts/ci/pre-commit.mjs`, `scripts/ci/pre-commit.test.mjs` | yes (`scripts/ci/**`) | SR-8 |
| `.github/workflows/skill-references.yml` | yes (`.github/workflows/**`) | new, SR-9 |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | stage 5 names the workflow |
| `crosscheck/skills/journal-context/SKILL.md` | yes (`crosscheck/skills/*/SKILL.md`) | `/journal-lint` loses its slash |
| `crosscheck/slash-allowlist.txt` | no | new, comments only |
| `crosscheck/docs/skills.md` | no | generated |
| `crosscheck/docs/orchestrator-coordination.md`, `crosscheck/docs/add/operating-modes.md`, `crosscheck/conformance/README.md` | no | unbuilt names and placeholders reworded |
| `crosscheck/skills/journal-context/docs/invariants/journal-context.md` | yes (Class B, a module invariant doc; the hook's glob does not reach it) | `/journal-lint` loses its slash, and a governance section names the amendment |
| `crosscheck/JOURNAL.md`, `crosscheck/skills/JOURNAL.md`, `crosscheck/docs/specs/rationale-2026-05-11.md` | no | unchanged; SR-2 skips these dated records |
| `docs/gates/skill-references.md`, `docs/gates/README.md` | no | explainer and inventory row |
| `docs/TASKS.md`, `JOURNAL.md` | no | PB-1.18 `done`, entry |
| `.assurance/protected-surface-amend/slash-references-2026-10-07.md` | no | new governance note |
| `intent/2026-10-07-slash-references*.md` | no | stage artefacts |

## Risks

- **False positives block commits.** The grammar (SR-3) is tuned against the 123 Markdown files in `crosscheck/`. The real-repository test fails if a later edit to the grammar flags text on `main`.
- **False negatives.** A reference the grammar leaves out, such as `/Name` or `x/name`, is not checked. The grammar table in the tests pins what counts.
- **Hook budget.** The check reads about 120 blobs through one `git cat-file --batch` process, which keeps it well under PC-7's 5 seconds.
- **Conformance.** `crosscheck/conformance/README.md` changes. `go run . ..` in `crosscheck/conformance` must still pass.

## Proof

- `node --test scripts/ci/*.test.mjs` passes, and each step 8 mutant fails at least one test.
- `node scripts/ci/skill-references.mjs` exits 0 on this branch, and exits 1 naming `crosscheck/skills/journal-context/SKILL.md:30` on `main`.
- `go run . ..` in `crosscheck/conformance` prints `RESULT: PASS`.
