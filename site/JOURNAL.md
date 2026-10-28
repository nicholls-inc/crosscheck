# Website journal

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
