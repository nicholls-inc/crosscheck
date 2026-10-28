---
name: Crosscheck
description: Check the code. Read the evidence.
colors:
  ground: "#f8f7f4"
  ink: "#18191b"
  text: "#25282e"
  muted: "#4f535b"
  code-ink: "#33363d"
  rule-header: "#5d5f68"
  rule-strong: "#7d808a"
  nav-underline: "#8d8f98"
  tip-border: "#9a9da2"
  rule: "#b6b8be"
  rule-faint: "#d9dadd"
  link: "#2a5a9e"
  link-hover: "#153c74"
  mark: "#f8eab4"
  button: "#232322"
  button-hover: "#000000"
  button-label: "#ffffff"
  help: "#6d717a"
typography:
  display:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "40px"
    fontWeight: 400
    lineHeight: "46px"
    letterSpacing: "-0.02em"
    fontVariation: "\"opsz\" 60"
  clock:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "clamp(56px, 7.2vw, 112px)"
    fontWeight: 380
    lineHeight: 1
    letterSpacing: "-0.035em"
    fontFeature: "\"lnum\", \"tnum\""
    fontVariation: "\"opsz\" 72"
  wordmark:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "28px"
    fontWeight: 400
    lineHeight: "34px"
    letterSpacing: "-0.012em"
  reading:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "46px"
    fontWeight: 380
    lineHeight: "52px"
    letterSpacing: "-0.02em"
    fontFeature: "\"lnum\", \"tnum\""
    fontVariation: "\"opsz\" 60"
  headline:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "32px"
    fontWeight: 400
    lineHeight: "38px"
    letterSpacing: "-0.022em"
    fontVariation: "\"opsz\" 48"
  title:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "24px"
    fontWeight: 400
    lineHeight: "31px"
    letterSpacing: "-0.012em"
    fontVariation: "\"opsz\" 36"
  tab:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "21px"
    fontWeight: 400
    lineHeight: "28px"
    letterSpacing: "-0.01em"
    fontVariation: "\"opsz\" 36"
  answer:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "21px"
    fontWeight: 380
    lineHeight: "30px"
    fontVariation: "\"opsz\" 24"
  question:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "20px"
    fontWeight: 380
    lineHeight: "27px"
  body:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "19px"
    fontWeight: 380
    lineHeight: "26px"
  body-note:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "17.5px"
    fontWeight: 380
    lineHeight: "25px"
  table:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "17px"
    fontWeight: 380
    lineHeight: "24px"
    fontFeature: "\"lnum\", \"tnum\""
  tip:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "15.5px"
    fontWeight: 400
    lineHeight: "22px"
  label:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "13.5px"
    fontWeight: 500
    lineHeight: "22px"
    letterSpacing: "0.01em"
  control:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "22px"
    letterSpacing: "0.01em"
  nav:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
    letterSpacing: "0.01em"
  mono:
    fontFamily: "IBM Plex Mono, ui-monospace, Menlo, monospace"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: "22px"
rounded:
  hairline: "1px"
  tip: "2px"
  button: "3px"
spacing:
  gutter: "clamp(16px, 3.125vw, 48px)"
  aside: "clamp(300px, 26.7vw, 410px)"
  col-gap: "48px"
  tab-gap: "44px"
  band-pad: "28px"
  section-pad: "44px"
  cell: "8px 10px 9px 12px"
