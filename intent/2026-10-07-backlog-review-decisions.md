# Decisions: the imported Crosscheck backlog, #19 to #41

Task: AD-1.1. Intent: [`2026-10-07-backlog-review.md`](2026-10-07-backlog-review.md). Checked against `origin/main` at `e5089f7` on 2026-10-07.

Every line cited below is on that commit. "Rule N" is design rule N of `docs/VISION.md` (lines 48 to 60). No decision rests on an LLM's view of an issue's value. A Drop cites a duplicate, a file that already does the work, or a line of the vision.

## Decision table

| Issue | Title | Decision | Serves | Reason | Row |
|---|---|---|---|---|---|
| #19 | Decide the auditor's write path and JSON sidecar | Refine | Rule 3 | The auditor's report has no machine-readable record that a later pass or a reader can rerun and compare | AD-1.2 |
| #20 | Add an `unaudited` verdict, and define the audited set | Refine | VA-1, rule 7 | The auditor gives `settled` to an artefact it could not check | VA-1.9 |
| #21 | Key lowry's D1 entry on any D2 gate red | Refine | Rule 1 | lowry's entry reads one of its four deterministic gates and ignores the other three | AD-1.3 |
| #22 | Build the judged-oracle harness | Refine | VA-1, rule 1 | The issue makes an LLM judge the pass condition. Rule 1 (`docs/VISION.md:48`) rules that out, so the rewrite removes the judge and keeps the deterministic parts | VA-1.10 |
| #23 | Make ADD Step 0 mode selection exhaustive and non-circular | Refine | Rule 3 | The mode is chosen by prose with an undefined predicate, so two runs on one repository can choose differently | AD-1.4 |
| #24 | Align AUTO 5's routing grammar | Refine | PB-1 | A deterministic CI check misses bare-slash and agent-to-agent edges | PB-1.19 |
| #25 | Known-status allowlist for the claims ledger | Refine | PB-1, rule 7 | A mistyped claim status passes the conformance check silently | PB-1.20 |
| #26 | Reconcile the invariant-heading grammar with the real corpus | Refine | PB-1 | The header and comment patterns disagree, and every real invariant doc fails the canonical form | PB-1.21 |
| #27 | Ship the rest of ADD | Refine | AD-1 | No claim in `crosscheck/conformance/claims.json` is `known-gap` any more, and AD-1 cites it as its document | AD-1.1 |
| #28 | Tighten the trigger criterion for ADD | Refine | Vision, limits | The cost of writing proofs and models is one of the hard limits the vision names (`docs/VISION.md:93`). A stated trigger keeps ADD's cost where it pays | AD-1.5 |
| #29 | Surface adversarial-probe routing during audit | Refine | Rule 1 | The choice of modules to probe is made silently. Stating it lets a person decide it | AD-1.6 |
| #30 | Coverage-gate retrofit pass | Refine | Vision, Tests class | A retrofitted `Invariant` comment claims coverage that nothing checks. The rewrite ties each retrofit to a killed mutant | AD-1.7 |
| #31 | Pre-flight invariants that change the whole test suite | Refine | Rules 1, 3 | It replaces a surprise at merge time with a measured count from a rerunnable command | AD-1.8 |
| #32 | Auto-close mechanical findings | Refine | Rule 1 | As written, an LLM's tag would close a finding. The rewrite lets only a deterministic check close one | AD-1.9 |
| #33 | Commit scaffolding before implementation | Refine | Vision, open question | Committing invariants and failing tests before the code makes any later change to them visible in the diff (`docs/VISION.md:114`) | AD-1.10 |
| #34 | Admin/governance reclassification of skills | **Drop** | | Done on `main`. Phases 1 to 5 ship, apart from two residues. The two small residues move to AD-1.11 | |
| #35 | Field report: governance-skill load | Refine | Vision, "Who reads what" | #34 says it closes #35 "conceptually", but lessons 1, 3, 4 and 6 are not done on `main` | AD-1.11 |
| #36 | Plugin referential-integrity gap | Refine | PB-1 | No check resolves a slash-reference, and `crosscheck/docs/skills.md` has drifted from `crosscheck/skills/` | PB-1.18 |
| #37 | `/draft-invariants` should read specs first | **Drop** | | Done on `main`: `crosscheck/skills/draft-invariants/SKILL.md:74` to `:128` | |
| #38 | assurance-probe, a deterministic test-strength layer | Refine | Rule 3, vision Tests class | Phase 1 ships, but the agent runs the tests, not the script, so the result does not rerun without the agent | AD-1.12 |
| #39 | Component-correct verification misses integration gaps | Refine | Rules 1, 3, 7 | The certificate said "verified" and "HIGH" on a fix that did not work, and no rerunnable test caught it | VA-1.11, AD-1.13 |
| #40 | Incomplete verification treated as sufficient | Refine | VA-1, rules 1, 7 | An incomplete LLM certificate was reported as "verified as correct with high confidence" | VA-1.11 |
| #41 | Field report: wistful-pet | Refine | Rules 1, 3, 7 | The trace missed a filter, and the proposed fix had the same race. A person caught both | VA-1.11, AD-1.13 |

