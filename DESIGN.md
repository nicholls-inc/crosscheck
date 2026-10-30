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
  tip-border: "#9a9da2"
  rule: "#b6b8be"
  link: "#2a5a9e"
  link-hover: "#153c74"
  mark: "#f8eab4"
  help: "#6d717a"
typography:
  headline:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "min(36px, 8.4vw)"
    fontWeight: 320
    lineHeight: "1.12"
    letterSpacing: "-0.03em"
    fontVariation: "\"opsz\" 72"
  reading:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "46px"
    fontWeight: 380
    lineHeight: "52px"
    letterSpacing: "-0.02em"
    fontFeature: "\"lnum\", \"tnum\""
    fontVariation: "\"opsz\" 60"
  question-heading:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "28px"
    fontWeight: 400
    lineHeight: "34px"
    letterSpacing: "-0.02em"
    fontVariation: "\"opsz\" 48"
  title:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "24px"
    fontWeight: 400
    lineHeight: "31px"
    letterSpacing: "-0.012em"
    fontVariation: "\"opsz\" 36"
  wordmark:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "20px"
    fontWeight: 520
    lineHeight: "20px"
    letterSpacing: "-0.004em"
    fontVariation: "\"opsz\" 24"
  answer:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "21px"
    fontWeight: 380
    lineHeight: "30px"
    fontVariation: "\"opsz\" 24"
  entry-title:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "20px"
    fontWeight: 450
    lineHeight: "27px"
    letterSpacing: "-0.005em"
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
  meta:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "16px"
    fontWeight: 380
    lineHeight: "23px"
  tip:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "15.5px"
    fontWeight: 400
    lineHeight: "22px"
  section-label:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 500
    lineHeight: "26px"
    letterSpacing: "0.01em"
  control:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: "22px"
    letterSpacing: "0.01em"
  label:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "13.5px"
    fontWeight: 500
    lineHeight: "22px"
    letterSpacing: "0.01em"
  mono:
    fontFamily: "IBM Plex Mono, ui-monospace, Menlo, monospace"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: "22px"
rounded:
  hairline: "1px"
  tip: "2px"
spacing:
  gutter: "clamp(20px, 12vw, 240px)"
  gutter-phone: "24px"
  column-max: "1200px"
  aside: "clamp(140px, 15vw, 220px)"
  col-gap: "48px"
  col-gap-narrow: "36px"
  disclosure-indent: "18px"
  cell: "8px 10px 9px 12px"
components:
  link:
    textColor: "{colors.link}"
  link-hover:
    textColor: "{colors.link-hover}"
  wordmark:
    textColor: "{colors.ink}"
    typography: "{typography.wordmark}"
  brand-mark:
    textColor: "{colors.ink}"
    size: "18px"
  study-title:
    textColor: "{colors.ink}"
    typography: "{typography.headline}"
  section-label:
    textColor: "{colors.link}"
    typography: "{typography.section-label}"
  section-label-hover:
    textColor: "{colors.link-hover}"
  section-first-line:
    textColor: "{colors.ink}"
    typography: "{typography.body}"
  question-link:
    textColor: "{colors.link}"
    typography: "{typography.body}"
    padding: "5px 0"
  question-link-current:
    textColor: "{colors.ink}"
  strength-toggle:
    textColor: "{colors.link}"
    typography: "{typography.control}"
    height: "44px"
  strength-toggle-pressed:
    textColor: "{colors.ink}"
  strength-icon:
    textColor: "{colors.muted}"
    size: "16px"
  filter-label:
    textColor: "{colors.ink}"
    typography: "{typography.label}"
  answer-question:
    textColor: "{colors.ink}"
    typography: "{typography.question-heading}"
  answer-text:
    textColor: "{colors.ink}"
    typography: "{typography.answer}"
  show-all:
    textColor: "{colors.link}"
    typography: "{typography.label}"
    height: "44px"
  group-title:
    textColor: "{colors.muted}"
    typography: "{typography.label}"
  entry-title:
    textColor: "{colors.ink}"
    typography: "{typography.entry-title}"
  entry-meta:
    textColor: "{colors.muted}"
    typography: "{typography.meta}"
  highlight-stroke:
    backgroundColor: "{colors.mark}"
    textColor: "{colors.ink}"
  help-term:
    textColor: "{colors.text}"
  tip:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    typography: "{typography.tip}"
    rounded: "{rounded.tip}"
    padding: "11px 13px 12px"
    width: "min(clamp(228px, 17.8vw, 290px), calc(100vw - 32px))"
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

