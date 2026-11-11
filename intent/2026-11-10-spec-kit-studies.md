# Intent: Spec Kit studies, and a site with more than one study

## Problem statement

On 2026-10-23 Harry filed 28 defect reports against GitHub's Spec Kit
(github/spec-kit #5071 to #5098), found and reproduced with Claude Code and
Crosscheck. Nothing on the Crosscheck site records them. The site at
`crosscheck.harrynicholls.com` holds one study, the EVerest OCPP 1.6 study, at
its root, so a second study has nowhere to go.

## Proposed outcome

- The root is a short landing page: a study index, three paragraphs on the
  tool, and the install commands.
- The EVerest study moves to `/studies/everest-ocpp/` with its glossary and
  record. `/everest.html`, `/glossary.html` and old fragment links on the root
  (`/#witness`, `/#q-proof&s=tested`) forward to it.
- `/studies/spec-kit/` presents all 28 reports: what goes wrong, which line
  of Spec Kit's own documentation, docstring, test or code each contradicts,
  and the upstream state of each, with the same question and strength filters
  as the EVerest study.
- `/studies/spec-kit-expressions/` looks closer at #5071, #5072 and #5073 on
  the workflow expression evaluator: the recorded outputs, what Jinja2 gives,
  and what changed upstream.
- `site/studies/spec-kit/spec-kit-study.json` is the record both pages render
  from. `scripts/check-site.mjs` fails if a count the hand-written answers
  repeat no longer matches it.

## Affected users and systems

Readers of the site and anyone holding an old link. `site/`,
`scripts/build-site.mjs`, `scripts/site-spec-kit.mjs`,
`scripts/check-site.mjs`, `scripts/check-public-site.mjs`,
`scripts/render-site-images.mjs` and their tests. No protected surface.

## Constraints

- The vision's two rules. An AI agent's reading of Spec Kit's source is not
  evidence: the page marks it Not yet reached. Upstream triage and merged fixes
  are the maintainers' decisions, reported as such, not as Crosscheck's
  evidence.
- The method comes from the transcript of the session that found the
  defects (2026-10-22), which is not public: CGV first, with no real bug among
  its 39 errors, then eight `/reason` subagents whose findings counted only when
  a reproduction showed them. The pages do not link the transcript.
- Harry confirmed on 2026-11-11 that they re-ran every reproduction before
  filing; the macOS output in each report is that run.
- A security finding from the same session was emailed to GitHub's security
  team, with no reply by 2026-11-11. The pages say only that it exists and was
  reported privately, and nothing about its subject.
- This session could not run Spec Kit's code (the auto-mode classifier refused
  third-party code), so the pages report the recorded outputs and say that the
  website did not rerun them.
- Old EVerest links keep working, fragment included.

## Open questions

- Should the reproductions be re-run on Spec Kit's main branch, in a sandbox
  like the Dafny and Lean containers, so each report's present state on main
  becomes Tested rather than Not yet reached?
- A differential test of the evaluator needs a reference. Spec Kit now
  documents departures from Jinja2 (left-to-right comparison grouping,
  numeric-string equality), so which behaviours should a reference model keep?