components:
  link:
    textColor: "{colors.link}"
  link-hover:
    textColor: "{colors.link-hover}"
  nav-link:
    textColor: "{colors.ink}"
    typography: "{typography.nav}"
  help-term:
    textColor: "{colors.text}"
  tip:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    typography: "{typography.tip}"
    rounded: "{rounded.tip}"
    padding: "11px 13px 12px"
    width: "min(clamp(228px, 17.8vw, 290px), calc(100vw - 32px))"
  clock:
    textColor: "{colors.ink}"
    typography: "{typography.clock}"
  highlight-stroke:
    backgroundColor: "{colors.mark}"
    textColor: "{colors.ink}"
  index-tab:
    textColor: "{colors.muted}"
    typography: "{typography.tab}"
    padding: "16px 0 14px"
  index-tab-open:
    textColor: "{colors.ink}"
  question-link:
    textColor: "{colors.link}"
    typography: "{typography.question}"
    padding: "11px 0 12px"
  question-link-current:
    textColor: "{colors.ink}"
  strength-toggle:
    textColor: "{colors.link}"
    typography: "{typography.control}"
    height: "44px"
  strength-toggle-pressed:
    textColor: "{colors.ink}"
  answer-text:
    textColor: "{colors.ink}"
    typography: "{typography.answer}"
  strength-tag:
    textColor: "{colors.ink}"
    typography: "{typography.label}"
  ledger-cell:
    textColor: "{colors.text}"
    typography: "{typography.table}"
    padding: "{spacing.cell}"
    height: "42px"
  ledger-header:
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    padding: "{spacing.cell}"
  copy-button:
    textColor: "{colors.link}"
    typography: "{typography.label}"
    height: "44px"
  code-block:
    textColor: "{colors.code-ink}"
    typography: "{typography.mono}"
---

# Design System: Crosscheck

## Overview

**Creative North Star: "The Evidence Record"**

The page reads like a well-set research record: a warm paper ground, dark ink, a text serif doing nearly all the talking, and fine horizontal rules that divide the page into bands. Every visual device either helps someone read a claim or helps them inspect the evidence behind it. A wide ledger table carries the evidence; narrow columns carry the prose. There are no cards, no shadows, no gradients for atmosphere and no imagery that is not itself evidence.

Hierarchy comes from serif size, optical sizing and position, not from colour or weight. Weight stays near regular (380 for reading, 400 for headings, 500 only for table headers, small sans labels and a pressed filter). The one warm accent, a pale highlighter yellow, marks the single result a reader most needs to see. Links look like links: blue and underlined. Specialist terms carry a dotted underline that opens a plain-English explanation; the two underline styles never trade meanings.

Density is moderate: generous enough to read, tight enough that the finding sentence, both clock readings and the index-tab strip share the first desktop viewport. On narrow screens everything stacks in source order and tables become labelled rows rather than shrinking.

**Key Characteristics:**
- Warm off-white paper ground with near-black ink and greys tuned slightly cool.
- Newsreader for reading, headings, tab labels and data; Public Sans only for navigation, controls and labels; IBM Plex Mono only for commands and identifiers.
- Bands: a narrow summary column beside a wide evidence column, each band closed by a 1px rule.
- Ruled ledger tables with tabular numerals as the primary evidence form.
- One highlighter accent, used on a single finding per band.
- Solid underline means navigation; dotted underline means explanation.

## Colors

A near-monochrome paper-and-ink palette with one link blue and one highlighter yellow; colour is reserved for meaning.

### Primary
- **Record Blue** (link): every inline link, question links, the strength toggles, Show everything and the copy control. Underlined at 1px, offset 4px to 5px, with the underline at 70% of the link colour on prose links and 45% on question links and toggles, where many links stack. It is also the focus-ring colour.
- **Deep Record Blue** (link-hover): link and control hover, where the underline also goes to full strength.

### Secondary
- **Highlighter Yellow** (mark): the early clock reading in the hero, the witness reading that is off, the chosen question, and text selection. Nothing else.