The page reads like a well-set research record: a warm paper ground, dark ink, a text serif doing nearly all the talking, and fine horizontal rules that divide the page into regions. Every visual device either helps someone read a claim or helps them inspect the evidence behind it. Content opens in place rather than being laid out all at once: one-line sections at the top, a filter row of questions and evidence strengths, then a grouped list of collapsed evidence entries, as in a well-kept archive. There are no cards, no shadows, no gradients for atmosphere and no imagery that is not itself evidence.

Hierarchy comes from serif size, optical sizing and position, not from colour or weight. Weight stays near regular (380 for reading, 400 for headings, 450 for evidence entry titles, 500 only for small sans labels and a pressed filter). The one warm accent, a pale highlighter yellow, marks the single result a reader most needs to see. Links look like links: blue and underlined. Specialist terms carry a dotted underline that opens a plain-English explanation; the two underline styles never trade meanings.

Density is calm: a small lockup, the study title, four single lines and the filter row share the first desktop viewport, with the page in a centred column and wide margins either side. On narrow screens everything stacks in source order, the margins shrink to 24px, and tables become labelled rows rather than shrinking.

**Key Characteristics:**
- Warm off-white paper ground with near-black ink and greys tuned slightly cool.
- Newsreader for reading, headings, questions, entry titles and data; Public Sans only for controls, standalone links and small labels; IBM Plex Mono only for commands and identifiers.
- A narrow label column beside a wide content column, repeated by the filter row, the answer, each evidence group and the footer.
- Native disclosures that open in place: one-line sections and collapsed evidence entries.
- Ruled ledger tables with tabular numerals as the primary evidence form.
- One highlighter accent, used on a single result per region.
- Solid underline means navigation; dotted underline means explanation.

## Colors

A near-monochrome paper-and-ink palette with one link blue and one highlighter yellow; colour is reserved for meaning.

### Primary
- **Record Blue** (link): every inline link, the section labels and their colons, the drawn plus/minus, question links, strength toggles, Show everything, Back to the filters and the copy control. Underlined at 1px, offset 4px in prose and 5px on sans controls, with the underline at 70% of the link colour on prose links, 60% on section labels and 45% on question links, toggles and Show everything, where many links stack. It is also the focus-ring colour.
- **Deep Record Blue** (link-hover): link and control hover, where the underline also goes to full strength.

### Secondary
- **Highlighter Yellow** (mark): the chosen question, the key evidence entry's title until it is opened, the witness reading that is off, and text selection. Nothing else.

### Neutral
- **Warm Paper** (ground): page background and tooltip background.
- **Ink** (ink): the lockup, the study title, the first line of each section, filter and evidence labels, the chosen question, the pressed strength toggle, the answer question and text, entry titles, glossary terms, table headers, strength values, readings, tooltip text.
- **Body Ink** (text): running prose, table cells, the no-script strength key, the footer sentence.
- **Graphite** (muted): group titles, strength icons in entries, entry meta lines, aside notes, the longer glossary account, explanations under readings, the filter hint, strength counts, filter status, the answer count line, mobile row labels, copy status, the copyright line.
- **Code Ink** (code-ink): inline code and command blocks; slightly lighter than body ink so commands sit back from prose.
- **Header Rule** (rule-header): the rule under a ledger header row, one step darker than the strong rule so the header reads as a heading line.
- **Strong Rule** (rule-strong): the rule closing the sections, the filter row and the answer; the rule above the footer and the glossary; the top rule of ledgers, the witness and glossary lists; the hover underline of entry titles.
- **Tip Border** (tip-border): the tooltip's 1px outline and pointer; the only edge drawn around a layer.
- **Rule** (rule): the rule above each evidence group, row dividers in tables and lists, scrollbar thumb.
- **Help Dot** (help): the dotted underline of help terms at rest, and a strength toggle with no evidence under the chosen question.

