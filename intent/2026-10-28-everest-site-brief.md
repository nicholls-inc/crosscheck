# Brief: the EVerest case-study site

Status: confirmed by the maintainer on 2026-10-28, through `/impeccable shape`.
It replaces the landing-page purpose in `2026-10-28-evidence-led-site*.md`.

## Problem

The current home page asks too much reading before a visitor knows what the
site is about. The maintainer found it confusing, so a first-time reader will
too. The site moves from promoting Crosscheck to presenting one thing: the
EVerest case study.

## 1. Job and audience

- Curious engineers who arrive from the "Review Evidence, Not Code" essay or a
  shared link. Their question: what was found, is it real, and how sure are we?
- Mode: Read. The visitor comes to understand something. The install call to
  action leaves the page body.

## 2. Outcome and proof

- Within ten seconds a reader can say: "A charger restarts a daily schedule one
  second early, because of leap seconds. It was found by comparing EVerest with
  a proved model."
- Success: a reader picks one question and gets a complete, short answer
  without scrolling past 13 sections.
- Source of truth is the content of `site/everest.html` and
  `site/everest-study.json`. Words and arrangement may change freely. Facts may
  not: every number, claim, qualification, limit and attribution stays as it is.

## 3. Structure (adapted from balfrontower.org)

The page becomes `site/index.html`. The current `index.html` landing page is
retired, and `everest.html` redirects to the new index or is removed.

Top: five expandable sections. "About this page" is open by default. Each opens
to two to four short paragraphs.

| Section | Holds |
| --- | --- |
| About this page | The one-line finding, how to use the questions, the essay link, one line on who did the work (Harry Nicholls) |
| The charger | What EVerest and OCPP 1.6 are, and what a charging schedule is |
| The finding | 11:00:00 against 10:59:59, the witness readings, the leap-second cause, low severity |
| The study | Model, proofs, the 2,000-case comparison, and who decided what |
| Repeat it | The pinned commit and the replay commands |

Middle: questions that filter the evidence. Selecting one shows only the
blocks that answer it.

1. What went wrong?
2. Does a one-second error matter?
3. Is this a proof that EVerest is wrong?
4. How was EVerest tested, and what came back?
5. What did a person decide, and what did the AI do?
6. What does the result depend on?
7. What wasn't checked?
8. Can I run it myself?

A second filter, in place of Balfron's document types, sorts by evidence
strength: Proved, Tested, Decided by a person, Assumed, Not yet reached.

Below: the evidence, split into small tagged blocks. Tables stay whole. With
no filter, every block shows in story order.

Footer: "The tool behind this study: Crosscheck", linking to
https://github.com/nicholls-inc/crosscheck, plus the publication record and
the glossary.

## 4. Scope and boundaries

- One page. The visual world is kept: Newsreader, Public Sans, IBM Plex Mono,
  warm paper, ruled tables and specialist-term help.
- Filters never hide a qualification. A block that carries a limit shows that
  limit inside the block.
- "About this work" shrinks to one line in the "About this page" section.

## 5. States

- No filter: every block, in story order.
- A question selected: its blocks only, under a heading that repeats the
  question, with a "show everything" link.
- A question and a strength filter that match nothing: say so, and offer to
  clear the filter.
- Each question has its own URL fragment, so one answer can be shared.

## 6. Interaction and layout

- Sections and filters are real controls that work with the keyboard and with
  touch. On a phone the five sections stack as an accordion.
- Without JavaScript every block shows, and each question works as a jump
  link. Filtering is an enhancement.
- Prose keeps a 45 to 60 character measure. Tables can use the full width.

## 7. Open items for the build

- `PRODUCT.md` still describes an install-led landing page. Update it to this
  purpose at build time.
- The site's build and check scripts (`scripts/build-site.mjs`,
  `scripts/check-site.mjs`) assume the current two pages; they need updating
  with the move to `index.html`.
- The question wording is a draft that the maintainer accepted for now.
