# Crosscheck website

## Platform and readers

A static public website for engineers evaluating Crosscheck, readers of the
published vision essay, and reviewers who want to inspect or reproduce a claim.
Hiring engineers are also readers: the case study should make the author's
contribution and judgment clear in applications and specific conversations.

## Purpose

Help readers understand, inspect and eventually produce evidence-carrying code.
The published essay supplies the argument, the EVerest OCPP 1.6 study supplies a
worked example, and the website connects that example to the existing tools.

## Selected approach

The user selected Canvas approach `evidence` version 3 on 2026-10-28: lead with
the EVerest evidence. The home page's primary action opens the study. The study
offers replay instructions and a downloadable editorial publication snapshot.
Installation remains available from navigation and after the study.

## Career and publication purpose

The website, published essay and OCPP case study tell one story about Harry
Nicholls's work on Crosscheck. Show the problem, contribution, AI assistance,
engineering decisions, measured result and limits. Relevant audiences include
AI platform and developer-tool teams, and teams applying AI to real problems.
The work supports applications for Senior/Staff Software Engineer and Product
Engineer roles as well as problem-specific side-door conversations.
Keep the product useful to engineers rather than turning its homepage into a
job advertisement. Application and side-door drafts live in publication docs;
personal job filters stay in local preparation notes outside the public output.

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

The confirmed host is the owner's home server, exposed through an existing
Cloudflare Tunnel, as with their other public apps. `site/LAUNCH.md` documents
a loopback-only Caddy origin and the existing-tunnel handoff. The public hostname
is a deployment input, not an invented URL in the site. Launch verification must
check the public route before sharing it in applications or outreach.
