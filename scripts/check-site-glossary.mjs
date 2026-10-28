#!/usr/bin/env node
// check-site-glossary.mjs checks that the help-tip text on the landing page
// matches the glossary page word for word, and that every term on the landing
// page has a definition.
//
//   node scripts/check-site-glossary.mjs [site-dir]
//
// Exit codes: 0 they match, 1 they differ (one line per problem on stdout),
// 2 a page cannot be read.

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? 'site';

function read(name) {
  try {
    return readFileSync(join(dir, name), 'utf8');
  } catch (err) {
    console.error(`cannot read ${join(dir, name)}: ${err.message}`);
    process.exit(2);
  }
}

// id -> text of every element whose id starts with def-.
function definitions(html) {
  const found = new Map();
  for (const [, id, text] of html.matchAll(/<(?:p|dd) id="(def-[a-z-]+)">([\s\S]*?)<\/(?:p|dd)>/g)) {
    found.set(id, text.trim());
  }
  return found;
}

const index = read('index.html');
const tips = definitions(index);
const glossary = definitions(read('glossary.html'));
const problems = [];

for (const [id, text] of glossary) {
  if (!tips.has(id)) problems.push(`${id}: in glossary.html, missing from the tips in index.html`);
  else if (tips.get(id) !== text) problems.push(`${id}: tip text in index.html differs from glossary.html`);
}
for (const id of tips.keys()) {
  if (!glossary.has(id)) problems.push(`${id}: a tip in index.html, missing from glossary.html`);
}
for (const [, term] of index.matchAll(/data-term="([a-z-]+)"/g)) {
  const id = `def-${term}`;
  if (!glossary.has(id) && !problems.some((p) => p.startsWith(`${id}:`))) {
    problems.push(`${id}: used by a term in index.html, defined nowhere`);
  }
}

for (const p of problems) console.log(p);
process.exit(problems.length ? 1 : 0);
