# Product

<!-- impeccable:product-schema 1 -->

## Platform
web

The requested surface is a public landing page for the Crosscheck developer-tool suite; the plugin itself runs in Claude Code.

## Users
Developers evaluating and installing Crosscheck. This audience is inferred from the product documentation and the confirmed installation CTA; narrower audience segments remain undecided.

## Product Purpose
Crosscheck supplies tools for verifying AI-generated code. Its stated goal is correctness evidence that does not depend on trusting an AI's judgment.

## Positioning
The Claude Code plugin combines Dafny verification and extraction, Lean models and proofs, differential random testing, and specification and assurance workflows. Deterministic checks and human judgment count as evidence; LLM search does not establish guarantees.

## Operating Context
Install through the nicholls Claude Code marketplace:

```sh
claude plugin marketplace add nicholls-inc/claude-code-marketplace
claude plugin install crosscheck@nicholls
```

## Evidence on Hand
- Product and installation: README.md and crosscheck/README.md.
- Product principles and limits: docs/VISION.md and CLAUDE.md.
- First user-selected case study: https://github.com/nicholls-inc/crosscheck/pull/144. A Lean model of the OCPP 1.6 composite charging schedule proves three invariants and tests EVerest's implementation against generated inputs. The PR reports a recurring-profile leap-second bug. This is not a proof of the whole EVerest library. Its review items and stated limits must remain available in linked evidence.
- No additional library case studies have been confirmed for this surface.

## Brand Commitments
Name: Crosscheck. The user rejected the initial mockups as gaudy and inconsistent with Crosscheck's spirit. Those mockups are not visual authority. Start again from docs/VISION.md and https://www.are.na/harry-nicholls/crosscheck, which must be reviewed before further visual work. The user requested access through https://mcp.are.na/mcp.

The vision centers readable claims, explicit evidence strength, named trust assumptions and reproducible checks. Distinguish future-state commitments from capabilities shipping today.

The user also rejected the second round as going too far toward austerity. Its barely styled document layouts are not visual authority. The user refined the Are.na channel from 33 to 14 blocks and requested another fresh start plus a full reading of https://brutalist-web.design/.

The revised 14-block channel was fetched through the connected MCP. It retains The Phenomenal World (3260572), Balfron Tower (19290565), the shifting-house PDF (50808365), LOW-TECH MAGAZINE (35367518), On Being (13147628), Reed Rosenbluth (51969521), Filip Gres (51969613), Dandelion (51969618), Hamilton Ulmer (51969625), RJ Clarke (51969638), Einar Wahlstroem (51969640), minimal (51969701), Jimi Filipovski (51969708), and Physical Intelligence (51969734). Removed references such as Paco Coursey and Eric Li must not steer this round.

Interpretation for exploration: purposeful composition, warmer surfaces, stronger type hierarchy, ordinary readable links, useful tabular structures and content-bearing imagery. This is not yet an approved visual direction. Brutalist Web Design was read in full: it prioritizes readers, explicit controls, normal scrolling and browser behavior, useful decoration and performance; it does not require an unstyled page. Both prior mockup rounds remain rejected.

## Current Surface Scope
User-confirmed: primary CTA is install the plugin. On 2026-10-27 the user asked for the landing page to be built from the approved comp, which authorized a website implementation. The page is a static HTML, CSS and JavaScript site in `site/`, with self-hosted fonts and no build step.

## Selected Landing-page Direction
The approved comp is `.impeccable/mocks/round-3/record-plain-english-v2.png`: the evidence-record composition from round three (`record.png`), refined for density (wider content, shorter masthead, smaller type), plain English and narrow intro columns. The build in `site/` reproduces its first viewport and extends it below with the study's limits, how to use the plugin, and what ships today against the vision. A separate `site/glossary.html` page holds the glossary that supplies every help explanation; the landing page keeps a hidden copy for the tips, and `node scripts/check-site-glossary.mjs` fails if the two differ.

## Language and Inspectability
User requirement: the website must make its own claims and evidence transparent and inspectable, like Crosscheck's vision for code changes. Use plain English and minimal jargon. Ubiquitous software-engineering and technology terms can be used without definitions or help styling; the user explicitly exempts "AI" and "plugin". Reserve dotted underlines, a question-mark/help cursor and plain-English hover explanations for specialist verification or domain terms that readers may not know. Explanations must also be available with keyboard focus and touch. Keep the strength and scope of each claim visible; tooltips explain terms rather than hide qualifications.

The wide page does not require wide prose. The user found the two introduction paragraphs too wide and hard to read. Keep them in narrower, centered reading columns; the evidence table can use the wider page. Proposed desktop reading measure: roughly 45–60 characters per line.