## Refined issues

Each section gives the text to put in the issue's body in place of the current text, and the queue row. The original body stays in the issue's history.

### #19 → AD-1.2

> **Auditor: write a rerunnable record next to the report, at one decided path.** Serves rule 3 of `docs/VISION.md`: everything reruns. `crosscheck/agents/auditor.md:184` writes only `.assurance/audit/<date>-audit.md`, and `:273` repeats it. The archived M4 spec (`crosscheck/docs/add/.retrospective/v1-archive/specs/modules/M4-auditor.md:202` to `:207`) asked for `docs/add/audit/<pass_id>.{md,json}`, and its JSON sidecar was the input to later passes. `auditor.md` discloses two other departures from M4 (`:63` to `:69`, `:232` to `:239`) but not this one.
>
> Acceptance:
> - Decide the path, and record why in `crosscheck/docs/add/JOURNAL.md`. Change `auditor.md:184` and `:273` to match.
> - Write a JSON sidecar holding the deterministic signals behind each verdict, with the command that recomputes them, so that a person can rerun a pass and compare. If ER-1's evidence record format fits, use it.
> - If a departure from M4 remains, state it under "What this audit does NOT catch".
>
> `auditor.md` is a Class A protected surface, so the pull request needs a governance note.

### #20 → VA-1.9

> **Auditor: never report `settled` for an artefact the auditor could not check.** Serves VA-1 and rule 7: every claim names its strength. `crosscheck/agents/auditor.md` has three verdicts (`:67` to `:68`). On `docs/invariants/*.md`, which carry no frontmatter, three of the five drift criteria "pass *vacuously*, not *cleanly*" (`:118` to `:129`), and the artefact still gets `settled`. `:60` says every audited artefact gets one verdict, and `:241` to `:242` say only flagged artefacts are judged. Together, these two leave the verdict unable to tell "clean" from "never examined". `:118` to `:129` disclose the vacuous pass in prose only.
>
> Acceptance:
> - Add a verdict, such as `unaudited`, for an artefact whose criteria cannot be computed. List it in its own rows of the verdict table.
> - Define the scan scope and the audited set, and give the verdict for an artefact in scope that no signal flagged.
> - Add a fixture: a metadata-less `docs/invariants/*.md` file that must not get `settled`.
> - Update `CLAIM-AUDITOR` in `crosscheck/conformance/claims.json`.
>
> Class A protected surface: governance note required.

### #21 → AD-1.3

