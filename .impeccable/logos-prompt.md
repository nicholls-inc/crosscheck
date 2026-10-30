Design five distinct logo alternatives for Crosscheck. Write files only inside `.impeccable/logos/`. Do not edit anything else in the repository.

## What Crosscheck is

Read `docs/VISION.md` and `PRODUCT.md` first. In short: Crosscheck is a suite of tools for verifying AI-generated code. Its rule is that no guarantee rests on an LLM's judgment. Only deterministic checks (Lean and Dafny proofs, differential tests, a contract-graph checker) and recorded human decisions count as evidence. It makes claims inspectable: each result states its strength (proved, tested, decided by a person, assumed, not yet reached), its assumptions and its limits. The name comes from checking one thing against an independent second thing. The first public use is a study in which a Lean model was compared with EVerest's charging-schedule code, finding a restart one second early.

## Where the logo lives

It sits at the top left of a reading site called "EVerest OCPP lib: a formal study", above that title, at roughly 20 to 28px tall. It also appears in the footer next to "The tool behind this study: Crosscheck". The site's visual world is called "The Evidence Record":
- warm paper ground `#f8f7f4`, ink `#18191b`, rules in `#7d808a` / `#b6b8be`
- one accent, highlighter yellow `#f8eab4`, used sparingly
- link blue `#2a5a9e`
- type: Newsreader (a text serif) for reading, Public Sans for labels, IBM Plex Mono for commands
- flat and ruled: hairline rules, no shadows, no gradients, square or barely rounded corners
The logo must belong in that world: quiet, precise, typographic, like a mark on a well-set research record. It must not look like a startup app icon.

## Deliverables

1. Five SVG marks: `.impeccable/logos/mark-1.svg` to `mark-5.svg`. Each is a self-contained SVG using paths and shapes only (no `<text>`, no external fonts, no raster images). Use `currentColor` for ink so the mark can recolour, and literal `#f8eab4` if a concept uses the highlighter. Use a tight `viewBox` and make it legible at 20px tall.
2. Each concept must come from a different idea. Do not make five variations of one shape. Candidate ideas to choose from or beat: two independent strokes that must agree; a check mark ruled against a reference line; a ledger tick; a proof's end mark (∎ / QED) recast; a crossing that forms a check; a diff or comparison of two readings. Avoid the obvious clichés (shield, padlock, magnifying glass, a generic tick in a circle, brackets around a tick).
3. `.impeccable/logos/index.html`: a comparison sheet showing each mark at 24px, 64px and 160px on the paper ground, in a lockup with the word "Crosscheck" set in a serif (Georgia fallback is fine), and once inverted (paper on ink). Under each, add one sentence on the idea and one on its risk. Inline the SVGs so the page renders from disk.
4. `.impeccable/logos/NOTES.md`: the five ideas, one paragraph each, and which one you recommend and why.

Keep the geometry clean: whole-pixel or half-pixel coordinates, consistent stroke weight within a mark, and optical balance at small sizes.