### Named Rules
**The One Mark Rule.** Highlighter yellow marks the one result a reader must not miss in a given region. If two things are highlighted, neither is. The key entry's title gives up its mark when the entry opens, because the reading inside carries it.

**The Ink Hierarchy Rule.** Hierarchy is carried by ink depth (ink, text, muted) and type size, never by introducing a new hue.

## Typography

**Display Font:** Newsreader (with Iowan Old Style, Georgia, serif), self-hosted variable, weights 200-800, optical sizing on.
**Body Font:** Newsreader, the same family.
**Label/Mono Font:** Public Sans (with Helvetica Neue, Arial) for controls, standalone links and small labels; IBM Plex Mono 400 (with ui-monospace, Menlo) for commands and code.

**Character:** A literary text serif used with scientific restraint, with a plain civic sans for wayfinding and a quiet mono for anything the reader might type. Optical-size settings (opsz 24 to 60) tighten the headings and figures without heavier weights.

### Hierarchy
- **Reading** (380, 46px/52px, -0.02em, opsz 60, tabular lining numerals): the two measured values in the witness figure, the largest type on the site. 38px/44px at 860px and below.
- **Headline** (320, min(36px, 8.4vw)/1.12, -0.03em, opsz 72): the study title on both pages, the page's one primary heading. A light cut at the largest optical size, so the title reads as display type while the heavier wordmark above stays smaller; weight, not size, separates them. It scales with the viewport (about 33px at 390px). The glossary's own heading uses the Question heading role (28px/34px).
- **Question heading** (400, 28px/34px, -0.02em, opsz 48, 28ch): the chosen question heading the answer.
- **Title** (400, 24px/31px, -0.012em, opsz 36): a visible ledger caption. Captions that only repeat an entry title are kept for screen readers.
- **Wordmark** (520, 20px/20px, -0.004em, opsz 24): "Crosscheck" beside the 18px mark, nudged down 0.11em so its capitals centre on it. Heavier and smaller than the light title below.
- **Answer** (380, 21px/30px, opsz 24, ink, 56ch): the short answer to the chosen question. 19px/27px on phones.
- **Entry title** (450, 20px/27px, -0.005em, ink): each evidence entry's title. 19px/26px on phones.
- **Body** (380, 19px/26px): the first line of each section (ink), the question links, and prose with no smaller role. 18px on phones, where the section first line and the questions drop to 17.5px/25px and 18px/25px.
- **Body note** (380, 17.5px/25px): opened section text (65ch), aside notes, study prose and lists, contribution lists, evidence notes and scope statements (65ch to 70ch), the witness caption and the footer sentence. Bold inside section text is 550 in ink.
- **Table** (380, 17px/24px, tabular lining numerals): ledger rows and the Graphite explanations under readings (40ch).
- **Meta** (380, 16px/23px, Graphite): the line under each entry title naming the source and strength.
- **Tip** (400, 15.5px/22px): help explanations.
- **Section label** (Public Sans 500, 15px/26px, +0.01em, Record Blue): About, Finding, Study, Repeat, each followed by a colon.
- **Control** (Public Sans 400, 15px/22px, +0.01em): strength toggles (500 when pressed), the no-script strength key, and standalone links (section links, source links, footer links) with a 5px underline offset. Back to the filters is the same at 14px.
- **Label** (Public Sans 500, 13.5px/22px, +0.01em): the Questions, Evidence strength and Evidence labels, table headers, the Strength column, witness labels, contribution terms. At 400: group titles in Graphite, the filter status, the answer count line, Show everything, the copy control and its status, the copyright line. Sentence case. Mobile row labels run at 13px in Graphite.
- **Mono** (IBM Plex Mono 400, 13.5px/22px): command blocks; inline code at 0.82em.

