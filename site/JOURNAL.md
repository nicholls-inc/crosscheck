# Website journal

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