> **lowry: enter the loop when any D2 gate is red, not only the build.** Serves rule 1: deterministic checks decide. `crosscheck/agents/lowry.md:64` to `:66` hands back with "Nothing to drive — the build is green". D2 lists four independent gates: build, unit tests, bidirectional invariant coverage and the conformance oracle. Its motivating case is a red coverage gate on green code, and D1 turns that case away. `crosscheck/docs/add/phase4-design-decisions.md:12` has the same build-only wording.
>
> Acceptance:
> - D1 enters when any D2 gate is red, and hands back only when all four are green.
> - The hand-back message names the gates it ran.
> - Mirror the change in `phase4-design-decisions.md` D1. Keep the D2.2 carve-out for aspirational and `not_implemented` invariants.
>
> Class A protected surface: governance note required.

### #22 → VA-1.10

> **ADD acceptance oracles: no LLM judge as a pass condition.** Serves VA-1 and rule 1. `crosscheck/conformance/acceptance/oracles.go:21` and `:33` define a Judged oracle as one whose transcript an LLM judge scores, and `:152` to `:153` makes every Judged oracle (A1 greenfield, A2 bootstrap, A5 drift-stop, A6 completeness, `:56` to `:66`) wait for "the scenario runner + LLM judge". Rule 1 (`docs/VISION.md:48`) says no guarantee rests on an LLM's judgment, so an LLM judge cannot make these oracles pass. CI runs `go test ./...` without `-tags acceptance` (`.github/workflows/ci.yml:44`), so none of the six oracles runs in CI.
>
> Acceptance:
> - Remove "LLM judge" as the pass condition from `oracles.go`, `RATIFY.md` and the oracle files. Split each Judged oracle into the parts a deterministic check decides and the parts a named person judges, which the vision calls `judged` strength. An LLM may draft a transcript or point at a likely failure. Its output is never the verdict.
> - Mechanise A5 (drift-stop): a deterministic check that fails when a commit in lowry's loop weakens a ratified invariant. `ClassifyCommitShape` (`oracles.go:105` to `:127`) reads only the commit subject and the amendment line, so the check must also read the diff of the invariant and test files, with the commit shape as one input.
> - Run the deterministic acceptance oracles in CI. The mode-selection function is AD-1.4.
> - Keep the `claims.json` entries for lowry and the auditor true as the oracles move.
>
> `.github/workflows/**` is Class A, so wiring CI needs a governance note.

### #23 → AD-1.4

> **ADD Step 0: choose the operating mode with a pure function of repository state.** Serves rule 3: the same repository gives the same mode on every run. `crosscheck/agents/add-orchestrator.md:133` and `crosscheck/docs/add/operating-modes.md:23` use "near-empty" with no definition. A thin repository is both "existing code" and "near-empty", and the ambiguity rule (`add-orchestrator.md:140` to `:141`) covers only code with a candidate spec. The bootstrap branch (`:125` to `:132`) names no next step. Step 1's zero-candidate fallback (`:165` to `:172`) chooses a mode inline and skips Step 0's ambiguity rule.
>
> Acceptance:
> - Define "near-empty" by a mechanical predicate, such as no source files outside a stated scaffolding list.
> - Implement mode selection as a pure function over a repository listing, with table tests for each mode and for the ambiguous cases. This is the deterministic part of #22.
> - Every Step 0 branch names its next step. Step 1's fallback calls the same function, and no path returns to Step 1.
> - When the function returns "ambiguous", the orchestrator asks one bundled question.
>
> Class A protected surface: governance note required.

### #24 → PB-1.19

> **Conformance AUTO 5: read the routing forms that `documented()` accepts, and agent-to-agent edges.** Serves PB-1: deterministic CI. `crosscheck/conformance/main.go:47` to `:48` match only `` `/x` `` and `/crosscheck:x`. `documented()` (`:241` to `:246`) also accepts a bare `/x `. AUTO 5 (`:361` to `:369`) flags every backticked bare `/name` that is not in `known`, so a bare reference to another plugin's skill fails, and it never extracts a bare agent name such as `byfuglien`, `hellebuyck`, `lowry` or `auditor`.
>
> Acceptance:
> - Extract the bare-slash form.
> - Resolve references to other plugins against the allowlist that PB-1.18 adds. Do not add a second list.
> - Detect agent-to-agent edges against the agent set.
> - Add a regression test for each of the three cases, and check by mutation that each test fails without its fix.
> - Narrow the "does NOT catch" text in the conformance README and in `CLAIM-SELF-COVERAGE`.