### Neutral
- **Warm Paper** (ground): page background and tooltip background.
- **Ink** (ink): headings, the open tab label and its bar, the chosen question, the pressed strength toggle, the answer text, clock readings, strength tags, table headers, the brand, tooltip text.
- **Body Ink** (text): running prose, table cells, the footer sentence.
- **Graphite** (muted): closed tab labels, the hero note, aside notes, explanations under readings, the filter label and status, the answer count line, mobile row labels, copy status, the copyright line.
- **Code Ink** (code-ink): inline code and command blocks; slightly lighter than body ink so commands sit back from prose.
- **Header Rule** (rule-header): the rule under a ledger header row, one step darker than the strong rule so the header reads as a heading line.
- **Strong Rule** (rule-strong): band dividers, the masthead rule, the tab strip's closing rule, and the top rule of ledgers, the clock ledger, the question list, witness and glossary lists.
- **Nav Underline** (nav-underline): the underline of masthead links at rest, quieter than Record Blue because the links are ink.
- **Tip Border** (tip-border): the tooltip's 1px outline and pointer; the only edge drawn around a layer.
- **Rule** (rule): row dividers in tables, lists, the question list and between tabs; the rule above an open tab panel; scrollbar thumb.
- **Faint Rule** (rule-faint): reserved hairline tone in the token set.
- **Help Dot** (help): the dotted underline of help terms at rest.
- **Press Black** (button, button-hover, button-label): reserved for a single filled primary action. No filled button ships on the case-study or glossary pages.

### Named Rules
**The One Mark Rule.** Highlighter yellow marks the one result a reader must not miss in a given band. If two things are highlighted, neither is.

**The Ink Hierarchy Rule.** Hierarchy is carried by ink depth (ink, text, muted) and type size, never by introducing a new hue.

## Typography

**Display Font:** Newsreader (with Iowan Old Style, Georgia, serif), self-hosted variable, weights 200-800, optical sizing on.
**Body Font:** Newsreader, the same family.
**Label/Mono Font:** Public Sans (with Helvetica Neue, Arial) for navigation, controls and data labels; IBM Plex Mono 400 (with ui-monospace, Menlo) for commands and code.

**Character:** A literary text serif used with scientific restraint, with a plain civic sans for wayfinding and a quiet mono for anything the reader might type. Optical-size settings (opsz 24 to 72) tighten the headings and figures without heavier weights.

### Hierarchy
- **Display** (400, 40px/46px, -0.02em, opsz 60): the hero finding sentence in the aside, left-aligned, capped at 12ch (16ch at 860px and below). 36px/42px at 1100px and below, 32px/38px at 640px and below.
- **Clock** (380, clamp(56px, 7.2vw, 112px), line-height 1, -0.035em, opsz 72, tabular lining numerals): the two clock readings in the hero, the largest type on the site. clamp(52px, 17vw, 76px) at 640px and below.
- **Reading** (380, 46px/52px, -0.02em, opsz 60, tabular lining numerals): measured values in the witness figure. 38px/44px at 860px and below.
- **Wordmark** (400, 28px/34px, -0.012em): the masthead brand.
- **Headline** (400, 32px/38px, -0.022em, opsz 48): every band heading, the answer heading (the chosen question, capped at 16ch) and the glossary title. 28px/34px on phones.
- **Title** (400, 24px/31px, opsz 36): table captions where they show. On the study page every caption is visually hidden because the band heading already names the table.
- **Tab** (400, 21px/28px, -0.01em, opsz 36): index-tab labels on the desktop strip and on phones. 23px/29px in the stacked accordion between 641px and 860px, and when script is off and the heads read as plain headings.
- **Tab lead** (21px/28px, opsz 30, ink, 22ch): the one-sentence lead at the head of each tab panel; it shares the Tab size so lead and label read as one voice.
- **Answer** (380, 21px/30px, opsz 24, ink, 56ch): the short answer under the chosen question. 19px/27px on phones.
- **Question** (380, 20px/27px): the question links. 19px/26px on phones.
- **Body** (380, 19px/26px): prose in the wide column where no smaller role applies (limits, sources), capped at 70ch. 18px on phones.
- **Body note** (380, 17.5px/25px): the hero note (34ch, Graphite), aside notes (36ch, 60ch once stacked), tab panel text, evidence notes and scope statements (70ch), the witness caption (70ch) and the footer sentence.
- **Study prose and lists** (380, 17.5px/25px): article prose, bulleted evidence lists and contribution lists, capped at 65ch, extending the body-note role.
- **Table** (380, 17px/24px, tabular lining numerals): ledger rows, glossary entries, and the Graphite explanations under clock and witness readings (40ch; the clock explanations drop to 15.5px/22px on phones).
- **Tip** (400, 15.5px/22px): help explanations.
- **Label** (Public Sans 500, 13.5px/22px, +0.01em): every table header, the Strength column, the strength tags under band headings, clock and witness labels, contribution terms, the copy control and its status. At 400 weight in Graphite: the filter status, the answer count line and the copyright line. The filter group label is 500 in Graphite. Sentence case. Mobile row labels run at 13px in Graphite.
- **Control** (Public Sans 400, 14px/22px, +0.01em): the strength toggles and Show everything (13.5px inside the count line). A pressed toggle goes to 500.
- **Standalone link** (Public Sans 400, 15px/22px, +0.01em, 5px underline offset): tab panel links, source links and footer links.
- **Nav** (Public Sans 400, 14px/20px, +0.01em): masthead links.
- **Mono** (IBM Plex Mono 400, 13.5px/22px): command blocks; inline code at 0.82em.

