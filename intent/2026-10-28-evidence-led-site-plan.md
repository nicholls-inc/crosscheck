# Evidence-led website implementation plan

1. Commit the intent, spec and this plan before implementation.
2. Copy only the existing `site/` implementation and its local font assets from
   the landing-page worktree into this assigned worktree. Preserve the design.
3. Add `site/everest-study.json` as the editorial publication snapshot. Add
   `scripts/build-site.mjs` to render shared evidence into the landing page and
   standalone study article, without browser fetching or a runtime dependency.
4. Extend `site/index.html` with the selected evidence-led entry and essay link;
   add `site/everest.html` for the full study and replay path. Keep the existing
   installation instructions, how-to paths, glossary, help and responsive CSS.
5. Add a site checker and focused checks for publication consistency and local
   navigation. Document static preview, regeneration, source provenance and
   publication boundaries in `site/README.md` and a site journal.
6. Run generation freshness, publication/link/glossary checks and JavaScript
   syntax checks. Inspect desktop and phone layouts and interaction paths
   using an installed browser if available. Run ripwire change/quality checks.
7. Review the exact diff under REVIEW.md, stage only intended files, commit,
   and offer the provider's Ship PR action. Do not push or deploy.

## Risks

An older site revision currently uses different result totals. Render both
surfaces from one pinned record to prevent drift. The pinned report contains a
legacy note inconsistent with its newer warning rule; disclose this explicitly.
Static HTML must remain fully readable when scripts fail. The existing site
worktree must remain untouched throughout.
