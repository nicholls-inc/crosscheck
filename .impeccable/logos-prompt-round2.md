Use the $impeccable skill for this task. It is brand-mark work inside an established visual world: load the skill's guidance on new visual work and its craft floor, and follow its discipline (diverge widely before converging, refuse category defaults, verify the render). Do not run its interactive question flow: the brief below is the answer to those questions, from the project's maintainer.

# Task: five new Crosscheck logo marks (round 2)

Write files only inside `.impeccable/logos/`. Do not edit anything else.

## What the mark must say

Read `docs/VISION.md` and `PRODUCT.md` before sketching. Crosscheck is a suite of tools for verifying AI-generated code. Its convictions, in the maintainer's words: **evidence, proof, transparency, not trusting LLMs, rigor.**

- No guarantee rests on an LLM's judgment. An LLM may draft code, specs and proofs, but only deterministic checks (Lean and Dafny proofs, differential tests, a contract-graph checker) and recorded human decisions count as evidence.
- Evidence comes in grades, and Crosscheck always says which: proved, tested, decided by a person, assumed, not yet reached. It never issues a blanket "approved".
- Every result shows its working: its assumptions, its trusted base, and where it stops. Nothing is hidden behind a verdict.
- "Crosscheck" means checking one account against an independent second one. In the first public study, a Lean model and EVerest's C++ code were run on the same inputs, and they disagreed by one second.

## What happened in round 1

Open `.impeccable/logos/index.html` and `NOTES.md` to see the first five marks. The maintainer's verdict: 4 and 5 are the closest, "but they're not great". Keep `mark-4.svg` exactly as it is. Delete nothing; marks 1, 2, 3 and 5 stay on disk but leave the comparison sheet.

Why round 1 fell short:
- Three of five marks were checkmarks. A tick means "approved", which is exactly the blanket verdict Crosscheck refuses to give. **No checkmarks or ticks in any form this round.**
- Mark 4, the divided QED square, works because it is a compact, solid silhouette with a real proof reference. Its weakness is that a diagonally split square is generic, and it says "proof" but not "independent check" or "transparency".
- Mark 5, the compared readings, says "comparison and disagreement", but it reads as a waveform or a circuit symbol, and it is spindly at 24px.

The bar: each new mark must be as solid and compact as 4, as specific to Crosscheck as 5 wanted to be, and distinctive enough that nobody could mistake it for another developer tool.

## Banned

Checkmarks and ticks; shields; padlocks and keys; magnifying glasses; eyes; brains, sparkles, stars or anything "AI"; circuit traces and waveforms; code brackets `</>` and `{}`; the letter C in a circle; gradients, outlines-of-filled-shapes as a style trick, and anything that only works in colour.

## Territories to mine (sources, not shapes; invent the form yourself)

- The notation of proof: the turnstile ⊢ ("proves") and double turnstile ⊨ ("models"), the end-of-proof square ∎, inference bars.
- Audit and record-keeping: a ledger rule, a witness's countersignature, a notary's perforation, a tally, a double-entry balance.
- Independent witnesses: two accounts that must agree, a second reading taken from a different instrument.
- Measurement and calibration: a plumb line, a set square, a reference mark, a tolerance.
- Showing the working: a form drawn open so its inside is visible, a boundary that is drawn rather than hidden, a trusted base marked explicitly.
- Refusal of blind trust: something that does not accept a claim until a second thing meets it.

## Method

1. Diverge: write at least 15 concepts, one line each, drawn from at least four different territories above (or better ones you find). Put them in `NOTES-round2.md`.
2. Score every concept 1 to 5 against each of the five convictions, and against distinctiveness and legibility at 24px. Keep the scores in the notes.
3. Build the best five that are also the most different from each other and from mark 4. No two may share a construction idea.
4. Craft each one: a self-contained SVG using paths and shapes only (no `<text>`, no fonts, no rasters), `fill`/`stroke` as `currentColor`, a tight `viewBox`, whole- or half-unit coordinates, one consistent stroke weight per mark or solid fills, and optical balance at 24px. A mark should be recognisable in silhouette, filled solid.
5. Verify: render each mark at 24px and 160px, for example with `magick` (ImageMagick is installed) or headless Chrome, and look at the result. Redraw any mark that blurs, fills in, or loses its idea at 24px. Record in the notes what you changed after looking.

## Deliverables

- `.impeccable/logos/mark-6.svg` to `mark-10.svg`.
- Update `.impeccable/logos/index.html` so it shows mark 4 and marks 6 to 10 only, each at 24px, 64px and 160px on paper `#f8f7f4`, in a lockup with the word "Crosscheck" set in a serif (Georgia is fine), once inverted (paper on ink `#18191b`), and once at 22px above the line "EVerest OCPP lib: a formal study" as it will sit on the site. Under each, add one sentence on the idea, one naming which of the five convictions it carries, and one on its risk.
- `.impeccable/logos/NOTES-round2.md`: the concept list with scores, the five chosen with reasons, what the 24px check changed, and your recommendation with the runner-up.

The world it lives in (unchanged): warm paper `#f8f7f4`, ink `#18191b`, rules `#7d808a`, a single highlighter yellow `#f8eab4` used sparingly, link blue `#2a5a9e`; Newsreader serif for reading, Public Sans for labels; flat and ruled, with hairline rules, no shadows and no gradients. The mark should look like it was set by a careful typographer for a research record, not drawn for an app store.