### Named Rules
**The Serif Carries It Rule.** Prose, headings, tab labels, questions, table cells and numbers are all Newsreader. The sans names things the reader acts on or scans by: navigation, filter controls, standalone links, table headers, strength tags and data labels. It never sets a heading or a paragraph, and it always carries +0.01em tracking at these sizes.

**The Reading Measure Rule.** A wide page does not mean wide prose. Aside notes stay near 36ch, the answer at 56ch, and prose in the wide column at 65ch to 70ch at most; only tables, the clock ledger and the question list use the full evidence column.

## Layout

A single centred page up to 1680px wide with a fluid gutter (16px to 48px). Below the masthead the page is a stack of **bands**: each is a two-column grid with a narrow aside (300px to 410px) holding the band's heading and summary, a 48px gap, and a wide evidence column holding tables, figures and commands. Each band ends with a 1px strong rule. Evidence blocks use 28px top and 26px bottom padding; the hero uses 36px/34px, the question band 40px/34px, the answer band 36px/34px, and the glossary band 44px/48px.

The page runs in this order: masthead, hero band, index tabs, question band, answer band (shown only once a question is chosen), evidence blocks, footer.

- **Hero band.** The display sentence and a Graphite note sit in the aside; the wide column holds the two clock readings as a ruled two-column ledger (strong rule above, normal rule under each cell, 48px gap), bottom-aligned with the sentence.
- **Index tabs.** On desktop with script, the five tab labels sit on one row (each its own width, 44px apart, the first inset 4px to align with the aside), and the open panel spans the full width beneath them, behind a normal rule and 24px of top padding. The strip is closed by a strong rule. Each panel repeats the band grid: the lead in the aside width, the text in up to two CSS columns (each at least 22rem, 1000px at most).
- **Question band.** The heading and one note in the aside; the questions in the wide column as two ruled columns (strong rule above, a normal rule under each question), then the strength filter row 14px below.
- **Evidence blocks.** Every block is a band: heading, strength tags and notes in the aside; ledgers, figures, lists and prose in the wide column. Prose there is capped at 70ch.

Responsive behaviour: at 1100px the aside fixes at 300px, the column gap narrows to 36px, and the display sentence drops to 36px. At 860px every band, tab panel and the footer collapse to one column in source order; the tab strip becomes an accordion with a normal rule between heads; the question list and witness and glossary grids go to one column. At 640px the masthead wraps (nav items wrap whole, never mid-label), the clock readings stack, each tab label pushes its +/- mark to the right edge, ledger tables become labelled rows (each cell preceded by its column name in a small sans label), and Show everything moves to its own line.

Without script, every tab panel is open and every evidence block shows; each tab head reads as a plain heading and each question is a jump link to its first block.

Spacing is set per context in pixel values rather than a strict scale; the recurring steps are 4px, 8px, 10px to 18px between related blocks, 24px to 30px inside panels and band entries, 36px to 48px column gaps, and 34px to 48px band padding.

The footer repeats the band grid: the sentence "The tool behind this study: Crosscheck" in the aside, then site links, the X and GitHub links, and the copyright line pushed to the right edge. On phones they stack in that order. The glossary page (`glossary.html`) shares the masthead and footer and is a single band: a Headline-sized page title and short note in the aside, the two-column definition list beside it.

