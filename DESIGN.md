---
name: Crosscheck
description: Check the code. Read the evidence.
colors:
  ground: "#f8f7f4"
  ink: "#18191b"
  text: "#25282e"
  muted: "#4f535b"
  code-ink: "#33363d"
  rule-strong: "#7d808a"
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
    lineHeight: "47px"
    letterSpacing: "-0.02em"
    fontVariation: "\"opsz\" 60"
  reading:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "46px"
    fontWeight: 400
    lineHeight: "52px"
    letterSpacing: "-0.02em"
    fontFeature: "\"lnum\", \"tnum\""
    fontVariation: "\"opsz\" 60"
  headline:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "32px"
    fontWeight: 400
    lineHeight: "38px"
    letterSpacing: "-0.03em"
    fontVariation: "\"opsz\" 48"
  title:
    fontFamily: "Newsreader, Iowan Old Style, Georgia, serif"
    fontSize: "25px"
    fontWeight: 400
    lineHeight: "32px"
    letterSpacing: "-0.012em"
    fontVariation: "\"opsz\" 36"
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
  nav:
    fontFamily: "Public Sans, Helvetica Neue, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  mono:
    fontFamily: "IBM Plex Mono, ui-monospace, Menlo, monospace"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: "23px"
rounded:
  hairline: "1px"
  tip: "2px"
  button: "3px"
spacing:
  gutter: "clamp(16px, 3.125vw, 48px)"
  aside: "clamp(300px, 26.7vw, 410px)"
  col-gap: "48px"
  band-pad: "28px"
  section-pad: "44px"
  cell: "8px 10px 9px 12px"
components:
  button-primary:
    backgroundColor: "{colors.button}"
    textColor: "{colors.button-label}"
    typography: "{typography.body}"
    rounded: "{rounded.button}"
    padding: "0 22px"
    height: "42px"
  button-primary-hover:
    backgroundColor: "{colors.button-hover}"
    textColor: "{colors.button-label}"
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
  ledger-cell:
    textColor: "{colors.text}"
    typography: "{typography.table}"
    padding: "{spacing.cell}"
    height: "42px"
  ledger-header:
    textColor: "{colors.ink}"
    typography: "{typography.table}"
    padding: "{spacing.cell}"
  finding-mark:
    backgroundColor: "{colors.mark}"
    textColor: "{colors.ink}"
    padding: "9px 15px 10px 12px"
  copy-button:
    textColor: "{colors.link}"
    typography: "{typography.label}"
  code-block:
    textColor: "{colors.code-ink}"
    typography: "{typography.mono}"
---

# Design System: Crosscheck

## Overview

**Creative North Star: "The Evidence Record"**

The page reads like a well-set research record: a warm paper ground, dark ink, a text serif doing nearly all the talking, and fine horizontal rules that divide the page into bands. Every visual device either helps someone read a claim or helps them inspect the evidence behind it. A wide ledger table carries the evidence; narrow columns carry the prose. There are no cards, no shadows, no gradients for atmosphere and no imagery that is not itself evidence.

Hierarchy comes from serif size, optical sizing and position, not from colour or weight. Weight stays near regular (380 for reading, 400 for headings, 500 only for table headers and small sans labels). The one warm accent, a pale highlighter yellow, marks the single result a reader most needs to see. Links look like links: blue and underlined. Specialist terms carry a dotted underline that opens a plain-English explanation; the two underline styles never trade meanings.

Density is moderate: generous enough to read, tight enough that the introduction, the first case study and its evidence table share the first desktop viewport. On narrow screens everything stacks in source order and tables become labelled rows rather than shrinking.

**Key Characteristics:**
- Warm off-white paper ground with near-black ink and greys tuned slightly cool.
- Newsreader for reading, headings and data; Public Sans only for navigation and small data labels; IBM Plex Mono only for commands and identifiers.
- Bands: a narrow summary column beside a wide evidence column, each band closed by a 1px rule.
- Ruled ledger tables with tabular numerals as the primary evidence form.
- One highlighter accent, used on a single finding per view.
- Solid underline means navigation; dotted underline means explanation.

## Colors

A near-monochrome paper-and-ink palette with one link blue and one highlighter yellow; colour is reserved for meaning.

### Primary
- **Record Blue** (link): every inline link and the copy control. Underlined at 1px, offset 4px, with the underline at 70% of the link colour. It is also the focus-ring colour.
- **Deep Record Blue** (link-hover): link hover, where the underline also goes to full strength.

### Secondary
- **Highlighter Yellow** (mark): the finding mark on the decisive table cell, the witness reading that is off, and text selection. Nothing else.

