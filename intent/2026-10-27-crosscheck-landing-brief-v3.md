# Crosscheck landing page — third exploration

Status: the user selected the evidence-record direction and requested a denser revision: wider main content, shorter header and much smaller type overall. The compact revision is pending review. Both prior rounds are rejected.

- **Job and action.** Help developers understand Crosscheck, examine evidence from a real library investigation and install the Claude Code plugin. Primary CTA: Install the plugin. Scope remains image mockups and a design brief only.
- **Authority.** docs/VISION.md, the user-refined 14-reference [Are.na channel](https://www.are.na/harry-nicholls/crosscheck), and [Brutalist Web Design](https://brutalist-web.design/), read in full. The channel now retains warmth, useful visual artifacts, mixed typography and deliberate hierarchy; it removed the starkest personal-document examples that drove round two. Treat content, browser affordances and reading as the material. Decoration must help explain, navigate or read the content. Do not return to either earlier round.
- **New structural idea.** Put a real piece of verification evidence into the page's composition. The EVerest witness supplies a specific finding, with expected and actual restart times; the report supplies a reproducible comparison table. Give these artifacts a natural relationship to the explanatory copy, rather than making them simulated product UI. Use a light mineral or warm off-white surface, dark ink, a readable text serif with a proportional sans for navigation and evidence labels, moderate headline scale, fine rules and clearly underlined links. These choices are proposed from retained references, not confirmed brand rules.
- **Narrative and honesty.** Explain the plugin's Dafny proof/extraction and Lean model/testing paths, show install, then demonstrate the EVerest OCPP 1.6 investigation with direct evidence links. Three model invariants are proved; EVerest is compared by tests. The report's minimized witness has the expected restart at 11:00:00 and an early restart at 10:59:59. The new model is not a proof of the entire library. Use the linked PR's spec qualifications, correspondence review, generator bounds and trusted-base limits in the supporting case study.
- **Usability and later implementation.** All meaningful controls are links or buttons with visible interaction states. Preserve normal browser navigation and scrolling. Text remains selectable; evidence tables retain labels on mobile. Layout stacks in source reading order, with installation visible near the introduction. If later implemented, honor reduced motion and provide feedback for copying commands. No web stack choice, code, deployment, or design-system contract in this phase.

## Source for the visual evidence
[PR #144](https://github.com/nicholls-inc/crosscheck/pull/144), pinned head `d41f08251ceeb70f53bc98d850fe56d70fc190c0`; `formal-verification/tests/ocpp16_composite/drt_report.md`, reread for this round.

The minimized daily-profile witness compares the values at 2026-08-13T10:59:59Z: model 6543.6 W, EVerest 9249.9 W. The model's daily schedule restarts at 11:00:00, while EVerest restarts a second early. EVerest revision `dd696165`, Lean v4.28.0. A displayed witness must be identified as this specific example, not as a generic amount of drift for all schedules.

Report summary, RNG seed 20261024, 2,000 generated cases per configuration:

| Recurring anchors | Pass | Warn | Fail |
| --- | ---: | ---: | ---: |
| 2017–2025 | 2,000 | 0 | 0 |
| 2010–2016 | 860 | 67 | 1,073 |
| Same older anchors, leap-corrected | 2,000 | 0 | 0 |

The correction is a diagnostic rerun, not an upstream fix. Do not aggregate these into a success percentage. The upstream issue was a draft in the pinned source. Keep the actual install commands from README.md available in the planned installation section.

## References carrying this exploration
Reed Rosenbluth: warm neutral ground and open, asymmetric composition. Hamilton Ulmer: mixed text voices and a figure that explains the author's argument. Dandelion: recognizable hyperlinks, useful ruled tables and modest browser controls. On Being: a distinct typographic hierarchy and comfortable reading. Physical Intelligence: a concise explanation that leads directly into inspectable research. Phenomenal World: linked editorial material with a distinct identity. These are sources for decisions, not page templates to reskin.

## Mockups for review
- `.impeccable/mocks/round-3/witness.png`: an asymmetric introduction beside a specific observed/expected restart witness, followed by the case-study results. Serif hierarchy and ruled tables carry the composition.
- `.impeccable/mocks/round-3/trace.png`: a stronger sans/serif hierarchy with a plot of the minimized witness: 6543.6 W until the recurrence boundary, then 9249.9 W; EVerest changes at 10:59:59, the model at 11:00:00. The highlighted interval explains the discrepancy.
- `.impeccable/mocks/round-3/record.png`: a centered explanatory opening leads into an asymmetrical case study and an evidence table, with installation commands below. The table separates proved model properties from sampled implementation results.

The built-in image tool generated these comps from retained Are.na reference screenshots. The exact prompt and an unapproved-status sidecar sit beside each image. No website implementation or new production design system was created. The images are presented directly in the conversation; no local direction-board server is needed.

## Selected direction and requested revision
The user chose `record.png`. Preserve its composition, palette, serif/sans pairing, factual copy, evidence table and installation action. Refine density only: considerably smaller left/right margins, a short single-line masthead, a shorter introductory block and much smaller type throughout. The compact mockup is `.impeccable/mocks/round-3/record-compact.png`; the earlier record remains the reference for its identity.

Proposed desktop geometry at 1536px wide: about 48px outer margins, 48px navigation masthead, 32px main heading, 15px prose, 14px evidence rows, and 12–13px navigation/captions/code. Introductory content ends around y=330 rather than occupying half the viewport. The case-study/table region uses the wider canvas, followed by compact installation. These are revision targets for the image, not an implementation contract or a claim of pixel-measured output. Mobile should keep legible body type and stack columns rather than scale down the desktop screenshot.

## Plain English and inspectable explanations
User requirement: the site itself must make claims and evidence transparent and inspectable. Use plain English, with minimal jargon. Ubiquitous software-engineering and technology terms need no definition or help styling; "AI" and "plugin" are explicit user examples. Familiar terms such as code, tests, test cases, inputs, outputs, installation and marketplace remain ordinary text. Reserve dotted underlines, a question-mark cursor (`cursor: help`) and plain-English hover explanations for specialist verification and charging terms, such as invariants, mathematical proofs, Dafny, Lean, OCPP and leap seconds. Apply the treatment consistently when those specialist terms occur in prose, headings, tables or controls.

Replace avoidable jargon directly: "invariants" becomes "rules"; "anchors" becomes "schedules starting in"; "no divergence" becomes "all 2,000 matched"; "flagged" becomes "cases differed"; "generated cases" becomes "test cases"; "RNG seed" becomes a plain instruction for repeating the run. Prefer link labels such as "Written rules", "Review of the reference program" and "Test results". Retain the exact OCPP 1.6 scope in the case-study details and explain its acronym if displayed.

Do not weaken claims while simplifying them. Mathematical proofs concern the reference program under stated assumptions; tests compare the implementation on the inputs tried. Show the sample count and scope in the main content. Keep direct source links, the pinned software version, test starting number, and instructions for repeating checks available in the case study. The site must distinguish the vision from what the tools do today.

### Help behavior for a later implementation
- A dotted underline identifies an explanation, while a solid underline identifies navigation. Pure help terms are not links. If a technical term is also a link or part of a button, preserve that control's navigation/action and provide the same explanation on hover and keyboard focus; do not nest a second interactive control inside it.
- Open the explanation on hover or keyboard focus; expose the relationship to assistive technology with a descriptive association such as `aria-describedby`. Use real tooltip content, not only a browser `title` attribute. A small help/question-mark cursor appears on pointer hover.
- Keep the tooltip open while the pointer moves onto it, and keep it visible until the user leaves both the term and explanation, moves focus, or dismisses it. Escape dismisses it. Explanations must remain readable at zoom and fit inside the viewport.
- On touch, tapping a help term opens its explanation; tapping outside or dismissing closes it. Keyboard users must be able to reach every help term. Explanation text should contain no unexplained jargon or required nested tooltips. Put further source links in the page or an adjacent details section rather than inside a transient tooltip.
- Use one glossary source for consistent definitions. Repeated terms share definitions. Help text explains the term; important limits, assumptions and uncertainty stay visible in the page rather than appearing only on hover.
- Before a later site is considered complete, check the whole surface for unexplained specialist terms and unnecessary help styling on familiar terms. Check hover, help cursor, keyboard focus, Escape, movement onto the tooltip, touch, viewport edges and zoom. Static image mockups show the intended appearance, not working interaction. The latest image, `.impeccable/mocks/round-3/record-plain-english-v2.png`, removes help styling from familiar terms including "AI", "plugin" and "test cases", while retaining specialist help and the narrower reading columns.

### Reading width
The user found the hero paragraphs too wide. Keep two centered introduction columns, each approximately 390px at a 1536px desktop viewport, with an approximately 80px gutter. Aim for 45–60 characters per line. Header, case study, evidence table and installation retain the wider page. On narrow screens, stack the prose columns and keep a readable font size. The current revised image is `.impeccable/mocks/round-3/record-plain-english-v2.png`, showing the intended help appearance, familiar-term exemptions and narrower introduction.