## Elevation & Depth

Completely flat. There are no box shadows anywhere. Structure comes from horizontal rules in four strengths and from the paper ground. The only layered element is the help tooltip, which sits on the same paper colour as the page and is separated by a 1px Tip Border outline and a pointed arrow toward its term, not by a shadow.

### Named Rules
**The Ruled, Not Lifted Rule.** Separate content with 1px horizontal rules. Never with shadows, tinted panels or cards.

## Shapes

Square by default. Rules are full-width horizontal hairlines; tables have no vertical lines. The tooltip has 2px corners, focus rings and help terms 1px; the reserved filled button style has barely softened corners (3px). Two 2px ink strokes mark state: the bar under the open index tab, and the underline of a pressed strength toggle. The tab disclosure mark is drawn, not typed: two 1.5px strokes in an 11px square, the vertical one collapsing to make a minus when the tab opens. The tooltip's border-drawn triangular pointer is the only non-rectangular form and always points at the term it explains.

## Components

### Buttons and Controls
Every control on the page is text-styled; there is no filled, outlined or ghost button.
- **Strength toggles:** a row of Public Sans 14px buttons in Record Blue with a 45% underline at 5px offset, 22px apart, each with a 44px hit area, after a Graphite "Evidence strength" label. Pressed is ink, weight 500 and a 2px ink underline. Hover to Deep Record Blue over 120ms ease-out. The row is hidden until script runs.
- **Show everything:** the same text style at 13.5px, beside the answer count; it clears both filters.
- **Copy control:** a button styled as a small sans link (13.5px label, Record Blue, underlined) with a 44px-tall hit area and a live status message beside it in Graphite.
- **Focus:** the global 2px Record Blue outline at 3px offset.

### Links
- Record Blue, 1px underline at 4px offset, underline at 70% strength; hover to Deep Record Blue with a full-strength underline, 120ms ease-out.
- External links open in a new tab (`target="_blank" rel="noopener"`), with a visually hidden "(opens in a new tab)" for screen readers. Links within the site open in place.
- Standalone links (tab panel links, source links, footer links) are Public Sans 15px/22px with +0.01em tracking and a 5px underline offset. Links inside a sentence stay in the surrounding Newsreader.
- Masthead links are ink-coloured Public Sans 14px with a Nav Underline at 6px offset, turning blue on hover. The current page's link carries a 2px ink underline.

### Help Terms and Tips (signature)
- **Term:** an inline button that inherits the surrounding type exactly, with a 1.5px dotted underline in Help Dot at 4px offset and a help cursor. Hover, or the open state (marked with a `data-open` attribute), darkens the dots to ink. Used only for specialist verification and domain terms defined on the glossary page.
- **Tip:** paper-coloured panel, 1px Tip Border outline, 2px corners, 11px/13px padding, Newsreader 15.5px, width 228px to 290px. Opens left of, below or above the term with a pointer arrow. Fades in over 140ms and rises 2px over 220ms on cubic-bezier(.16,1,.3,1). Opens on hover, focus and tap; stays open while the pointer moves onto it; Escape and outside clicks close it.

### Clock Readings (signature)
Two Clock-sized readings with tabular numerals on a ruled two-column ledger, each under a small sans label and over a Graphite explanation capped at 40ch. The early reading carries a highlighter stroke through the lower part of its figures (yellow from 58% to 94% of the line, overhanging the digits by 0.08em to 0.1em). With script and no reduced-motion preference, the stroke wipes in from the left once, after fonts load, over 1100ms with a 350ms delay on cubic-bezier(.16,1,.3,1).

### Index Tabs (signature)
Ruled, not filled. Five disclosure buttons in the Tab role; closed labels are Graphite, the open label and hovered labels are ink (120ms). Each label carries the drawn +/- mark, which turns from plus to minus over 220ms. On desktop the open tab sits on the strip's rule like an index tab: a 2px ink bar under its label, flush with the rule. One panel is open at a time and all may be closed; the first is open on load, or the one a link points to. On tablets and phones the same heads stack as an accordion. Without script every panel is open and the heads are plain ink headings with no marks.

