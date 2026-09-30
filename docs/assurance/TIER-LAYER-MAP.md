# Tier–Layer Map

This document is Crosscheck's definition of what the AI-Native SDLC playbook calls
**"regulated and critical code"**. The playbook concentrates human approval at defined
gates but leaves each repository to say which of its own surfaces are critical. That
answer lives here: the tier a change falls into determines which artefact must be
committed before a human is asked to approve anything.

The check is deterministic. The **tier-gate CI job** (`scripts/ci/tier-gate.mjs`, run by
`.github/workflows/tier-gate.yml`, with its own tests in
`scripts/ci/tier-gate.test.mjs`) evaluates every pull request against the rules below.
It must pass **before the review gate opens**. A failing tier gate is not a review
comment: the review has not started yet. The ruleset on the default branch requires no
status checks, so the gate cannot stop a merge. The maintainer does not merge while it is red (see
[Evidence and sign-off](#evidence-and-sign-off)).

## Declaring a tier

Every pull request declares its tier in one of two ways:

- a `Tier: N` line in the PR body (`Tier: 2`), or
- a `tier:N` label on the PR (`tier:2`).

A `Tier:` line starts with the keyword, optionally indented or after a list or quote
marker (`- `, `* `, `+ `, `> `), as a citation does. `Tier:` in the middle of a line
does not declare a tier, so quoted text cannot set it. If the body has more than one
`Tier:` line, the first wins. If both a line and a label are present, the line wins.
If neither is present, the tier gate fails.

**Protected-path floor.** If the diff touches any path listed in
`.claude/rules/protected-surfaces.md`, the change is at least Tier 3 regardless of what
it declares. A declaration of Tier 1 or Tier 2 on such a diff is a gate failure, not a
judgement call. Declaring a tier *above* the floor is always permitted.

## The tiers

### Tier 1 — routine

**Scope:** documentation, tests, and non-behavioural code (formatting, renames,
comments, build plumbing that changes no output).

**Artefact required:** a reference to the governing `intent.md` — its path under
`intent/`, cited in the PR body.

**Worked example.** A PR fixes three typos in `crosscheck/README.md` and adds a missing
assertion to an existing vitest case. Nothing in the diff is a protected path, no
behaviour changes. The author declares `Tier: 1` and cites
`intent/2026-08-docs-tidy.md`. The tier gate checks the declaration and that the cited
intent file exists; review then covers the diff itself.

### Tier 2 — standard

**Scope:** behavioural code changes — anything that alters what the software does for a
user or a caller. New MCP tool behaviour, changed parsing, changed thresholds in
non-protected code.

**Artefact required:** a spec for the change, in addition to the `intent.md` it derives
from. The spec must flag unresolved concerns rather than quietly settle them. Either
of these satisfies the requirement:
- the PR changes a root `spec.md`;
- the PR body has a `Spec: <path>` line that cites an existing file.

A root `spec.md` left over from an earlier change does not count.

**Worked example.** A PR changes how `dafny_verify` reports timeouts, so that callers
can tell a timeout from a verification failure. No protected surface is touched, but
the observable behaviour of a tool changes. The author:
- declares `Tier: 2`;
- writes a spec describing the new result shape and the open question about
  backwards compatibility;
- cites the intent.

**CGV worked example.** A PR adds a new constraint kind to the checker. For CGV, the
spec can be the changed Lean definitions plus the new `test_fixtures/*/expected.json`,
cited as `Spec: cgv/test_fixtures/<fixture>/expected.json`. The Lean statements are a
stronger spec than prose, so a prose `spec.md` is optional.

### Tier 3 — critical / protected

**Scope:** protected surfaces (per `.claude/rules/protected-surfaces.md`), gate logic,
hooks, CI enforcement, and invariants. In short: anything that changes how the project
decides whether other changes are safe.

**Artefacts required:**

1. **A plan.** It lists the files that change, the order of work, the risks, and the
   proof or tests that will show the change is correct. It is written so that an
   engineer who never saw the conversation could implement it. Either of these
   satisfies the requirement:
   - the PR changes a root `plan.md`;
   - the PR body has a `Plan: <path>` line that cites an existing file.

   A root `plan.md` left over from an earlier change does not count.
2. **A governance note, for any edit to a protected path.** This is the
   `## Protected-Surface Amendment` block produced by `/protected-surface-amend`. The
   PR must change it, under `.assurance/protected-surface-amend/` or
   `.assurance/add-session-*/`. It must name every changed protected file, and its
   block goes in the PR description. A note from an earlier change does not count.
3. **For a CGV proof surface** (`BehaviorModel.lean`,
   `cgv/prover/protected-statements.txt`, or its generator
   `cgv/prover/scripts/ProtectedStatements.lean`), a `## Protected-surface change` section in
   the PR body. See `.claude/rules/protected-surfaces.md`.

**No LLM verdict is an artefact.** Tier 3 used to require an `intent-check`
attestation. That record is the verdict of an LLM back-translator and diff-checker, and
`docs/VISION.md` rules out LLM judgement as evidence. `/intent-check` remains a useful
advisory tool to run locally, but the gate never reads its output.

**Worked example.** A PR tightens the abort threshold inside
`crosscheck/skills/reason/SKILL.md`. That path matches `crosscheck/skills/*/SKILL.md`,
so the Tier 3 floor applies, even though the author first thought of it as a wording
change. The PR must carry:
- `Tier: 3`;
- a changed `plan.md`, or a `Plan:` citation;
- a new governance note naming that `SKILL.md`.

If either artefact is missing, the tier gate fails.

**CGV worked example.** A PR strengthens `runChecker_sound_all`. The `CGV CI` manifest
check fails until the author regenerates `cgv/prover/protected-statements.txt`. That
file is protected, so the PR becomes Tier 3. It needs a plan, a governance note naming
the manifest, and a `## Protected-surface change` section that says the guarantee now
promises more.

## Evidence and sign-off

The tier gate checks that the artefacts exist. It does not decide whether the change is
correct. That evidence comes from deterministic CI jobs, and on a pass the gate reports
which job holds the evidence for each class of changed file:

| Changed path | Deterministic evidence |
|---|---|
| `cgv/**` | `CGV CI`: `cargo test`, `lake build` (proofs and `#guard` tests), fixtures, statement manifest and axiom check |
| `crosscheck/mcp-server/**`, `crosscheck/docs/invariants/**` | `CI`: `npm test`, including the property tests |
| `crosscheck/conformance/**` | `CI`, conformance job: `go vet`, `go test`, `go run . ..` |
| `scripts/ci/**`, `.claude/hooks/protected-surface-guard.mjs` | `Tier Gate`: `node --test scripts/ci/*.test.mjs` |
| `evals/**` | `Incident Eval Check` |
| `crosscheck/skills/**`, `crosscheck/agents/**`, `.claude/rules/**`, `docs/assurance/**` | not yet reached |
| `.github/workflows/**` | not yet reached |
| any other `.md` or `.pdf` file | none required at this tier (prose) |
| anything else | not yet reached |

The first matching row wins. Each "not yet reached" line in the report names the
property that blocks a deterministic check and the open question:

- **Skills, agents, rules and governance documents.** Their behaviour is prompt text that
  an agent interprets. The open question is what a replayable behavioural eval of a
  prompt artefact looks like.
- **Workflow definitions.** A workflow runs only on GitHub's runners, on GitHub's events.
  The open question is how to replay a workflow against recorded events before it merges.
- **Everything else that is not prose**, such as `logic-distribution/**`,
  `formal-verification/**`, `crosscheck/scripts/**`, `crosscheck/demo/**` and
  configuration files. No CI workflow runs a check on the path. The open question is
  which deterministic check the code needs, and which workflow runs it.

The full rule is TG-8 in `intent/2026-09-30-tier-anchor-spec.md`.

**The human sign-off is the maintainer's merge.** The default branch has a ruleset,
`default`, that asks for one approving review. The maintainer is the only person who
can approve and cannot approve their own pull request, so every merge bypasses the
ruleset. The ruleset requires no status checks, so no CI job, this gate included, can
block a merge. A red check is information for the maintainer. Approval by someone other
than the author is not yet reached, because the repository has no second person with
write access.

## Reading the map

The tier is a floor on scrutiny, never a ceiling. A reviewer who believes a Tier 2
change is really Tier 3 should say so in review; the correct remedy is to raise the
declaration and add the missing artefacts, not to argue the path list. Changes to the
path list itself are Tier 3 by construction, because `.claude/rules/**` is protected.

If you are unsure which tier applies, ask the Crosscheck maintainers via a GitHub issue
on this repository before opening the PR. The full explanation of the gate's failure
message is in `docs/gates/tier-layer-gate.md`.
