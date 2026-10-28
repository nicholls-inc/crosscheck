# Crosscheck website

## Platform and readers

A static public website for engineers evaluating Crosscheck, readers of the
published vision essay, and reviewers who want to inspect or reproduce a claim.

## Purpose

Help readers understand, inspect and eventually produce evidence-carrying code.
The published essay supplies the argument, the EVerest OCPP 1.6 study supplies a
worked example, and the website connects that example to the existing tools.

## Selected approach

The user selected Canvas approach `evidence` version 2 on 2026-10-28: lead with
the EVerest evidence. The home page's primary action opens the study. The study
offers replay instructions and a downloadable editorial publication snapshot.
Installation remains available from navigation and after the study.

## Product truth

Proved, tested, observed and judged are distinct strengths. A model proof is not
a proof of the production implementation. AI suggestions are search outputs,
not evidence. Each published result names its assumptions, sources and limits.
The full goal-to-requirement-to-spec-to-code vision is not yet implemented.

## Visual authority

Preserve the owner's existing landing-page implementation: Newsreader prose and
headings, Public Sans labels, IBM Plex Mono commands, warm paper, dark ink,
blue underlined links and ruled evidence tables. Keep prose narrow and tables
wide. The selected Canvas previews refine this existing identity. This is an
extension, not a replacement visual direction. See DESIGN.md.

## Operating context

`site/` serves as static files with no installation or build required. Node's
built-in modules regenerate publication fragments and check consistency.
Specialist terms have help on hover, keyboard focus and touch, backed by a
standalone glossary. The article remains readable without JavaScript.

## Publication boundary

The EVerest results are reported from a pinned WIP study; this site does not
claim a new independent run. The publication venue and timing remain the
owner's decision. Deployment, edits to the published essay and upstream issue
submission are separate actions.