### Named Rules
**The Serif Carries It Rule.** Prose, the title, questions, answers, entry titles, table cells and numbers are all Newsreader. The sans names things the reader acts on or scans by: section labels, filter controls, standalone links, the small labels that head the filter row and the evidence groups, table headers and data labels. It never sets the title, a question, an answer, an entry title or a paragraph, and it always carries +0.01em tracking at these sizes.

**The Reading Measure Rule.** A wide page does not mean wide prose. An opened section's first line stops at 62ch and its text at 65ch, the answer at 56ch, and prose in evidence entries at 65ch to 70ch at most; only tables and the witness figure use the full content column.

## Layout

A centred column at most 1200px wide inside fluid gutters of clamp(20px, 12vw, 240px), so wide screens get wide margins; on phones the gutter is 24px. The column ends with 64px of padding. There is no masthead and no navigation header.

The page runs in this order: lockup and title, sections, filter row, answer (shown only once a question is chosen), evidence list, footer.

- **Lockup and title.** 40px from the top (26px on phones): the 18px mark and the wordmark, 9px apart, linking to the repository; then the study title 22px below.
- **Sections.** Four one-line disclosures, all closed by default, full width, 22px above and 30px below, closed by a strong rule. Their text is indented 18px under the label to clear the drawn plus.
- **Label-column grid.** The filter row, the answer, each evidence group and the footer share one grid: a label column of clamp(140px, 15vw, 220px), a 48px gap (36px at 1100px and below), and the content column. Labels in that column sit 4px in.
- **Filter row.** Three columns: the Questions label with a Graphite hint 4px under it in the Label role at 400 (13.5px/20px, capped at 26ch), the quietest text in the row ("Each shows a short answer and the evidence for it."); the questions themselves, lifted 9px so the first one sits on the labels' line; and the Evidence strength label and toggles at 250px to 320px. The row carries two labels and no others: no row-wide "Filter" label and no group titles over the questions; 36px above, 34px below, closed by a strong rule. At 1100px and below the strengths move under the questions; at 860px all three stack.
- **Answer.** The label column stays empty; the question heading and the answer sit in the content column, 34px above, 32px below, closed by a strong rule.
- **Evidence list.** The Evidence label, then groups in the order Overview, Finding, Proof, Tests, Assumptions, Limits, Repeat, People. Overview holds the evidence at a glance; People comes last so the record reads before the account of who decided it. Each group opens with a normal rule, its title in the label column and its entries beside it. A group with no visible entry hides.
- **Footer.** A strong rule, then the sentence "The tool behind this study: Crosscheck", with the mark inline, in the label column; site links, then the X and GitHub links in the content column. On phones they stack in that order. The copyright line sits outside the footer element, centred on the page, 48px below the footer and 22px above the page bottom.

Responsive behaviour: at 1100px the column gap narrows to 36px and the filter row goes to two columns. At 860px every label-column grid collapses to one column in source order, labels lose their 4px inset, and the witness grid and the glossary heading row go to one column. At 640px the gutter becomes 24px, the title drops to 26px, each section's label runs into its first sentence on the same line, like a run-in heading, and the sentence wraps in full beneath it, and the sections' and entries' plus marks hang 17px into the page margin so every text block keeps the same 24px margin on both sides, and ledger tables become labelled rows (each cell preceded by its column name in a small sans label).

With script, any filter change scrolls the page so the filter row's top edge sits at the top of the screen, and the evidence list keeps at least a screen's height so it can. Below 861px, where the stacked filters fill the screen, the page anchors to the answer, or to the evidence list when only a strength is pressed. Without script, every entry still opens, each question is a jump link to its first entry, the strength key is a plain list, and each entry ends with "Back to the filters".

