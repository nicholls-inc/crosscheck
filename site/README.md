# Crosscheck website

A static website about one study: EVerest's OCPP 1.6 charging schedules,
checked with Crosscheck. It is the worked example for the published
[Review Evidence, Not Code](https://brainflow.substack.com/p/review-evidence-not-code)
essay. `index.html` is the study; `everest.html` forwards old links to it.

Serve this directory with any static server, for example from the repository root:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory "$PWD/site"
```

Open `http://127.0.0.1:8765`. If that port already serves another worktree, stop
that preview first and run the command from this worktree's repository root.
Each worktree has its own `site/`; rebuilding one does not update a server rooted
in another. No build step, browser API key, analytics, external
font service, package installation or client-side framework is required.

## Updating the publication

`everest-study.json` is the editorial publication snapshot and downloadable
record. It is deliberately not the verifier's `evidence-record/1` format. It
summarizes historical evidence; generating the website does not rerun any proof
or experiment. The article discloses the pinned report's legacy classification
note alongside its newer warning rule.

Edit the record, then render the shared HTML fragments:

```sh
node scripts/build-site.mjs
node scripts/build-site.mjs --check
node scripts/check-site.mjs
node --test scripts/site.test.mjs
node --test scripts/check-public-site.test.mjs
```

To check source paths, reported totals and download hashes against the original
study, when that commit is available in the local Git object store:

```sh
node scripts/check-site.mjs --verify-sources
```

The build also writes each page's head metadata (canonical URL, preview tags,
icons and JSON-LD), `robots.txt`, `sitemap.xml` and `llms.txt` from the record,
the page titles and descriptions, and the glossary. The icons and preview cards
in `cards/` are drawn by `node scripts/render-site-images.mjs`, which needs
ImageMagick and Google Chrome; rerun it after changing the mark, the study
title or the palette.

The checks fail on stale generated fragments, broken local links, mismatched
study revisions, inconsistent counters, missing assets or divergent glossary
help. HTML is checked in and fully readable without JavaScript. `help.js` adds
term explanations, one finding highlight and copy controls.

When advancing the study, choose one full immutable commit. Check its spec,
model, report, human decisions, harness and driver prerequisites together.
Update all claims and limitations in the record before regenerating both
surfaces. Keep model proofs distinct from tests of production code. Changing
the warning rule can change totals without changing the underlying divergence.

The launch kit prepares an origin configuration. It does not deploy remotely,
edit the published essay, publish externally or file the upstream issue.

For the confirmed home-server deployment through an existing Cloudflare Tunnel,
see [LAUNCH.md](LAUNCH.md). The launch kit includes a localhost-only Caddy config
and a public-route check; building it does not change the live tunnel.

## Provenance

The initial HTML, CSS, glossary, interaction script and three local font files
were copied from the owner's existing `landing-page/site/` worktree on
2026-10-28, then extended here. The other worktree was not modified.

The site retains that implementation's Newsreader, Public Sans and IBM Plex
Mono typography, warm paper background, ruled evidence tables, specialist-term
help, responsive source order and installation instructions. See
[JOURNAL.md](JOURNAL.md) for the change record.
