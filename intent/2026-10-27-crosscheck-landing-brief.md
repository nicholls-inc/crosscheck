# Crosscheck landing page — rejected first brief

The user rejected all three initial mockups on 2026-10-27 as gaudy and inconsistent with Crosscheck. The visual directions below are superseded and must not be used as a baseline. Start from scratch using docs/VISION.md, read in full, and https://www.are.na/harry-nicholls/crosscheck. Read the actual collection before choosing a replacement visual direction. The user requested https://mcp.are.na/mcp; initialization returned HTTP 401, so authenticated access is pending. The collection has not yet been reviewed. No replacement mockup has been generated or approved. The installation CTA, image-only scope and evidence boundaries remain confirmed.

Status: draft for visual-direction selection. Image mockups only; no website implementation. User-confirmed primary CTA: **Install the plugin**.

## Job and audience
Help developers evaluating Crosscheck understand how it checks AI-generated code, inspect a concrete library investigation, and install the Claude Code plugin. Visitor mode: Persuade. Narrower audience segmentation is not yet confirmed.

## Outcome and evidence
Lead with a plain explanation of the plugin and an immediately visible install action. The supporting action reads the EVerest case study. Ground that case study in [PR #144](https://github.com/nicholls-inc/crosscheck/pull/144), inspected at head `d41f08251ceeb70f53bc98d850fe56d70fc190c0`; it was open and unmerged when inspected on 2026-10-27.

The PR models OCPP 1.6 composite charging schedules in Lean, proves three model invariants, and compares EVerest's implementation with generated tests. It reports recurring profiles restarting one second early per leap second since their anchor. The observed impact is one to three seconds. This is a model-proof and implementation-testing case study, not a whole-library verification claim or endorsement by EVerest.

The full case study must expose the signed spec, model, correspondence review, report and reproduction commands. Preserve the PR's review qualifications: I4 is not proved; a spec correction needs confirmation; two correspondence verdicts were reclassified; Lean was checked locally; the upstream issue is a draft, not a filed or accepted report. Detailed qualifications belong with linked evidence, with a short visible scope statement beside the landing-page claim.

## Page sequence
1. Crosscheck wordmark, Case studies, Docs and GitHub navigation.
2. Product offer, a one-sentence mechanism, Install the plugin, and the first case-study link.
3. One substantial EVerest study: the behavior, the discrepancy, and separate Proved / Tested / Found statements. Start with one real study; add studies only when evidence is supplied.
4. Explain the selected workflow: specify the intended behavior, prove properties of the model, compare the implementation. Explain separately that Dafny can verify and extract code; the Lean/DRT path tests an existing implementation against a model.
5. Installation block with both actual marketplace commands, prerequisites link and final CTA.

```sh
claude plugin marketplace add nicholls-inc/claude-code-marketplace
claude plugin install crosscheck@nicholls
```

## Visual directions under review
- **The standard, made visible:** a cobalt technical-standards cover, enormous engineered sans lettering, direct install action, a quiet evidence index, and a white EVerest case-study region visible in the opening viewport. Risk: monumentality can outrun explanation; keep the plugin description prominent.
- **The engineering datasheet:** mineral white, forest ink and an acid-lime study band; the early restart becomes an explicitly illustrative timing diagram. Risk: familiar engineering territory, and charts must not imply invented measurements.
- **The category standard:** a white developer-tool page with a clear offer, restrained pipeline preview and substantial case study. Risk: less distinctive identity. Offered as an alternative, not a recommendation.

No direction is approved yet. Direction seed: `74eaab91`, assigned grounded candidate 5. The grounded list, ordered before presentation: engineering datasheet; systems research proceedings; precision test-instrument front panel; scientific atlas; technical standards cover; transit timetable; source-diff review. These span measurement, publication, physical controls, spatial navigation and code-review traditions. Avoid the category's black neon terminal and its warm editorial opposite.

The six dealt catalog systems were fused with product content and declined on audience identification and product clarity: cracktro queue (artifact text layers), collider display (concentric evidence), kiosk print (artifact cells), toy shelf (proof boxes), daylight section (evidence section), and fermentation calendar (verification batches). The standards direction takes their system disciplines: still foreground; one focal finding; committed page-scale color; progressive detail; a continuous evidence chain; explicit provenance. It borrows no decorative motifs.

## Layout, behavior and constraints for a later build
On desktop, keep offer and install within the opening viewport and reveal the first study immediately below. On mobile, order product name, offer, explanation, install, case-study link and evidence; stack tables as labeled rows, preserve chart labels, and avoid horizontal scrolling. Keep the diagram explicitly illustrative and readable without animation. Any later motion must have a still reduced-motion state.

Installation should reveal or navigate to the exact command block. Copy controls need success and failure feedback; links must retain visible keyboard focus. Evidence distinctions must be textual, never color alone. Reserve space for long library names and multi-line property descriptions. No invented studies, customer logo wall, adoption counters, testimonials, or pricing.

## Deliverables and open decision
Image-generation mockups and this brief only. Mockup typography and wording are proposals; the brief and linked PR govern factual claims. No stack selected, no production assets extracted, no DESIGN.md or implementation contract written, and no website built or deployed. Select a direction or request a revision; the selected image and brief can then be marked approved for a later build.