### #25 → PB-1.20

> **Conformance ledger: reject an unknown claim status.** Serves PB-1 and rule 7. The ledger loop (`crosscheck/conformance/main.go:400` to `:404`) handles only `unreviewed` and `known-gap`. Any other string, `reviewed-disclsed` for example, passes. `claims.json` uses `reviewed-disclosed` and `reviewed-accurate`.
>
> Acceptance:
> - An allowlist of `unreviewed`, `known-gap`, `reviewed-disclosed` and `reviewed-accurate`.
> - An error for any other status, and for an empty one.
> - A unit test with a typo'd status. Document the allowlist in the header comment.

### #26 → PB-1.21

> **Invariant headings: one grammar for the gate, the templates and the real docs.** Serves PB-1: a deterministic check that passes while the repository's own docs break it gives false assurance. `HEADER_RE` accepts only `I` and `COMMENT_RE` accepts `[A-Z]+` in `crosscheck/skills/invariant-coverage-scaffold/references/python-template.md:19` to `:20`, `typescript-template.md:15` to `:16` and `go-template.md:27` to `:28`. Every real invariant doc uses `### I1 — Name`, which `add-orchestrator.md:335` says fails the gate:
> - `crosscheck/docs/invariants/extractDifficultyMetrics.md:20`
> - `parseDafnyOutput.md:16`
> - `shouldExclude.md:18`
> - `crosscheck/skills/journal-context/docs/invariants/journal-context.md:12`
>
> `crosscheck/conformance/heading_grammar_test.go` checks only a synthetic corpus and never `COMMENT_RE`. `crosscheck/docs/examples/workflows/example.md:55` and `:107` still use `**Q1.`.
>
> Acceptance:
> - Pick one alphabet, and make the header pattern, the comment pattern and the `add-orchestrator` grep agree.
> - Migrate the four docs, or bless the h3 form everywhere.
> - The guard test reads the real `docs/invariants/*.md` files and exercises the comment pattern.
> - Migrate the example.
>
> The invariant docs are Class B and the skill and agent files are Class A, so the pull request needs a governance note.

### #27 → AD-1.1

> **AD-1: review the imported Crosscheck backlog against the vision.** This issue began as the epic that closed the ADD design-vs-shipped gap. No claim in `crosscheck/conformance/claims.json` is `known-gap` any more: six are `reviewed-disclosed` and one is `reviewed-accurate`. That says the gaps are disclosed, not that each child issue is closed. It is now the tracker for roadmap item AD-1. The decisions for #19 to #41 are in `intent/2026-10-07-backlog-review-decisions.md`, and each refined issue has a row in `docs/TASKS.md`. The `tracked_in` links in `claims.json` still point at `nicholls-inc/claude-code-marketplace` issues. That is recorded here, not changed.

### #28 → AD-1.5

> **add-orchestrator: state when ADD pays for itself.** Serves the vision's statement that the cost of writing proofs and models is a hard limit (`docs/VISION.md:93`). `crosscheck/agents/add-orchestrator.md:11` to `:12` triggers on "drive ADD" and similar phrases, and it states no criterion. `crosscheck/docs/add/orchestrator-improvements.md:63` records the change as improvement 6.
>
> Acceptance:
> - The preamble names the trigger: a feature with recurring bugs of one class and no behavioural contract.
> - It also names the anti-pattern: a one-shot bug in a feature with a coherent contract.
> - A `crosscheck/docs/add/JOURNAL.md` entry links the field report.
>
> Class A protected surface: governance note required.

### #29 → AD-1.6

