# crosscheck/JOURNAL.md

Journal for the Crosscheck plugin. Decisions that affect skills, agents, the MCP server, or the plugin's overall shape land here. Narrower work (a specific skill, a Docker image, the Lean pipeline) may earn its own deeper shard when there's enough to say; for now this is the only journal under `crosscheck/`. Entries newest first. The repo-root [AGENTS.md](../AGENTS.md) walk-up rule sends agents through this file before touching anything below it.

---

## 2026-10-07 — One invariant-heading grammar for the gate, the templates and the real docs

**Type:** fix
**Touches:** conformance/heading_grammar_test.go, skills/invariant-coverage-scaffold/references/{python,go,typescript}-template.md, docs/examples/workflows/tier-a/check_invariant_coverage.py, docs/invariants/*.md, skills/journal-context/docs/invariants/journal-context.md, docs/examples/workflows/example.md, ../docs/TASKS.md
**Why:** The coverage gate's comment pattern read any upper-case ID prefix while its header pattern read only `I`, and every real invariant doc used `### I1 — Name`, which the `add-orchestrator` grep counts as zero invariants. The guard test passed because it never read a real doc or a comment pattern (#26).
**Links:** [intent](../intent/2026-10-07-invariant-heading-grammar.md), [spec](../intent/2026-10-07-invariant-heading-grammar-spec.md), [plan](../intent/2026-10-07-invariant-heading-grammar-plan.md)

The alphabet is `I`. The header pattern, the `add-orchestrator` grep and `draft-invariants` already used it, so the comment pattern narrowed to match rather than the grammar widening. The four invariant docs and the worked example now use `## I<N>: <Name>`. `heading_grammar_test.go` extracts each template's comment pattern and checks it against its header pattern over a list of IDs, and reads every `docs/invariants/*.md` in the repository for a heading in any other form. Eight mutants, among them the old comment pattern and one h3 heading, each fail a test. A comment with another prefix is now ignored silently (PB-1.26), and the tier-b example parses a third form, `## Invariant <ID>:` (PB-1.27).

---

## 2026-10-07 — `dafny_evidence` checks that `requirement` names a tracked file, and refuses a theorem named twice

**Type:** feature
**Touches:** mcp-server/src/tools/evidence.ts, mcp-server/src/index.ts, mcp-server/dist/index.js, ../intent/2026-10-06-dafny-evidence-record-spec.md, ../docs/TASKS.md
**Why:** The evidence record format defines `requirement` as a repository path with an optional anchor, but DE-1 accepted any non-blank string, so a record could name a requirement no reader can open at `commit`. `theorems` accepted a name twice, so `basis.theorems` could repeat it.
**Links:** [intent](../intent/2026-10-07-dafny-evidence-inputs.md), [spec](../intent/2026-10-06-dafny-evidence-record-spec.md)

The task offered a choice: check the path, or describe the field as free text. The format already calls it a path, so the tool now checks it. DE-1 refuses a requirement whose path part, the trimmed text before the first `#`, has an empty, `.` or `..` segment or a `\`, which covers an absolute path, and one that ends in an empty anchor. DE-4 runs the same tracked-file checks on that path as on `file`, before any Dafny run, so with DE-3 it names a tracked regular file at `commit`. The anchor is not checked, because the format fixes no anchor syntax, and the spec flags that as not yet reached. DE-1 also refuses each theorem name that appears more than once.

---

## 2026-10-07 — `npm test` replays real Dafny output through `dafny_evidence`

**Type:** test
**Touches:** mcp-server/src/__tests__/fixtures/, mcp-server/src/__tests__/integration/evidence.dafny-output.integration.test.ts, mcp-server/src/__tests__/e2e/dafny-output.e2e.test.ts, ../intent/2026-10-06-dafny-evidence-record-spec.md, ../docs/TASKS.md
**Why:** `npm test` covered DE-5 to DE-8 only with Dafny output typed by hand, and no test ran the evidence record checker on a record the tool wrote.
**Links:** [intent](../intent/2026-10-07-dafny-evidence-real-output-tests.md), [spec](../intent/2026-10-06-dafny-evidence-record-spec.md)

Eight Dafny programs, from a proved lemma to an included `{:axiom}`, were run through `dafny_evidence` against the real image, and every Dafny run's arguments, exit code and output are committed as fixtures. The integration test replays them and refuses a run whose arguments differ, so a change to how the tool calls Dafny forces a new recording. The fixtures show two things the hand-written logs did not: a lemma that rests on an unproved include, and one with an `assume`, both appear as `Passed` in the verification log, so only verify's exit code refuses them. The e2e test rewrites the fixtures under `RECORD_DAFNY_FIXTURES=1` and otherwise fails when the image's output drifts from them. The rerun command still runs only in the e2e suite.

---

## 2026-10-07 — `dafny_evidence` reruns by image ID, and narrows its output and include paths

**Type:** feature
**Touches:** mcp-server/src/tools/evidence.ts, mcp-server/dist/index.js, ../intent/2026-10-06-dafny-evidence-record-spec.md, ../docs/TASKS.md
**Why:** The rerun command named the image tag, so a rebuilt or retagged image ran under the same command. The output path could land in a dot directory or a dot-named file that a hook or tool reads as configuration, and an include could be any file Dafny reads as source.
**Links:** [intent](../intent/2026-10-06-dafny-evidence-paths-and-image-id.md), [spec](../intent/2026-10-06-dafny-evidence-record-spec.md)

The rerun command now names the image by the ID the trusted base records. A probe showed that `docker run` with an ID no local image has exits 125 without a pull, that `docker save` and `docker load` keep the ID, and that a second build of the same Dockerfile does not. So the command runs the recorded image or fails, and an auditor on another machine loads the author's saved image first. A rerun with no help from the author waits on a published image with a pinned base, and the spec's concern says so. This reverses the earlier argument for the tag, which held that a command failing on another machine was worse than one running a different image there. `outputPath` must end in `.json` and have no part, directory or file name, that starts with `.`, which replaces the `.git` rule. An include must end in `.dfy`, because Dafny 4.11.0 reads `include "Lib.txt"` as source.

---

## 2026-10-07 — `dafny_evidence` and its rerun command run Dafny as `nobody` with no capabilities

**Type:** feature
**Touches:** mcp-server/src/docker.ts, mcp-server/src/tools/evidence.ts, mcp-server/dist/index.js, ../intent/2026-10-06-dafny-evidence-record-spec.md, ../docs/TASKS.md
**Why:** The Dafny runs of `dafny_evidence` and the rerun command a record hands an auditor ran the toolchain as root, with Docker's default capabilities and no process limit.
**Links:** [intent](../intent/2026-10-07-dafny-evidence-docker-hardening.md), [spec](../intent/2026-10-06-dafny-evidence-record-spec.md)

Both now pass `--cap-drop=ALL --security-opt=no-new-privileges --pids-limit=512 --user=65534:65534`, read from one list, `SANDBOX_FLAGS` in `docker.ts`, so the tool's runs and the rerun cannot drift. The user is set at run time rather than in the image, so the image and every earlier record's trusted base stay the same. The process limit counts threads too: a probe peaked at 23 under the tool's CPU limit and 186 with `--cores 64`, and a run over the limit exits 1. Running as `nobody` means a Linux tree that others cannot read makes Dafny fail and the tool refuse; a new concern in the spec records why the caller's own user was not used. `dafny_verify`, `dafny_compile` and the Lean tools keep their old flags, because `dafny_compile` writes into a mount that `nobody` may not be able to write on Linux.

---

## 2026-10-06 — The MCP tool `dafny_evidence` emits an evidence record, and no skill calls it yet

**Type:** feature
**Touches:** mcp-server/src/tools/evidence.ts, mcp-server/src/docker.ts, mcp-server/src/index.ts, mcp-server/dist/index.js, README.md, ../CLAUDE.md, ../docs/TASKS.md
**Why:** ER-1's acceptance asks for one Crosscheck pipeline to emit an evidence record. `dafny_verify` returned success on a source string, with no commit, no trusted base and no rerun command.
**Links:** [intent](../intent/2026-10-06-dafny-evidence-record.md), [spec](../intent/2026-10-06-dafny-evidence-record-spec.md), #80, #81

The new MCP tool `dafny_evidence` takes a committed `.dfy` file in a clean work tree and the theorems that prove a statement. It runs `dafny verify` and `dafny audit` with the work tree mounted, and emits a record with one `proved` claim, or refuses. The audit is the load-bearing part: a probe showed that a bodiless `lemma {:axiom}` passes `dafny verify` with exit 0, and that `dafny audit` reports it but also exits 0, so the tool and the rerun command both require a line that is exactly "Dafny auditor completed with 0 findings". The trusted base names the Dafny version the image reports, the Z3 in that release, and the local image ID. The rerun command names the image tag, not a digest, so a later rebuild can change what it runs. Nothing checks that the statement matches the contracts, and the record does not say whether a person reviewed it.

Review decisions on the same PR tightened it, under one rule: the record must be true of what was checked, so a gap that is cheap to close is closed rather than documented. Theorem names must be fully qualified, as the `proved` row of the evidence record spec's basis table asks, and the tool reads them from Dafny's own verification log rather than matching the source, which also retires the lexical check. An include that leaves the tracked files refuses (DE-12). The image ID is read before the runs and every run uses it, and HEAD and the tree are checked again afterwards (DE-13). `outputPath` stays inside the work tree (DE-11). Neither Dafny run in the rerun command is in a pipe, so the audit's exit code counts (DE-9). The round 3 review then showed that `dafny verify` checks no proof in an included file and that `dafny audit` reports only the files it is named, so a theorem resting on an included false lemma or `{:axiom}` passed both. The tool now verifies with `--verify-included-files`, audits every included file, mounts the tree read-only, and overwrites only an earlier evidence record. Round 4 found that `git status` hides an edit to a file flagged `assume-unchanged` or `skip-worktree`, so the clean-tree check now reads `git ls-files -v` as well, and the record is written to a temporary file and renamed into place after the output path is checked again.

The Dafny pipeline as a user runs it does not emit a record yet. The MCP tool does, when something calls it, but no skill calls it, because the skills are protected surfaces. So the ER-1.3 row now reads "The MCP server emits an evidence record for a Dafny run (`dafny_evidence`)", and ER-1.6 carries ER-1's goal: one Crosscheck pipeline, `/generate-verified`, emits a record by calling `dafny_evidence` (#80). The maintainer's convergence round made the audit match exact, and filed the remaining review items as ER-1.6 to ER-1.10: the wiring, Docker hardening, tighter `outputPath`, include and rerun-image rules, tests that run the evidence record checker and real Dafny outside the e2e suite, and a decision on mounting a `git archive` copy. The last round's low findings became ER-1.11 (the `requirement` description and duplicate theorem names), ER-1.12 (an include's `..` through a linked directory) and ER-1.13 (an inherited `GIT_DIR`, and conversion filters that `git status` hides). DE-3 and DE-12 now say those gaps are not yet reached.

---

## 2026-10-06 — The research doc and the reference workflows say "not yet reached"

**Type:** docs
**Touches:** docs/research/assurance-hierarchy.md, docs/examples/workflows/example.md, docs/examples/workflows/tier-b/assurance-squad.md, docs/examples/workflows/tier-b/assurance-pr-gate.md, docs/orchestrator-coordination.md, ../docs/TASKS.md
**Why:** VA-1.2 changed the README and the hierarchy guide, but the guide's "full treatment" still called Layer 6 "best-effort", said no theorem can prove a spec complete, and called performance, partition failures and security "out of scope for the hierarchy entirely". The reference workflows labelled Layer 6 issues `Layer 6 (best-effort)`.
**Links:** [intent](../intent/2026-10-06-not-yet-reached-research-docs.md)

The research doc's Scope section, its Layer 1, 2, 3 and 6 text, and its "What this hierarchy is not good for" section (now "Where this hierarchy does not reach yet") use the wording VA-1.2 gave the README. The workflows label Layer 6 "search only". Dated research records, the ADR, the reports and the ADD retrospectives keep their wording, since they record what was believed at the time. The research doc's Layer 5 confidence stays for VA-1.7.

---

## 2026-10-06 — The README and the hierarchy guide say "not yet reached"

**Type:** docs
**Touches:** README.md, docs/assurance-hierarchy.md, ../docs/TASKS.md
**Why:** `docs/VISION.md` says no class of code is outside the vision and that spec completeness is provable relative to a formal requirement. The README called performance, partition failures and security "out of scope", Layers 2 and 3 "deliberately not addressed", and Layer 6 "best-effort".
**Links:** [intent](../intent/2026-10-06-not-yet-reached-docs.md)

Each of those statements now says "not yet reached", names the blocking property, and names the open question, taken from the vision's class table where one fits. Layer 6 points at roadmap item RQ-1 and calls `/spec-adversary` a search. "What Crosscheck is not good for" became "Where Crosscheck does not reach yet". The research doc still has the old wording (VA-1.6), and both files still give Layer 5 the `/intent-check` accuracy as a confidence (VA-1.4). The Layer 3 row of the hierarchy table now says what is reached (CGV) and what is not. Its blocking property names Dafny-verified units, since CGV does check declared contracts along Python data paths. It calls the CGV extractor untrusted but auditable, matching `cgv/README.md`, and says a kernel replay of the built environment is not yet reached (TB-1). Agents and skills still say "best-effort" in places, and they are protected surfaces, so VA-1.5 owns them.

---

## 2026-05-14 — release pipeline: drain the 2.4.0 → 2.5.0 backlog and harden the commit convention

**Type:** release-process / governance
**Touches:** [CLAUDE.md](../CLAUDE.md) (commit conventions), [.husky/commit-msg](../.husky/commit-msg) (enforcement), this entry triggers `Release-As: 2.5.0` for the crosscheck component.
**Why:** Sixteen commits landed after the 2.4.0 release without producing a release PR. The Release workflow ran every time and reported `No user facing commits found since c62ea3c... - skipping`. Cause: every one of those commits was `refactor(crosscheck): …` — release-please's default semver rules only treat `feat:` and `fix:` (and `feat!:` / `BREAKING CHANGE`) as user-facing. `refactor:` is silently ignored for versioning. The previous convention allowed `refactor:` for behavioral changes to `SKILL.md` / `agents/*.md`, which is exactly the failure mode that produced the backlog.

The fix is two-track. **Convention:** behavior changes to behavioral artifacts must be `feat:` (new behavior) or `fix:` (corrective). `refactor:` is reserved for structural changes that genuinely do not alter behavior — and when they touch a behavioral artifact, they must be split into a separate commit that does not. **Enforcement:** `.husky/commit-msg` now blocks `refactor:` on behavioral artifacts in the same way it already blocks `docs:`, with an error message that names release-please as the reason. Without enforcement, the convention is just prose and the same drift recurs.

This entry is also the carrier for the `Release-As: 2.5.0` trailer on the merging PR — it touches `crosscheck/` so release-please attributes the trailer to the crosscheck component, and it documents the rationale in the place a future maintainer will look when the next release misbehaves.

---

## 2026-05-11 — /rationale: FORMAL routing Layer 1 vs Layer 4

**Type:** propagated-discovery
**Touches:** skills/rationale/SKILL.md (Steps 3, 4, 6)
**Why:** Lands the snapshot's §4 FORMAL design into the operational prompt. Before this PR, every FORMAL leaf was routed through a single discharge path — *"draft Dafny spec → offer `/spec-iterate`"* — with no distinction between code that's a Layer 1 candidate (pure, ships as Dafny extraction) and code that needs Layer 4 (effectful/networked/concurrent, Lean as DRT oracle, production code stays as-is). The snapshot named both routes and the picking heuristic; SKILL.md was still pointing only at Layer 1.
**Links:** [snapshot §4](docs/specs/rationale-2026-05-11.md), parent snapshot PR (#169), C0 branch PR (#173)

Step 3's classification table widens the `[FORMAL]` verification-method cell from a single Dafny route into two layer-tagged routes (Layer 1 Dafny, Layer 4 Lean pipeline). Classification guidelines gain a *FORMAL routing* line that names the purity/effect-profile heuristic — pure-functional shape → Layer 1, effectful/networked/concurrent/shipping-floats → Layer 4.

Step 4's `[FORMAL]` section is restructured around the two discharge routes. *Layer 1* keeps the existing draft-Dafny-spec mechanics. *Layer 4* names the full Lean pipeline (`/lean-spec` → `/lean-impl` → `/correspondence-review` → `/drt-oracle`), is explicit that the Lean model is **not** shipped, and marks the leaf verified only after `/drt-oracle` reports clean. A new *Picking the route* paragraph encodes the heuristic and instructs the skill to ask the user when ambiguous rather than guess.

Step 6's verification-checklist `[FORMAL]` bullet widens accordingly — routing-by-profile is now an explicit gate the skill self-checks before delivery.

Step 5's worked example is intentionally untouched: the sort case is a clean Layer 1 example, so the existing verification results (`dafny_verify` discharge) still match. A worked Layer 4 example would be a follow-up addition once a candidate effectful module is in hand; not part of this PR.

---

## 2026-05-11 — /rationale: promote trust boundaries to C0 top-level branch

**Type:** propagated-discovery
**Touches:** skills/rationale/SKILL.md (Steps 2, 5, 6)
**Why:** Lands the snapshot's §2 design decision into the operational prompt. Before this PR, trust boundaries were a single footnote in the final verification checklist (*"Trust boundaries noted (Dafny limitations, extern methods, float precision)"*), buried below the structural soundness check. The snapshot reframed them as a first-class C0 branch because they bound what every other leaf can verify — a verified `sort.py` proof is meaningless if its comparison operator is `{:extern}`. SKILL.md was still pointing the other way.
**Links:** [snapshot §2](docs/specs/rationale-2026-05-11.md), parent snapshot PR (#169)

Step 2 gains a C0 trust-boundary branch alongside C1/C2/C3, with three generic leaf templates (external dependencies enumerated; domain limitations documented; trust assumptions stated). A new *Why C0 is first-class* paragraph names the conditioning relationship — every downstream claim is conditional on the C0 leaves, and that conditioning is now visible in the tree rather than implicit. Step 5's worked sort example gains two C0 leaves (no extern/IO/network as STATIC; `<=` totality + transitivity as SEMANTIC), and the summary table updates accordingly. Step 6's standalone *"Trust boundaries noted"* footnote bullet is replaced by *"C0 trust-boundary branch enumerated"* — making the check structural rather than ad-hoc.

---

## 2026-05-11 — /rationale snapshot: defer STATIC citation post-process

**Type:** correction
**Touches:** docs/specs/rationale-2026-05-11.md (§4 STATIC, §8 deferred-until-field-evidence)
**Why:** No fabricated `STATIC` citations observed in `/rationale` invocations to date. The snapshot's §4 declared the deterministic read-and-string-search post-process as part of the design; this is premature surface area against an unobserved failure mode. §8 already establishes the right pattern for this exact reflex — deferred until field evidence — applied there to tree completeness.
**Links:** [snapshot](docs/specs/rationale-2026-05-11.md), parent snapshot PR (#169), cascade PR (#170)

The §4 STATIC paragraph drops the post-process language and adds a one-line pointer to §8. §8 gains a second deferred concern ("STATIC citation honesty") with the same shape as the existing tree-completeness block — observed-failure trigger, on-shelf option (read-and-string-search), explicit cost framing. Strict v2 reading would make this a new dated snapshot superseding 2026-05-11; chose the lighter-touch amendment within the same week as merge, since this is correction of premature scope rather than design evolution. Downstream effect: the planned SKILL.md catch-up PR series drops from three to two (C0 trust-boundary branch and FORMAL Layer 1 vs Layer 4 routing remain).

---

## 2026-05-11 — /rationale Layer-4 docs cascade + status flip

**Type:** propagated-discovery
**Touches:** docs/specs/rationale-2026-05-11.md (status field), agents/byfuglien.md, agents/hellebuyck.md, docs/agents.md, docs/skills.md, docs/assurance-hierarchy.md
**Why:** Downstream from the snapshot merged at 3da376d. The snapshot reassigned `/rationale` from byfuglien (Layers 1–3) to hellebuyck (Layer 4 — semi-formal rationales); the agent pages, skill catalogue, agent overview, and assurance-hierarchy mapping all still pointed the other way. Also flips the snapshot's frontmatter from `Status: Draft` to `Status: Snapshot` per v2 methodology (`docs/add/.retrospective/findings-and-methodology-v2.md:218-223`) — Status: Snapshot = committed, and merge is the ratification signal.
**Links:** [snapshot](docs/specs/rationale-2026-05-11.md), [v2 methodology](docs/add/.retrospective/findings-and-methodology-v2.md), parent PR (#169)

Single PR, low risk. `/rationale` removed from byfuglien's skill tables, classification, skill-readme list, and Phase 4 quality gates; added to hellebuyck's new "Adequacy (Layer 4 — semi-formal rationales)" subsection with matching classification, skill-readme entry, and validate-output gates (claim tree soundness, classification accuracy, actionable output). `docs/agents.md` moves `/rationale` from byfuglien's "Spec management" bullet to hellebuyck's "Layer 4 (impl–spec alignment)" bullet and rewords the handoff seam — the `/rationale` → `/spec-adversary` chain is now intra-hellebuyck rather than byfuglien→hellebuyck; the seam stands. `docs/skills.md` reflows the "Spec management & adequacy" section into "Spec management" (byfuglien) and folds `/rationale` into the existing "Layer 4 (impl–spec alignment, semi-formal rationales)" section. `docs/assurance-hierarchy.md` extends the Layer 4 row with `/rationale` and adds a "When to use what" pointer. Snapshot text is left untouched apart from the status flip — its byfuglien.md:147 cross-reference becomes stale, but the snapshot is a dated artefact (v2 §3.3); stale line numbers are expected, content stands. SKILL.md catch-up (C0 trust branch, FORMAL Layer 1 vs Layer 4 routing, STATIC citation post-process) ships separately per the snapshot's own callout.

---

## 2026-05-11 — /rationale design-intent snapshot

**Type:** propagated-discovery
**Touches:** docs/specs/rationale-2026-05-11.md (new), docs/specs/ (new directory)
**Why:** Driving the v2 retrospective methodology against a real skill. `/rationale` shipped via SKILL.md a while back; the design intent behind it had never been written down. Snapshotting it now so future drift can be checked against something.
**Links:** [spec snapshot](docs/specs/rationale-2026-05-11.md), [v2 retrospective](docs/add/.retrospective/findings-and-methodology-v2.md), [SKILL.md](skills/rationale/SKILL.md)

First snapshot of `/rationale`'s design intent — what the skill is for, the seams with `/spec-iterate`, the Lean pipeline, `/spec-adversary`, `/intent-check`, and `/assurance-probe`, and what's deliberately not in scope. Seven design decisions land in this snapshot: (1) the skill sits at Layer 4 (semi-formal rationales) and moves from byfuglien to hellebuyck ownership; (2) FORMAL claims split into two discharge routes — Layer 1 → `/spec-iterate` for pure code shipping to production, Layer 4 → `/lean-spec`/`/lean-impl`/`/drt-oracle` for impl-vs-model verification (Lean is the model used as a DRT oracle, not shipped code); (3) STATIC citations get a fast deterministic post-process check (no LLM in the loop) that catches fabricated `file:line` references; (4) trust boundaries promote from a final-checklist bullet to a `C0` top-level branch so they're part of the argument structure rather than a footnote; (5) no on-disk persistence yet; (6) no in-skill chaining — orchestrators compose skills; (7) multi-target applicability is design intent — the goal-structured argument plus four-class leaf taxonomy are target-agnostic, but decomposition templates and FORMAL discharge routes are target-specific and extend together (implementation verification works today; spec analysis, acceptance-scenario adequacy, and others are downstream extensions). One question is *deferred until field evidence*: tree completeness — `/rationale-adversary` (sibling skill) and an in-skill completeness pass are both on the shelf; neither lands until invocations show the gap matters in practice. The Layer-4 reassignment implies cascading updates to `agents.md`, `skills.md`, `byfuglien.md`, and `hellebuyck.md` — downstream work, not part of this PR. First thing under `crosscheck/docs/specs/`, which lands the v2 §3.3 dated-snapshot pattern here as a side-effect.

---

## 2026-05-11 — v1 assurance-driven development stack archived [ADR-0001]

**Type:** retraction
**Touches:** docs/add/ (v1 archived, v3 starts), .assurance/ (one v1 output archived)
**Why:** The v1 design-doc stack inside `docs/add/` reproduced the failure mode it was meant to prevent, so we pulled it after one retrospective.
**Links:** [ADR-0001](../docs/decisions/0001-sharded-journal-architecture.md), [v2 retrospective](docs/add/.retrospective/findings-and-methodology-v2.md)

The v1 install was docs-only — no skill behaviour changed, no MCP-server code changed, nothing in `agents/` or `skills/` was rewired. So pulling it is cheap and contained to documentation. The retrospective under `docs/add/.retrospective/` is the place to read about what happened and where the design is heading; the archive under `.retrospective/v1-archive/` is for anyone hitting a stale link. The plugin runtime is otherwise unchanged.
