# Career evidence implementation plan

1. Commit this intent, spec and plan before implementation.
2. Update PRODUCT.md for the chosen approach and current document. Keep private
   job filters in .canvas/, and keep the live document untouched.
3. Extend site/everest-study.json with author/contribution/decision context;
   render it through scripts/build-site.mjs into site/everest.html. Add a quiet
   homepage author link. Preserve current typography and use the band layout.
4. Add site/LAUNCH.md, deploy/Caddyfile and deploy/crosscheck-site.service.example
   for the confirmed home server and existing tunnel. Test the origin using an
   installed Caddy binary with a temporary site root; do not deploy remotely.
5. Add scripts/check-public-site.mjs and focused checks for stale deployments.
   Add docs/publication/crosscheck-story.md with copy-ready application and
   outreach drafts plus a destination/relevance worksheet. No outreach sending.
6. Run generation, publication/link checks, Node tests, Caddy validation and
   local origin/public-route checks. Check desktop and phone interactions and
   screenshots in one batch, obtain a finish review, fix material findings and
   document any visual-system changes. Run ripwire contract and quality checks.
7. Review the exact diff under REVIEW.md, stage only intended files and commit.
   Offer Ship PR for the committed implementation after checks pass, clearly
   stating that public deployment and external outreach have not occurred.

## Risks

Do not overstate authorship or real-world impact. The case demonstrates a
reported low-severity finding and engineering decisions, not a deployed fix or
proven safety of an entire charger. Serve only site/, not the repository root.
Containerized cloudflared needs a reachable origin in its own network namespace;
localhost instructions assume a host-level tunnel agent and must say so.
