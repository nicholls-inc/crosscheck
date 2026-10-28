# Crosscheck website

## Platform and readers

A static public website about one thing: the EVerest OCPP 1.6 case study.
The first reader is a curious engineer arriving from the "Review Evidence, Not
Code" essay or a shared link, who wants to know what was found, whether it is
real and how sure the study is. Hiring engineers and reviewers who want to
reproduce the result are also readers.

## Purpose

Explain one finding and make every claim behind it inspectable. The site is a
reading surface, not a product landing page: it does not sell or install
Crosscheck. Crosscheck appears once, in the footer, as "The tool behind this
study", linking to the repository.

## Selected approach

On 2026-10-28 the maintainer found the evidence-led landing page too
text-heavy and chose to rebuild the site around the case study alone, borrowing
two ideas from balfrontower.org: expandable index tabs at the top, and plain
questions that filter the evidence. A second filter sorts evidence by strength.
The confirmed brief is `intent/2026-10-28-everest-site-brief.md`. The study is
`site/index.html`; `site/everest.html` only forwards old links.

## Career and publication purpose

The website, published essay and OCPP case study tell one story about Harry
Nicholls's work on Crosscheck. Show the problem, contribution, AI assistance,
engineering decisions, measured result and limits. Relevant audiences include
AI platform and developer-tool teams, and teams applying AI to real problems.
The work supports applications for Senior/Staff Software Engineer and Product
Engineer roles as well as problem-specific side-door conversations.
Keep the study useful to engineers rather than turning it into a job
advertisement; the author appears in one line of the About tab. Application and side-door drafts live in publication docs;
personal job filters stay in local preparation notes outside the public output.

## Product truth

Proved, tested, observed and judged are distinct strengths. A model proof is not
a proof of the production implementation. AI suggestions are search outputs,
not evidence. Each published result names its assumptions, sources and limits.
The full goal-to-requirement-to-spec-to-code vision is not yet implemented.

## Visual authority

Preserve the owner's existing visual implementation: Newsreader prose and
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