### Neutral
- **Warm Paper** (ground): page background and tooltip background.
- **Ink** (ink): headings, table headers, the brand, emphasised results, tooltip text.
- **Body Ink** (text): running prose and table cells.
- **Graphite** (muted): secondary notes, explanations under readings, mobile row labels, copy status.
- **Code Ink** (code-ink): inline code and command blocks; slightly lighter than body ink so commands sit back from prose.
- **Strong Rule** (rule-strong): band dividers, masthead rule, table top rules.
- **Rule** (rule): row dividers inside tables, glossaries and witness lists; scrollbar thumb.
- **Faint Rule** (rule-faint): reserved hairline tone in the token set.
- **Help Dot** (help): the dotted underline of help terms at rest.
- **Press Black** (button, button-hover): the single filled button and its hover.

### Named Rules
**The One Mark Rule.** Highlighter yellow marks the one result a reader must not miss in a given band. If two things are highlighted, neither is.

**The Ink Hierarchy Rule.** Hierarchy is carried by ink depth (ink, text, muted) and type size, never by introducing a new hue.

## Typography

**Display Font:** Newsreader (with Iowan Old Style, Georgia, serif), self-hosted variable, weights 200-800, optical sizing on.
**Body Font:** Newsreader, the same family.
**Label/Mono Font:** Public Sans (with Helvetica Neue, Arial) for navigation and data labels; IBM Plex Mono 400 (with ui-monospace, Menlo) for commands and code.

**Character:** A literary text serif used with scientific restraint, with a plain civic sans for wayfinding and a quiet mono for anything the reader might type. Optical-size settings (opsz 36 to 60) tighten the headings without heavier weights.

### Hierarchy
- **Display** (400, 40px/47px, -0.02em, opsz 60): the page title only, centred on desktop. 36px at 860px and below, 32px left-aligned at 640px and below.
- **Reading** (400, 46px/52px, tabular lining numerals): measured values in the witness figure. 38px on narrow screens.
- **Headline** (400, 32px/38px, -0.03em, opsz 48): band headings. The case-study name runs slightly larger (34px/40px). 28px on phones.
- **Title** (400, 25px/32px, opsz 36): the evidence heading above a ledger; table captions run at 22px/30px.
- **Body** (380, 19px/26px): prose. Introduction columns are capped near 430px, roughly 50 to 60 characters. 18px on phones.
- **Body note** (380, 17.5px/25px): notes, scope statements, figure captions, capped at 36ch to 78ch depending on context.
- **Table** (380, 17px/24px, tabular lining numerals): ledger rows; headers at 500 in ink.
- **Tip** (400, 15.5px/22px): help explanations.
- **Label** (Public Sans 500, 13.5px/22px): column labels in definition lists, claim-table row headers, footer heading. Sentence case, no tracking.
- **Nav** (Public Sans 400, 14px/20px): masthead links.
- **Mono** (IBM Plex Mono 400, 13.5px/23px): command blocks; inline code at 0.82em.

### Named Rules
**The Serif Carries It Rule.** Prose, headings, tables and numbers are all Newsreader. The sans appears only at label size for wayfinding and data labels; it never sets a heading or a paragraph.

**The Narrow Prose Rule.** A wide page does not mean wide prose. Running text stays near 45 to 60 characters per line; only tables use the full evidence column.

## Layout

A single centred page up to 1680px wide with a fluid gutter (16px to 48px). Below the masthead the page is a stack of **bands**: each is a two-column grid with a narrow aside (300px to 410px) holding the band's heading and summary, a 48px gap, and a wide evidence column holding tables, figures and commands. Each band ends with a 1px strong rule. The first band starts at 28px; later bands use 44px top and 48px bottom padding.

The introduction is the one centred composition: a centred display heading over two narrow prose columns (about 432px and 418px, 60px apart) and a centred action row.

Responsive behaviour: at 1100px the aside fixes at 300px and the gap narrows to 36px. At 860px every band and the footer collapse to one column in source order; definition grids go to one column. At 640px the masthead wraps, ledger tables become labelled rows (each cell preceded by its column name in a small sans label), and the action row left-aligns.

Spacing is set per context in pixel values rather than a strict scale; the recurring steps are 4px, 8px, 12px to 18px between related blocks, 28px band entry, 36px to 48px column gaps, and 44px to 48px section padding.

## Elevation & Depth

Completely flat. There are no box shadows anywhere. Structure comes from horizontal rules in three strengths and from the paper ground. The only layered element is the help tooltip, which sits on the same paper colour as the page and is separated by a 1px grey border and a pointed arrow toward its term, not by a shadow.