### Question List (signature)
Questions are Newsreader 20px links in two ruled columns, with a 45% Record Blue underline. The chosen question turns ink, drops its underline and gains a highlighter stroke behind its words (yellow from 58% to 92% of the line). Choosing one writes its fragment to the URL, shows the answer band and filters the evidence blocks; the strength toggles narrow them further.

### Answer Band
A band that appears only when a question is chosen: the question as a Headline in the aside (capped at 16ch, focused without a ring when it appears), then the Answer text in ink and a Graphite count line with Show everything.

### Evidence Blocks
Each block is a band whose aside holds its Headline, a line of strength tags (Public Sans 500 in ink, 10px below the heading, joined by middle dots) and Graphite notes. Blocks revealed by a filter fade in and rise 6px over 420ms on cubic-bezier(.16,1,.3,1). Where a band heading names a ledger, the ledger's caption is kept for screen readers only.

### Ledger Tables (signature)
- Full-width, collapsed, left-aligned, tabular lining numerals, Newsreader 17px cells under Public Sans label headers.
- Strong rule above the header row, a Header Rule below it, normal rules between rows. Cells 42px tall with 8px/10px/9px/12px padding.
- The Strength column sets its values as ink sans labels.
- On phones the header row is visually hidden and each row becomes a stacked block with the column name above each value.

### Witness Figure
A ruled two-column definition list: a small sans label, a large serif reading with tabular numerals, then a Graphite explanation capped at 40ch. The off reading carries a static highlighter stroke (yellow from 62% to 92% of the line).

### Command Block
Plex Mono 13.5px/22px in Code Ink with no background panel or border. Each command is its own line that wraps under itself with a 2ch hanging indent; flags never break, and revision hashes may break anywhere. Followed by the copy control.

### Navigation
A single-line masthead: the brand in Newsreader 28px ink at left, four sans links at right 36px apart, closed by a strong rule. Wraps on phones.

### Contribution and decisions

Unboxed definition lists in the existing band grid, capped at 65ch. Public Sans
500 labels at 13.5px/22px sit above Newsreader 17.5px/25px descriptions. Normal
rules separate entries, with 13px top and 15px bottom padding. Decision
boundaries occupy a separate muted line, 6px below the explanation.

## Do's and Don'ts

### Do:
- **Do** set prose, headings, tab labels, questions, tables and numbers in Newsreader, with optical sizing and tabular lining numerals in tables, clocks and readings.
- **Do** keep prose at a reading measure even on a wide page: about 36ch in the aside, 56ch for the answer, 65ch to 70ch in the wide column.
- **Do** put evidence in ruled ledger tables in the wide column, with the summary in the narrow aside.
- **Do** separate bands and rows with 1px horizontal rules in the rule tones.
- **Do** underline every link solidly in Record Blue, and give every specialist term a dotted Help Dot underline with a working explanation on hover, focus and tap.
- **Do** mark state with ink and a 2px ink stroke: the bar under the open index tab, the underline of a pressed toggle.
- **Do** keep every tab panel and every evidence block readable without script.
- **Do** keep table labels visible on phones by turning rows into labelled blocks.
- **Do** honour reduced motion: highlighter strokes appear fully drawn and transitions and animations are removed.

### Don't:
- **Don't** add box shadows, cards, tinted panels or vertical table rules.
- **Don't** fill, box or border index tabs; they sit on the rule.
- **Don't** use Highlighter Yellow for decoration or on more than the one decisive result in a band.
- **Don't** give familiar words (AI, plugin, code, tests) the dotted help treatment, and don't use a dotted underline on a link.
- **Don't** set headings or paragraphs in Public Sans, or add uppercase tracked labels.
- **Don't** add outlined, ghost or pill buttons, chip toggles, or corners rounder than 3px.
- **Don't** use typed glyphs or icon fonts for disclosure marks; draw them with strokes.