> **add-orchestrator: name the modules to probe before probing, for a person to confirm.** Serves rule 1: humans decide. Today `/spec-adversary` is surfaced only after the audit, in Step 11, as a closing recommendation of the two coverage-thinnest modules (`crosscheck/agents/add-orchestrator.md:385` to `:389`, `:555`). It is not a confirmed choice made before the probe runs, and the selection is not recorded.
>
> Acceptance:
> - The audit step's output has a `recommended-probes` section naming 1 or 2 modules. Each choice cites a count the run computed (findings per module, a declared risk tag), not an LLM rating.
> - A documented per-run cap.
> - The operator can override the choice before dispatch.
>
> `/spec-adversary` stays a search tool (VA-1). Class A protected surface: governance note required.

### #30 → AD-1.7 (depends on AD-1.12)

> **add-orchestrator: retrofit `Invariant` comments only where a mutant shows the test checks the invariant.** Serves the vision's Tests class: how to prove that a test checks what it claims to check (`docs/VISION.md:89`). A comment alone makes the bidirectional coverage gate pass on presence. Retrofitting comments onto existing tests would add coverage claims that nothing checks.
>
> Acceptance:
> - The apply step proposes candidate tests for each new invariant.
> - It adds the comment only when AD-1.12's probe shows the test killing a mutant derived from the invariant's failure condition. It records the mutant and the command.
> - The batch is shown in the pull request for the person to accept.
>
> Class A protected surface: governance note required.

### #31 → AD-1.8

> **add-orchestrator: measure, before apply, what an invariant's governance hook does to the whole suite.** Serves rules 1 and 3: a deterministic, rerunnable count informs the person's decision.
>
> Acceptance:
> - Triage marks an invariant whose `Governance:` hook is repository-wide (a pre-commit listener, a lint rule, a fixture validator) as `preflight: required`.
> - The hook runs in a probe worktree, and the triage block records the command and the number of failing tests or files.
> - The operator chooses: accept now, narrow the invariant, or defer.
>
> Class A protected surface: governance note required.

### #32 → AD-1.9

> **ADD triage: close a finding without a person only when a deterministic check confirms the fix.** Serves rule 1. As written, the issue had an LLM tag each finding `mechanical` or `judgement` and close the mechanical ones itself. That tag is LLM judgment.
>
> Acceptance:
> - A finding may be auto-applied only when a deterministic check confirms the closure, for example "invariant Y has no `Governance:` line", which a grep decides. The checks come from a committed, fixed list, not from one the orchestrator writes at run time.
> - Every other finding goes to the person, with any LLM tag shown as a suggestion.
> - The pull request lists the auto-applied closures in one section, and the merge is the human decision.
> - Affects `add-orchestrator.md`, `audit-spec-coverage`, `audit-invariant-consistency` and `spec-adversary`.
>
> Class A protected surfaces: governance note required.

### #33 → AD-1.10

> **add-orchestrator: commit the spec, the invariants and the failing tests before implementation starts.** Serves the vision's open question "How to keep the AI that writes the code from also writing the checks that grade it" (`docs/VISION.md:114`). A scaffolding commit fixes the checks before the code exists. Any later change to them then shows in the diff, and the field test also showed it saves work when a session fails.
>
> Acceptance:
> - A scaffolding-commit step between triage close and implementation dispatch, with a stated artefact set: spec, invariants, failing tests and per-file plan.
> - Implementation does not dispatch without it.
> - The pull request body lists any change the implementation made to the scaffolding files.
> - A journal entry records the rationale.
>
> Class A protected surface: governance note required.

### #35 → AD-1.11

> **Crosscheck skills: ask the person only what needs a person.** Serves "Who reads what" (`docs/VISION.md:95`) and rule 1: a person decides judgments, and the agent does the rest. #34's reclassification shipped. These lessons from the 2026-05-12 field report are not done on `main`:
> 1. A session-aware mode that takes the recommended option for a non-blocking question and states it as reversible.
> 2. A per-skill budget of `AskUserQuestion` calls.
> 3. The `/spec-adversary` default cap (3, `crosscheck/skills/spec-adversary/SKILL.md:47`, `:143`).
> 4. Skills that still end by sending the person to the next skill (`suggest-specs`, `rationale`, `draft-invariants`).
>
> Two residues of #34 are also done here: `crosscheck/skills/lean-spec/SKILL.md:175` still says "Ask the user:" before the failure-artefact rule, and `crosscheck/skills/assurance-probe/SKILL.md:230` keeps a manual byfuglien trigger.
>
> Not carried: lesson 2, which would let `/protected-surface-amend` create a roadmap item itself. It contradicts `.claude/rules/protected-surfaces.md:159` to `:160` and the vision's rule that every claim traces up to a requirement a person approved (`docs/VISION.md:11`). Lesson 8 asks for a change to the field-report plugin, which lives outside this repository.
>
> Class A protected surfaces: governance note required.

