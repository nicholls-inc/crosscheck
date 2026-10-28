# Evidence-led Crosscheck website

## Intent

The published vision essay, EVerest OCPP study in PR #144, and website in the
landing-page worktree need one reader journey: understand evidence-carrying
code, inspect a real claim and its limits, then reproduce it or use Crosscheck.

The user selected Canvas approach `evidence`, version 2, and requested its build
on 2026-10-28. That selection authorizes the implementation described here.
The existing website's identity, glossary explanations and installation route
are retained. This branch adds a static site; the source worktree is read only.

## Outcome and users

Engineers and readers of the published essay can enter through EVerest's
evidence table, follow the smallest witness into a full study, download a
pinned publication record, and find replay and installation instructions.

## Constraints and decisions

- Use immutable study revision `7d05c4e62284b4101e8f23722ee3b6111e5ca0e9`.
- Treat historical results as reported, not checks rerun during this build.
- Preserve the distinction between model proofs and sampled production tests.
- Keep the report's classification caveat visible alongside its totals.
- Use a standalone website article; no external publication or upstream filing.
- Do not alter `nuanced/Untitled.md`, the published post, PR #144, other
  worktrees, verification tools, protected surfaces or deployment configuration.

## Open questions

None required for this implementation. External publication venue and timing
remain the owner's choice; the site article can be published with the site.

## Acceptance and plan

See [spec](2026-10-28-evidence-led-site-spec.md) and
[plan](2026-10-28-evidence-led-site-plan.md). This is a Tier 2 website change.
