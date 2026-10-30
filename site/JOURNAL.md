# Website journal

## 2026-10-30 — The glossary reads like the study page

**Type:** fix
**Touches:** site/glossary.html, site/styles.css, DESIGN.md
**Why:** The glossary used the same tokens but a different arrangement: a large serif
heading, a two-column grid of small terms and a rule under every term. It read like
another site.

The glossary is now set as the study page's evidence list: a small "Glossary" label
and hint, then two groups ("EVerest and OCPP", "How the study checks") with their
titles in the label column, and each term as an open entry in the entry-title type with
its definition at reading measure. Definition text, ids and deep links are unchanged, so
check-site's match with the study page's tooltips still holds. The `.band` and `.terms`
rules, used only by the old glossary, are deleted. Two sentences in the longer accounts
lost a colon and an "admits".

A layout and type pass followed. "Glossary" had become the smallest heading on its own
page, so it moved into the question-heading role (28px) on the content column's edge.
The nine-term "How the study checks" group split into the study page's own groups
(Proof, Tests, Assumptions, People). The longer accounts dropped to 16px so the
definition leads, and a link into the glossary now highlights the term it lands on.
The Impeccable type and layout scans are clean.

A critique of the glossary (22/32) followed. Its longer accounts were the least plain
prose on the site and nothing on the study page led to them. They are now plain and one
idea per paragraph, with "profile", "warning" and the smallest example explained. Three
evidence entries on the study page link to the account for their term. The glossary has
a jump list of all thirteen terms, each term links to itself, each line is measured in
its own size, and jump targets land 24px down instead of 48px. "Reference program"
became "reference model", the term Lean-based differential testing uses (Cedar's Lean
model, Resonate's reference model), with "the model" kept as its short form. The page
metadata was left to another agent. The glossary also gained a plain "Back to the study"
link in its heading row.

On phones, Harry noticed two inconsistencies. The Finding first line wrapped while the
other three were cut short with an ellipsis, and evidence text sat 42px from the left
but 24px from the right, because entries and sections indented 18px to make room for
their plus mark. Now every first line wraps to its full sentence, and the plus marks
hang in the page margin, so every text block on a 390px phone measures 24px on both
sides. Measured with .impeccable/measure-phone.mjs; 36 browser checks pass. Harry then
asked for each label to share its sentence's first line, so on phones the label now runs
into the sentence like a run-in heading ("+ About: A study of EVerest…") and the rest
wraps beneath it.

The study title broke differently on the two pages. The study page sets it in an h1,
which takes `text-wrap: balance`; the glossary sets it in a p, which took the paragraphs'
`pretty` wrap. The shared title rule now sets `balance` itself. A comparison of every
shared element (lockup, title, footer, copyright) across both pages at 1440, 700 and
390px now finds no difference, and every glossary block has the same 24px margin on
both sides at 390px as the study page.

## 2026-10-30 — The first screen explains itself and the filters can be shared

**Type:** fix
**Touches:** site/index.html, site/help.js, site/styles.css, site/everest-study.json,
scripts/check-site.mjs, scripts/site.test.mjs, DESIGN.md,
intent/2026-10-28-everest-site-brief.md
**Why:** A design critique (28/40) found the first screen gave 10:59:59 without its
cause, the only usage line sat in the closed About section, strength filters could
dead-end and were lost on refresh, and the filed issue sat at the end of the list.

The Finding line now ends "a leap second early", and a hint under the Filter label
says how to use the questions. About stays closed; the brief, which said it was open,
now records that choice. Both filters live in the fragment (`#q-proof&s=tested`), so
Back, refresh and shared links keep them, and an unknown fragment is reported.
Each strength shows how much evidence it holds for the chosen question and dims at
zero. The upstream issue moved into the Finding section, entries gained the
sections' plus, toggles are 44px, the title is no longer a self-link, and a phone
reader can return to the questions without losing the filter. The ledger's
"Unfinished" became "Not yet reached", and check-site now rejects any claim
strength outside the five names. Study codes (A1, D4, Q4, I4, N4) follow plain
words instead of leading. Two strength icons read as controls, so they were
redrawn: Tested is a flask, not two lines that read as an equals sign or a menu, and
Not yet reached is the proved square left open at a corner, not three dots that read
as "more". The open square also stays distinct from the dashed Assumed circle, which
is why the first drawing, a three-quarter circle, was dropped. The filter row was then
distilled: "Questions" moved into the label column with a shorter hint under it, the
row-wide "Filter" label and the three question-group titles went (space alone keeps
the groups), and strength counts appear only once a question is chosen. The row is
about 80px shorter and its first line now reads label, question, strength label.
A prose pass then replaced colons used as connectors with full stops, named the
actor in passive sentences ("the study compares", "the harness compares"), split the
"What does the result depend on?" answer and the report caveat into short
sentences, and replaced three aphoristic lines with plain statements. No facts or
numbers changed. A second pass, at Harry's request, rewrote the remaining convoluted
sentences in plain subject-verb order, split each answer into one paragraph per idea,
explained the 55 vacuous passes where they appear, and named Harry for every human
decision ("Decided by Harry"), with the full name kept for the byline and footer.
Harry's first-person account is now set as a quote, apart from the third-person page.

A second critique (29/40) led to another pass. Answers now live once, under their
question in the evidence list, so readers without JavaScript see them; check-site
fails if an inline question drifts from its link. The went-wrong and matter answers
name the filed issue. The title became "EVerest and OCPP 1.6: a formal study"
(check-public-site follows it). The two numeric ledgers stay tables on phones,
standalone links have 44px hit areas, entry titles are headings, strength buttons
read "Proved, 2 pieces", the witness tooltip opens in the margin, the quote mark
hangs at size, and a font kerning gap in "myself?" is switched off for that pair.
"The reading Harry approved" replaces "the reviewed reading"; the severity rating
stays the study's. Generated prose uses curly punctuation. A bad filter link is
reported under the Questions hint. Declined: question-group titles (removed by
Harry's distill pass), a second strength link on phones, and cutting About to four
paragraphs (the brief now follows the one-idea rule). Verified with 36 headless
Chrome checks at 1440px and 390px and 13 site and launch tests.
Opening only one entry per question was considered and
deferred by the owner. Verified in headless Chrome at 1440px and 390px (20
behaviour checks) plus the site tests and the Impeccable detector.

## 2026-10-29 — The study links the filed EVerest report

**Type:** fix
**Touches:** site/everest-study.json, site/index.html, scripts/build-site.mjs,
scripts/site.test.mjs, docs/publication/crosscheck-story.md
**Why:** EVerest issue #3323 was filed, but the publication still called it a
draft and said no upstream filing existed.
**Links:** [EVerest issue #3323](https://github.com/EVerest/everest/issues/3323)

The publication record now states that the report is open and links to it. The
site and application-story draft do not imply the report is accepted or fixed;
the diagnostic input change remains distinct from a production repair. The
issue had no comments when checked. Site generation, publication validation and
all 10 website tests pass.

## 2026-10-28 — The site becomes the EVerest case study

**Type:** feature
**Touches:** PRODUCT.md, site/, scripts/build-site.mjs, scripts/check-site.mjs,
scripts/site.test.mjs, scripts/check-public-site.mjs,
scripts/check-public-site.test.mjs
**Why:** The maintainer found the landing page too much text to grasp, even as
its author. They chose to drop the product pitch and present only the EVerest
study, using balfrontower.org's expandable index tabs and question filters.
**Links:** [brief](../intent/2026-10-28-everest-site-brief.md)

`index.html` is now the study. Two clock readings open it, five index tabs give
context, and eight questions filter the evidence blocks, each tagged with its
strength (proved, tested, decided by a person, assumed, unfinished or not yet
reached). A strength filter narrows further. Without JavaScript every block
shows and each question is a jump link. `everest.html` forwards old links,
fragment included. The install call to action is gone; the footer reads "The
tool behind this study: Crosscheck".

Copy was rewritten and regrouped, but every number still renders from
`everest-study.json`, and the hand-written short answers restate facts from the
previous page without new claims.

## 2026-10-28 — The study explains the author's engineering contribution

**Type:** feature
**Touches:** PRODUCT.md, site/, scripts/build-site.mjs, scripts/site.test.mjs,
scripts/check-public-site.mjs, scripts/check-public-site.test.mjs, deploy/,
docs/publication/crosscheck-story.md
**Why:** The user selected evidence approach v3 and clarified that the site,
essay and case study should demonstrate their work for applications and
specific side-door conversations. Hosting is the user's home server behind an
existing Cloudflare Tunnel.
**Links:** [intent](../intent/2026-10-28-career-evidence.md),
[spec](../intent/2026-10-28-career-evidence-spec.md),
[plan](../intent/2026-10-28-career-evidence-plan.md)

The homepage identifies Harry Nicholls and links to his contribution. The
study's shared record now separates human direction/interpretation decisions,
AI drafting and deterministic checks. Decision/boundary pairs explain the use
of a Lean model, transition-instant tests and bounded reporting. Scientific
counts and source pins are unchanged.

Publication docs contain unsent application/outreach drafts. Exact job filters
stay in local preparation notes outside the committed site. A Caddy origin and
optional systemd template prepare the confirmed hosting path; a smoke checker
detects an old worktree, mismatched record, missing replay sections or stale
assets. Public routing and external sending remain user-operated actions.

Local checks exercised Caddy on loopback with test state isolated in .canvas/.
The shipped config disables autosave persistence so a separate origin does not
overwrite another Caddy autosave. The initial local test used Caddy's default
app-data autosave location before that setting was added; this was disclosed
to the user and did not involve the home server or tunnel.

## 2026-10-28 — The EVerest evidence connects the essay to the tools

**Type:** feature
**Touches:** site/, scripts/build-site.mjs, scripts/check-site.mjs
**Why:** The user chose the evidence-led Canvas approach, building on the
published vision post, PR #144 and the existing landing-page implementation.
**Links:** [intent](../intent/2026-10-28-evidence-led-site.md),
[spec](../intent/2026-10-28-evidence-led-site-spec.md),
[plan](../intent/2026-10-28-evidence-led-site-plan.md)

The home page leads to a local EVerest study. One editorial JSON record renders
the shared claims and witness, reported results, dependencies, limits and replay
instructions into checked-in HTML. It pins study revision `7d05c4e`, replacing
the older landing-page snapshot, and labels its results as reported. The source
report contains both an updated warning rule and a legacy note describing
extra-limit instants as unclassified; the publication preserves that caveat.
It does not reinterpret the historical counts or claim a new scientific run.

The original landing-page worktree remains untouched. Its design, glossary and
install workflow are retained. External publication, deployment and the draft
upstream issue remain separate actions.