### #36 → PB-1.18

> **Check every slash-reference in Crosscheck's Markdown, in pre-commit and in CI.** Serves PB-1 and its dual-track principle. No check resolves `/name` in `crosscheck/skills/*/SKILL.md` against the skill directories. AUTO 2 covers five doc files, and AUTO 5 covers agent bodies. `crosscheck/docs/skills.md:3` says "all 29 skills", but `crosscheck/skills/` has 30, and `journal-context` is missing. One reference does not resolve: `` `/journal-lint` `` at `crosscheck/skills/journal-context/SKILL.md:30`.
>
> Acceptance:
> - A checker, run by the pre-commit hook and by CI, resolves each unprefixed `/name` in `crosscheck/**/*.md` against `crosscheck/skills/` and `crosscheck/agents/`, and each `/<plugin>:<skill>` against an allowlist file, empty by default.
> - It errors on an unresolved reference, with file and line.
> - `crosscheck/docs/skills.md` is generated from `crosscheck/skills/`, and CI fails on drift.
> - A fixture with a broken reference fails the check.
>
> `scripts/ci/**` and `.github/workflows/**` are Class A, so the pull request needs a governance note. PB-1.19 reuses the allowlist.

### #38 → AD-1.12

> **assurance-probe: make the mutation probe a script that reruns.** Serves rule 3 and the vision's Tests class (`docs/VISION.md:89`). Phase 1 ships as `crosscheck/skills/assurance-probe/` (`SKILL.md:18`, `:26` to `:30`). The mutation code in `lib/mutations.py` is deterministic, but it never runs the covering test: the agent following `SKILL.md` runs it. So a killed or survived row cannot be rerun without the agent. Open question 1 in this issue was answered "yes commit the reproducer script", but committing it is an opt-in flag (`SKILL.md:223`).
>
> Acceptance:
> - One command applies each mutant, runs the covering test, restores the source, and writes `killed`, `survived` or `errored` with the test command and any seed.
> - Running it twice on one commit gives the same rows.
> - The reproducer is committed by default, under one directory that CI and pre-commit can exclude.
> - Phases 2 and 3 stay gated as `references/phase-gating.md` says.
>
> `SKILL.md` is Class A: governance note required. If the maintainer opens a roadmap item for the vision's Tests class, this row moves under it.

### #39, #40, #41 → VA-1.11

> **Reasoning skills: label the certificate as search output, and never as "verified".** Serves VA-1 and rules 1 and 7. `docs/VISION.md:103` names the semi-formal reasoning skills as search tools whose output is never evidence. `crosscheck/skills/reason/SKILL.md:47`, `:68`, `:75` and `:154` call a premise "verified by reading code" and define HIGH confidence as "All premises verified". `trace-execution/SKILL.md:66` and `locate-fault/SKILL.md:41` repeat this. `crosscheck/agents/byfuglien.md` has a certificate-completeness gate that re-executes a skill (`:130` to `:155`), but no rule for a certificate that is still incomplete after re-execution, which is the case in #40. `crosscheck/README.md:35` says "Use the byfuglien agent to verify your bug fix". In #40, an interrupted certificate was reported as "verified as correct with high confidence". In #39 and #41, a "HIGH" certificate was wrong.
>
> Acceptance:
> - These skills and byfuglien describe their output as a search result that points at likely problems.
> - A certificate missing a required step says it is incomplete and gives no confidence.
> - The words "verified" and "correct" are not used as verdicts, and the README example is reworded.
>
> Class A protected surfaces: governance note required.

