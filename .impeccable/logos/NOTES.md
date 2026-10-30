# Crosscheck logo alternatives

These are five standalone geometry-only SVGs for the Evidence Record. Their ink uses `currentColor`; no font, bitmap, script or external reference is required. The comparison sheet embeds the same geometry and sets the Crosscheck lockup in Georgia, with system sans labels. Each concept includes 24px, 64px and 160px specimens, a paper-on-ink lockup, and an expandable study placement at 20px. The forms intentionally remain monochrome so yellow stays available to highlight evidence on the study itself.

## 1. Independent agreement

Two separate checking strokes repeat the same geometry, making agreement between independent readings the identity. The paired ticks can suggest read receipts, and the independence needs the name to explain it. The strokes stay separate, with matching proportions and a shared rising direction; neither stands in for a second LLM’s approval. The identity is about the act of comparison, not the strength of a particular claim.

## 2. The reference rule

A check sits against a ruled baseline: a result made relative to an explicit reference. The familiar check is easy to read, but could be mistaken for a blanket approval badge. The low horizontal rule gives the gesture a reference and a stable typographic footing. Its open silhouette and few elements survive at masthead size, while the square stroke ends sit comfortably beside serif type.

## 3. Ledger entry

A compact ledger keeps a small checking stroke beside the record it belongs to. The row detail softens at the smallest size and may suggest a checklist application. Three rules evoke a record, a vertical separator establishes a checking margin, and the short tick belongs to one row. At 20px the margin and rows remain distinct, but the checking stroke is deliberately subordinate.

## 4. Divided QED

Two solid triangles rebuild a proof-ending square, with a rising seam keeping the two halves visibly independent. The proof reference favours formal methods and may overstate what tested or human-decided evidence establishes. The two triangles together recall a QED square; a rising diagonal channel holds their edges apart. This is the most compact and typographic option, though the diagonal split could suggest a general division symbol. It should be a brand mark, never a status label applied to every result.

## 5. Compared readings

Two stepped readings span the same interval but differ at one transition, exposing a discrepancy for inspection. Without context, the stepped lines could read as a waveform or an electronics symbol. The two square-ended traces rise at different horizontal positions, keeping the disagreement visible. This connects most directly to the study’s early restart while remaining a general comparison symbol; it is a schematic relationship, not a plot of the study data.

## Recommendation

Choose **mark 2, The reference rule**. It has the strongest immediate reading at 20–28px, sits naturally above a serif study title, and brings the site’s horizontal ruling into the identity without packing a table into a tiny symbol. The baseline makes the idea of checking relative to a reference concrete. Keep it monochrome and give it roughly half its height in clear space beside the name. Its approval-badge risk matters: use it as Crosscheck’s identity, with explicit proved, tested, human-decided, assumed and not-yet-reached labels continuing to carry the evidence strength.

Mark 5 is the stronger alternative if distinctiveness and the comparison mechanism matter more than immediate recognition. It makes room for disagreement, which is central to Crosscheck finding bugs rather than merely confirming success.

## Geometry and use

Coordinates are whole pixels; stroked marks use one consistent 2-unit stroke with square-ended default butt caps and miter joins. ViewBoxes include the half-stroke extents at their edges. The mark heights vary optically: the paired checks form a diagonal unit, the ledger runs wide, and QED stays square. Scale each by height rather than forcing all into square boxes. The filled QED square has a 3-unit diagonal offset between its two halves. All marks also work on an ink ground by setting the surrounding colour to `#f8f7f4`.

For recolouring, inline the SVG and set CSS `color` on its parent. A separately loaded SVG image does not inherit the host document’s colour. The provided SVG files contain only paths inside the SVG root; the HTML adds accessible names to the inline comparison specimens.