### Named Rules
**The Ruled, Not Lifted Rule.** Separate content with 1px horizontal rules. Never with shadows, tinted panels or cards.

## Shapes

Square by default. Rules are full-width horizontal hairlines; tables have no vertical lines. The filled button has barely softened corners (3px), the tooltip 2px, focus rings and help terms 1px. The tooltip's border-drawn triangular pointer is the only non-rectangular form and always points at the term it explains.

## Components

### Buttons
Quiet and decisive; there is exactly one filled button style.
- **Shape:** barely softened corners (3px).
- **Primary:** Press Black fill, white Newsreader 18px at 400, 42px minimum height, 170px minimum width, 22px horizontal padding.
- **Hover / Focus:** fill goes to pure black over 120ms ease-out; active nudges down 1px. Focus uses the global 2px Record Blue outline at 3px offset.
- **Secondary:** a plain underlined text link sits beside the button; there is no outlined or ghost button.
- **Text action:** the copy control is a button styled as a small sans link (13px, Record Blue, underlined) with a 44px-tall hit area and a live status message beside it in Graphite. It keeps the text-link look; there is no outlined button.

### Links
- Record Blue, 1px underline at 4px offset, underline at 70% strength; hover to Deep Record Blue with a full-strength underline, 120ms ease-out.
- Masthead links are ink-coloured Public Sans 14px with a grey underline at 6px offset, turning blue on hover.

### Help Terms and Tips (signature)
- **Term:** an inline button that inherits the surrounding type exactly, with a 1.5px dotted underline in Help Dot at 4px offset and a help cursor. Hover, or the open state (marked with a `data-open` attribute), darkens the dots to ink. Used only for specialist verification and domain terms defined in the page glossary.
- **Tip:** paper-coloured panel, 1px grey border, 2px corners, 11px/13px padding, Newsreader 15.5px, width 228px to 290px. Opens left of, below or above the term with a pointer arrow. Fades in over 140ms and rises 2px over 220ms on cubic-bezier(.16,1,.3,1). Opens on hover, focus and tap; stays open while the pointer moves onto it; Escape and outside clicks close it.

### Ledger Tables (signature)
- Full-width, collapsed, left-aligned, tabular lining numerals, Newsreader 17px.
- Strong rule above the header row, a darker rule below it, normal rules between rows. Cells 42px tall with 8px/10px/9px/12px padding.
- Row-header variants put a small sans label in a fixed 190px to 230px first column.
- On phones the header row is visually hidden and each row becomes a stacked block with the column name above each value.

### Finding Mark
The decisive result cell gets a full-bleed Highlighter Yellow block in ink. With JavaScript and no reduced-motion preference, it wipes in from the left over 900ms (250ms delay) when scrolled into view. A thinner highlighter-pen variant (yellow band from 62% to 92% of line height) marks an off reading in the witness figure.

### Witness Figure
A ruled two-column definition list: a small sans label, a large serif reading with tabular numerals, then a Graphite explanation capped at 40ch.

### Command Block
Plex Mono 13.5px in Code Ink, scrolling sideways inside the block rather than wrapping mid-command, with no background panel or border; followed by the copy control.

### Navigation
A single-line masthead: the brand in Newsreader 28px ink at left, four sans links at right 36px apart, closed by a strong rule. Wraps on phones.

## Do's and Don'ts

### Do:
- **Do** set prose, headings, tables and numbers in Newsreader, with optical sizing and tabular lining numerals in tables and readings.
- **Do** keep running prose near 45 to 60 characters per line, even on a wide page.
- **Do** put evidence in ruled ledger tables in the wide column, with the summary in the narrow aside.
- **Do** separate bands and rows with 1px horizontal rules in the three rule tones.
- **Do** underline every link solidly in Record Blue, and give every specialist term a dotted Help Dot underline with a working explanation on hover, focus and tap.
- **Do** keep table labels visible on phones by turning rows into labelled blocks.
- **Do** honour reduced motion: the finding mark appears fully drawn and transitions are removed.

### Don't:
- **Don't** add box shadows, cards, tinted panels or vertical table rules.
- **Don't** use Highlighter Yellow for decoration or on more than the one decisive result in a band.
- **Don't** give familiar words (AI, plugin, code, tests) the dotted help treatment, and don't use a dotted underline on a link.
- **Don't** set headings or paragraphs in Public Sans, or add uppercase tracked labels.
- **Don't** add a second filled button style, outlined buttons, or corners rounder than 3px.