### #39, #41 → AD-1.13

> **Reasoning skills: turn the reported failure into a test that reruns.** Serves rules 1 and 3: the certificate points, and a rerunnable test decides. In #39, a fix passed a `/reason` certificate and still failed in the repository with zero remotes. In #41, a trace missed a filter in the entry function, and the proposed fix had the race it was meant to remove. A person caught both.
>
> Acceptance:
> - When `/reason`, `/trace-execution` or `/locate-fault` is used on a reported bug, it first asks for, or drafts, a test that reproduces the reported scenario end to end and fails before the fix. A `[BEHAVIORAL]` claim names that test and its command.
> - It treats the proposed fix as a new claim and checks it for the same bug pattern.
> - It reads each function on the traced path in full, and lowers its stated confidence when it did not.
> - When a prior certificate on the same bug exists, its alternative hypotheses are rechecked.
>
> The test, passing after the fix, is the evidence (`tested`). The certificate is not. Class A protected surfaces: governance note required.

## Dropped issues

### #34: done on `main`, apart from two residues

Every phase of the reclassification has shipped. Each item below is quoted from `origin/main` at `e5089f7`:

- **Phase 1.** `crosscheck/docs/orchestrator-coordination.md` covers markers (§1), findings files (§2) and work directories (§3).
- **Phase 2.**
  - `crosscheck/skills/protected-surface-amend/SKILL.md:8` to `:9`: "Primarily agent-invocable … reads the staged diff".
  - `crosscheck/skills/assurance-init/SKILL.md:64` reads `.assurance/layer-audit-result.json`.
  - `crosscheck/skills/suggest-specs/SKILL.md:93` writes `.assurance/suggest-specs-queue.json`.
- **Phase 3.**
  - `crosscheck/skills/draft-invariants/SKILL.md:365` to `:372` emits a governance fragment.
  - `crosscheck/skills/lean-spec/SKILL.md:78` and `crosscheck/skills/lean-impl/SKILL.md:83` say "Do **not** stop to ask the user to confirm".
  - `crosscheck/skills/rationale/SKILL.md:25` writes to `.crosscheck/work/rationale/`.
- **Phase 4.**
  - `crosscheck/skills/check-regressions/SKILL.md:116` to `:136` splits "Actions Taken" from "Decisions for Review".
  - `crosscheck/skills/spec-iterate/SKILL.md:104`, `crosscheck/skills/generate-verified/SKILL.md:76` and `crosscheck/skills/lean-spec/SKILL.md:177` write a failure artefact after five attempts.
  - `crosscheck/skills/extract-code/SKILL.md:33` detects the language.
  - `crosscheck/skills/informal-spec/SKILL.md:184` to `:192` writes an orchestrator-readable marker.
- **Phase 5.**
  - `crosscheck/skills/spec-adversary/SKILL.md:151`, `crosscheck/skills/assurance-roadmap-check/SKILL.md:111` and `crosscheck/skills/assurance-status/SKILL.md:171` write findings files.
  - `crosscheck/skills/intent-check/SKILL.md:208` has "What this does NOT catch".

Two small residues remain, at `lean-spec/SKILL.md:175` and `assurance-probe/SKILL.md:230`. They move to AD-1.11, together with #35's open lessons. Close #34 with a link to this section, and say in the close comment that the two residues stay open in AD-1.11.

### #37: done on `main`

`crosscheck/skills/draft-invariants/SKILL.md` already does each change the field report asks for:

- **Read the spec first.** `:74`: "Spec discovery (do this FIRST, before any elicitation)".
- **Mine the audit table.** `:87` to `:89` and `:98`.
- **Quote, don't paraphrase.** `:99` to `:101` ("Do not paraphrase — quote") and the refuse-and-redirect rule at `:112` to `:117`.
- **Elicit cold only as a fallback.** `:128`.

Close #37 with a link to this section.
