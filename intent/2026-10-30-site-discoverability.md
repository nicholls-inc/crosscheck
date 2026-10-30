# Make the study site findable, shareable and readable by agents

## Intent

The user asked on 2026-10-30 for everything the EVerest study site needs before
it is deployed: a favicon, a page title in the browser tab, link previews with
an image on platforms that unfurl links, search engine indexing, and content
that AI crawlers and agents can read. The pages people see in a browser must
not change.

## Outcome and constraints

- Every page names an absolute canonical URL, Open Graph and X card tags, a
  favicon set and schema.org JSON-LD. The study page also links its JSON
  publication record as an alternate.
- `robots.txt`, `sitemap.xml` and `llms.txt` are generated from the same
  record and pages, with the same origin.
- One constant in `scripts/build-site.mjs` holds the public origin. Absolute
  URLs come only from it.
- The favicon is the existing Counterproof mark. The preview cards use the
  site's own fonts and palette, rendered by a script that can be rerun.
- No rendered pixel changes. Only `<head>` gains tags, and a script proves the
  page renders identically with and without them.
- `scripts/check-public-site.mjs` fails if the deployed pages name a different
  origin than the one being checked, so a wrong origin cannot ship unnoticed.

## Open decisions

The public hostname is still the user's choice. The origin constant starts as
`https://crosscheck.harrynicholls.com/`, a guess under the domain on the
owner's GitHub profile. Changing it is one edit and one rebuild.

Tier 1 by its own criteria would be wrong: the site's output changes. Tier 2;
no protected surfaces.