The glossary page (`glossary.html`) shares the lockup, title line and footer, and its body is set as the study page's evidence list. A strong rule 28px below the title, then a heading row on the label-column grid. Its label column holds "Back to the study", a standalone Public Sans 15px Record Blue link with a 44px hit area, its line centred on the heading's (above the heading on phones). The content column holds "Glossary" (the page's h1) in the Question heading role (28px/34px) and a Graphite Body note intro under it, both on the content column's edge so they line up with the terms. Then five groups named as on the study page: EVerest and OCPP, Proof, Tests, Assumptions, People. Each group opens with a normal rule, its title in the label column and its terms beside it. Every term is an always-open entry: the term in the entry-title role (Newsreader 450, 20px/27px, ink), its short definition in Body note at 65ch, and the longer account a step quieter in Graphite at 16px/24px. Under the intro, a list of all thirteen terms as jump links (Public Sans 15px Record Blue with 44px hit areas, wrapping). Each term title links to itself in ink, underlined in Strong Rule only on hover, so a reader can copy a link to one term. Definitions and longer accounts are each capped at 65ch in their own size, so the 16px account runs no longer than the definition. The longer accounts are plain, one idea per paragraph. A link into the glossary (`glossary.html#everest`) lands 24px below the top (the page's scroll padding) and marks that term with the highlighter. Three evidence entries on the study page (Why it happens, What the tests report, Agreement at each instant) end with a Public Sans "More on … in the glossary" link to the term's longer account, and `?explain=` with a term the study page lacks forwards to that glossary fragment. The site's name for the program EVerest is compared with is "reference model", the term used in Lean-based differential testing; "the Lean model" and "the model" are its short forms. No rules between terms within a group, and no second column.

Spacing is set per context in pixel values rather than a strict scale; the recurring steps are 4px insets, 6px to 18px between related lines, 22px to 36px of region padding, and 48px column gaps.

## Elevation & Depth

Completely flat. There are no box shadows anywhere. Structure comes from horizontal rules in three strengths plus the darker ledger header rule, and from the paper ground. The only layered element is the help tooltip, which sits on the same paper colour as the page and is separated by a 1px Tip Border outline and a pointed arrow toward its term, not by a shadow.

### Named Rules
**The Ruled, Not Lifted Rule.** Separate content with 1px horizontal rules. Never with shadows, tinted panels or cards.

## Shapes

Square by default. Rules are full-width horizontal hairlines; tables have no vertical lines. The tooltip has 2px corners, focus rings and help terms 1px. The tooltip's border-drawn triangular pointer always points at the term it explains.

Every mark is drawn, never typed. The Crosscheck mark, Counterproof, is two filled rectilinear proof signs that face each other and do not meet, in currentColor at 22px in the lockup and 0.85em inline. The section plus is two 1.5px strokes in a 10px square; the vertical one collapses to make a minus when the section opens. Evidence entries carry the same plus in Graphite. The five strength icons are 16px outline drawings with 1.5px round-capped strokes: a ticked square (proved), a flask (tested), a person (decided by Harry), a dashed circle (assumed) and a square left open at its top-right corner (not yet reached, which includes an unfinished proof: the proved square, not yet closed). Each reads as a kind of evidence, never as a control: no lines that read as a menu or an equals sign, no dots that read as "more", no ring that reads as a radio button or a spinner. The only 2px stroke marks state: the underline of a pressed strength toggle.

## Components

### Lockup
The 18px Counterproof mark and the 20px Newsreader wordmark at weight 520 in ink, 9px apart, with no underline at rest; hovering underlines the wordmark at 5px offset. It links to the repository in a new tab. The study title below it is the page's one primary heading. On the study page it is not a link; on the glossary it links home without an underline.

### Sections (signature)
Native disclosures, one line each until opened. The summary holds a Record Blue section label with a drawn plus to its left and a colon after it, then the section's first sentence in ink at Body size, truncated with an ellipsis. Hovering deepens the label and its underline. Opened, the plus becomes a minus, the first sentence wraps and stops at 62ch, and the rest follows in Body note at 65ch with one standalone link at the end. On phones the label runs into the first sentence on the same line and the sentence wraps in full beneath it; the plus hangs in the page margin, centred on the first line.

### Question List (signature)
Questions are three short ordered lists, what happened, how sure the study is, and doing it yourself, separated by 18px of space and never titled; the questions name their own group. Each is a Newsreader 19px link with a 45% Record Blue underline and 5px of padding above and below. The chosen question turns ink, drops its underline and gains a highlighter stroke behind its words (yellow from 58% to 92% of the line). Choosing one writes its fragment to the URL, shows the answer and filters the evidence list; choosing it again clears it. The chosen link carries a visually hidden "(chosen; choose it again to clear)" for screen readers.

### Strength Filter
Five text-styled toggles stacked under an "Evidence strength" label, each a 16px drawn icon 10px before a Public Sans 15px label in Record Blue with a 45% underline at 5px offset, in rows at least 44px tall. Once a question is chosen, each label is followed by a Graphite count of the evidence under that question, in tabular numerals 6px after it; before then the toggles carry no counts, because a dead end exists only once a question narrows the list; a toggle whose count is 0 turns Help Dot with no underline but stays pressable. Pressing one filters the list, pressing again clears it; pressed is ink, weight 500 and a 2px ink underline. Hover goes to Deep Record Blue. Without script the toggles are hidden and the same icons and words show as a plain key in Body Ink. A Graphite status line reports the count when only a strength is pressed, or says when a link names a question or strength the page does not have. The URL fragment carries both filters: `#q-<question>`, `#s=<strength>`, or `#q-<question>&s=<strength>`; every change is a history entry, so Back undoes it and a shared link restores it.

### Answer
Each answer exists once, as a `data-answer` block placed under its question's anchor in the evidence list. Without script that block shows there (the question in a small label above it), so a reader who follows a question link still gets the answer. With script those inline copies hide and the answer band shows the chosen one. scripts/check-site.mjs fails if an inline question drifts from its link or a question lacks exactly one answer. The went-wrong and matter answers end by naming the filed upstream issue, EVerest #3323, with a link. A bad filter link is reported in Ink under the Questions hint, which stays on screen at every width, and the page scrolls the filters into view. The band is shown only when a question is chosen: the question as a 28px heading (capped at 28ch, focused without a ring when it appears), the 21px ink answer set as one short paragraph per idea with 12px between them, then a Graphite count line with Show everything, a Record Blue text button with a 44px hit area that clears both filters. When the strength leaves nothing, it reads "Clear the strength filter" instead. At 860px and below a second text button, "Change the question", scrolls back to the filters without changing them; "Back to the filters" does the same.

### Evidence Entries (signature)
Each entry is a native disclosure with no box. The title row is an `h4` inside the summary, so heading navigation reaches every entry, and a hidden separator keeps the title and meta line from being read as one word. The summary holds the 20px ink title, its strength icons in Graphite 6px apart, and a 16px Graphite meta line naming the source and strengths. A Graphite plus, the sections' drawn mark, hangs 18px left of the title in the column gap and becomes a minus when the entry opens; at 860px and below it hangs 17px into the page margin instead, so entry text keeps the group titles' left edge. Hovering underlines the title in Strong Rule. The key entry's title carries the highlighter until it is opened. Opened, the body follows with 10px above and 30px below: a Graphite note first, then the evidence itself (ledgers, the witness figure, lists, prose, commands). With script and no reduced-motion preference, the body fades in and rises 4px over 360ms on cubic-bezier(.16,1,.3,1). Without script each body ends with "Back to the filters".

### Links
- Record Blue, 1px underline at 4px offset, underline at 70% strength; hover to Deep Record Blue with a full-strength underline, 120ms ease-out.
- External links open in a new tab (`target="_blank" rel="noopener"`), with a visually hidden "(opens in a new tab)" for screen readers. Links within the site open in place.
- Standalone links (section links, source links, footer links, Back to the filters) have a hit area at least 44px tall from vertical padding on an inline block, without changing the visible rhythm. They are Public Sans 15px/22px with +0.01em tracking and a 5px underline offset. Links inside a sentence stay in the surrounding Newsreader.
- **Focus:** the global 2px Record Blue outline at 3px offset with 1px corners.

### Help Terms and Tips (signature)
- **Term:** an inline button that inherits the surrounding type exactly, with a 1.5px dotted underline in Help Dot at 4px offset and a help cursor. Hover, or the open state, darkens the dots to ink. Used only for specialist verification and domain terms defined on the glossary page.
- **Tip:** paper-coloured panel, 1px Tip Border outline, 2px corners, 11px/13px padding, Newsreader 15.5px, width 228px to 290px. Opens below or above the term with a pointer arrow. Fades in over 140ms and rises 2px over 220ms on cubic-bezier(.16,1,.3,1). Opens on hover, focus and tap; stays open while the pointer moves onto it; Escape and outside clicks close it.

### Ledger Tables (signature)
- Full-width, collapsed, left-aligned, tabular lining numerals, Newsreader 17px cells under Public Sans label headers.
- Strong rule above the header row, a Header Rule below it, normal rules between rows. Cells 42px tall with 8px/10px/9px/12px padding.
- The Strength column sets its values as ink sans labels.
- On phones the prose-heavy ledgers (the claims table and the not-yet-reached table) hide their header row and turn each row into a stacked block with the column name above each value. The two numeric ledgers (`ledger-numeric`: case totals and instant counters) stay real tables with a visible header, so their columns can still be compared.

### Witness Figure
A ruled two-column definition list: a small sans label, a Reading-sized value with tabular numerals, then a Graphite explanation capped at 40ch. The off reading carries a static highlighter stroke (yellow from 62% to 92% of the line).

### Command Block
Plex Mono 13.5px/22px in Code Ink with no background panel or border. Each command is its own line that wraps under itself with a 2ch hanging indent; flags never break, and revision hashes may break anywhere. Followed by the copy control: a 13.5px sans Record Blue text button with a 44px hit area and a live Graphite status beside it.

### Quote
Harry's first-person account is the only first-person text on a third-person page, so it is set apart: a figure indented 18px, Newsreader 19px/28px in Ink at opsz 24, capped at 60ch, with a Graphite opening quote mark at 1.4em hung in the indent, its top on the first line's cap height, and a closing mark after the last paragraph, then a Graphite Label caption reading "Harry". No italic (Newsreader ships roman only), no border and no panel.

### Definition Lists
Unboxed lists capped at 65ch for who did what and the study's design decisions: Public Sans 500 terms at 13.5px/22px above Newsreader 17.5px/25px descriptions, normal rules between entries, 13px above and 15px below each. Decision boundaries take a separate Graphite line 6px below. The glossary uses the evidence-entry pattern instead (see Layout); a term with more to say adds a longer Graphite account 8px under the short definition that tooltips use.

## Do's and Don'ts

### Do:
- **Do** set prose, the title, questions, answers, entry titles, tables and numbers in Newsreader, with optical sizing and tabular lining numerals in tables and readings.
- **Do** keep prose at a reading measure even on a wide page: 62ch for an opened section's first line, 65ch for section text, 56ch for the answer, 65ch to 70ch in evidence entries.
- **Do** hold labels in the narrow label column and content in the wide column, in every region that uses the grid.
- **Do** let sections and evidence entries open in place as native disclosures, readable and usable without script.
- **Do** put evidence in ruled ledger tables, and separate regions and rows with 1px horizontal rules in the rule tones.
- **Do** underline every link solidly in Record Blue, and give every specialist term a dotted Help Dot underline with a working explanation on hover, focus and tap.
- **Do** mark state with ink: the chosen question, the pressed toggle with its 2px ink underline.
- **Do** draw every mark and icon as an SVG or CSS stroke in currentColor.
- **Do** keep table labels visible on phones by turning rows into labelled blocks.
- **Do** honour reduced motion: transitions and animations are removed.

### Don't:
- **Don't** add box shadows, cards, tinted panels or vertical table rules.
- **Don't** box or border sections or evidence entries; they open on the rule.
- **Don't** add a hero, a masthead or a navigation header above the lockup.
- **Don't** use Highlighter Yellow for decoration or on more than the one decisive result in a region.
- **Don't** give familiar words (AI, plugin, code, tests) the dotted help treatment, and don't use a dotted underline on a link.
- **Don't** set the title, questions, answers, entry titles or paragraphs in Public Sans, or add uppercase tracked labels.
- **Don't** add filled, outlined, ghost or pill buttons, chip toggles, or corners rounder than 2px.
- **Don't** use typed glyphs, emoji or icon fonts for disclosure marks or strength icons.
