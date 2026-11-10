// Render publication fragments into checked-in HTML. No browser fetch is needed.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname, relative } from 'node:path';
import { specKitFragments, specKitCounts } from './site-spec-kit.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const record = JSON.parse(readFileSync(resolve(root, 'site/studies/everest-ocpp/everest-study.json'), 'utf8'));
const esc = (value) => String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
// Prose gets typographic quotes and apostrophes before escaping; commands, paths and attributes use esc() alone.
const prose = (value) => esc(String(value).replace(/"([^"]*)"/g, '\u201C$1\u201D').replace(/'/g, '\u2019'));
const number = (value) => value.toLocaleString('en-GB');
const source = (key, label) => `<a href="${record.repository}/blob/${record.studyCommit}/${record.sources[key]}" target="_blank" rel="noopener">${prose(label)}<span class="visually-hidden"> (opens in a new tab)</span></a>`;
const list = (items) => `<ul class="study-list">${items.map((item) => `<li>${prose(item)}</li>`).join('')}</ul>`;

function claims() {
  return `<table class="ledger"><caption>What the evidence says</caption><thead><tr><th scope="col">What was checked</th><th scope="col">How it was checked</th><th scope="col">Strength</th><th scope="col">Result and boundary</th></tr></thead><tbody>${record.claims.map((c) => `<tr><td data-label="What was checked">${prose(c.statement)}</td><td data-label="How it was checked">${prose(c.support)}</td><td class="strength" data-label="Strength">${esc(c.strength)}</td><td data-label="Result and boundary">${prose(c.boundary)}</td></tr>`).join('')}</tbody></table>
<p class="evidence-scope">Proofs cover the reference model. Tests compare EVerest with it on the inputs tried. The study reports these results; this website did not rerun them.</p>`;
}

function witness() {
  const w = record.witness;
  return `<figure class="witness"><figcaption>One daily default schedule, first started at <time datetime="${w.anchor}">11:00:00 UTC on 14 September 2016</time>.
<span class="witness-time">Read at <time datetime="${w.readAt}">10:59:59 UTC on 13 August 2026</time></span></figcaption><dl>
<div><dt>The reference model</dt><dd><span class="reading">${number(w.modelWatts)}&nbsp;W</span><span class="why">The second period still applies. Restart is at ${esc(w.expectedRestart)}.</span></dd></div>
<div><dt>EVerest at <code>${record.everestCommit.slice(0, 8)}</code></dt><dd><span class="reading reading-off">${number(w.everestWatts)}&nbsp;W</span><span class="why">It has already restarted, one second early, so the limit is too high.</span></dd></div>
</dl></figure>`;
}

function results() {
  return `<table class="ledger ledger-numeric"><caption>Reported case totals</caption><thead><tr><th scope="col">Configuration</th><th scope="col">Pass</th><th scope="col">Warn</th><th scope="col">Fail</th></tr></thead><tbody>${record.results.map((r) => `<tr><td data-label="Configuration">${prose(r.configuration)}</td>${['pass', 'warn', 'fail'].map((key) => `<td data-label="${key[0].toUpperCase() + key.slice(1)}">${number(r[key])}</td>`).join('')}</tr>`).join('')}</tbody></table>
<p class="evidence-note">Seed ${record.seed}; ${number(record.results[0].cases)} cases per main configuration. The mutation self-test uses 300 cases and seed 1. Exact means within ${prose(record.tolerance)}.</p>
<p class="evidence-note">${prose(record.classificationCaveat)}</p>`;
}

function counters() {
  const labels = {instants: 'Instants compared', modelHasLimit: 'Model gives a limit', exact: 'EVerest agrees within <button type="button" class="term" data-term="tolerance" aria-describedby="def-tolerance">tolerance</button>', below: 'EVerest below the model', aboveOrMissing: 'EVerest above, or missing a required limit', extraLimit: 'EVerest gives a limit; model gives none', neither: 'Neither gives a limit', vacuousCases: 'Cases with no model limit anywhere'};
  return `<table class="ledger ledger-numeric"><caption>Agreement at individual instants</caption><thead><tr><th scope="col">Counter</th><th scope="col">2017–2025 anchors</th><th scope="col">2010–2016 anchors</th></tr></thead><tbody>${Object.entries(labels).map(([key, label]) => `<tr><td data-label="Counter">${label}</td><td data-label="2017–2025 anchors">${number(record.counters.recent[key])}</td><td data-label="2010–2016 anchors">${number(record.counters.older[key])}</td></tr>`).join('')}</tbody></table>`;
}

function replay() {
  return `<p>Install elan with <code>${esc(record.environment.lean)}</code>, Python 3, git and curl. The recorded driver build used:</p>
${list([record.environment.compiler, `Boost ${record.environment.boost}`, `fmt ${record.environment.fmt}`])}
<p>See ${source('driver', 'the driver prerequisites')} before running.</p>
<p class="evidence-note">${prose(record.environment.boundary)} The leap-second table must list the three leap seconds since 2010. The driver build checks downloaded sources against SHA-256 hashes in ${source('build', 'the pinned build script')}.</p>
<div class="commands"><pre id="replay-commands" tabindex="0" aria-label="Commands to repeat the study"><code>${record.commands.map((c) => `<span class="cmd">${esc(c).replace(/\//g, '/<wbr>').replace(/(--[a-z-]+)/g, '<span class="flag">$1</span>').replace(/\b([a-f0-9]{40})\b/g, '<span class="hash">$1</span>')}</span>`).join('\n')}</code></pre><button type="button" class="copy" data-copy="replay-commands">Copy replay commands</button><span class="copy-status" role="status" aria-live="polite"></span></div>
<p class="evidence-note">${prose(record.replayNote)}</p>`;
}

function limits() {
  return `<p>${prose(record.coverage)}</p><p class="evidence-note">${prose(record.ambiguity)}</p><table class="ledger"><caption>Not yet reached by this study</caption><thead><tr><th scope="col">Behavior</th><th scope="col">Blocking property</th><th scope="col">Open modeling question</th></tr></thead><tbody>${record.notYetReached.map((r) => `<tr><td data-label="Behavior">${prose(r.behavior)}</td><td data-label="Blocking property">${prose(r.blocker)}</td><td data-label="Open modeling question">${prose(r.question)}</td></tr>`).join('')}</tbody></table>`;
}

function sources() {
  return `<p>Study snapshot: <code class="revision">${record.studyCommit}</code>. EVerest source: <a href="https://github.com/EVerest/EVerest/tree/${record.everestCommit}" target="_blank" rel="noopener"><code>${record.everestCommit.slice(0, 8)}</code><span class="visually-hidden"> (opens in a new tab)</span></a>.</p>
<ul class="source-links">${Object.entries({spec: 'Written rules', model: 'Model and proofs', review: 'Correspondence review', report: 'Test report', driver: 'Driver and shims', build: 'Build inputs', harness: 'Test harness', proofCheck: 'Proof check'}).map(([key, label]) => `<li>${source(key, label)}</li>`).join('')}</ul>
<p class="evidence-note">${prose(record.status)}. Main runs dated ${record.runDate}; comparison counters measured ${record.measurementDate}.</p>
<p class="evidence-note">${prose(record.publicationNote)}</p>`;
}

function contribution() {
  const c = record.contribution;
  return `<figure class="quote"><blockquote><p>${prose(c.summary)}</p><p>${prose(c.human)}</p></blockquote><figcaption>${esc(record.author.name.split(' ')[0])}</figcaption></figure>
<dl class="contribution"><div><dt>AI assistance</dt><dd>${prose(c.agents)}</dd></div>
<div><dt>What the checks establish</dt><dd>${prose(c.checks)}</dd></div>
<div><dt>The reported result</dt><dd>${prose(c.result)}</dd></div></dl>
<p class="evidence-note">The ${source('spec', 'signed interpretation decisions')} and ${source('review', 'correspondence review with Harry’s decisions')} record the human judgments behind this account.</p>`;
}

function choices() {
  return `<dl class="contribution">${record.engineeringDecisions.map((d) => `<div><dt>${prose(d.choice)}</dt><dd>${prose(d.reason)} <span class="decision-boundary">${prose(d.boundary)}</span></dd></div>`).join('')}</dl>`;
}


const specKit = JSON.parse(readFileSync(resolve(root, 'site/studies/spec-kit/spec-kit-study.json'), 'utf8'));
const specKitParts = specKitFragments(specKit);
const kit = specKitCounts(specKit);

// The study index on the landing page. Each entry's one line is a fact from its record.
const studies = [
  { path: 'studies/everest-ocpp/', title: 'EVerest and OCPP 1.6: a formal study', kind: 'Lean reference model and differential tests',
    line: `EVerest restarts a daily charging schedule one second early. Three rules of a Lean model are proved, and 2,000 generated cases per configuration compare EVerest with it.`, date: record.runDate },
  { path: 'studies/spec-kit/', title: 'Spec Kit: 28 reported defects', kind: 'Reproduced reports and their upstream outcomes',
    line: `${kit.total} defects in GitHub’s Spec Kit, each with a reproduction script and the line of Spec Kit’s own documentation it contradicts. ${kit.merged} have a merged fix.`, date: specKit.filedOn },
  { path: 'studies/spec-kit-expressions/', title: 'Spec Kit’s workflow expressions', kind: 'A closer look at three of the reports',
    line: 'The evaluator called itself a safe Jinja2 subset, but answered some comparisons differently from Jinja2. What the reports showed, and what changed upstream.', date: specKit.filedOn },
];

const fragments = {claims, witness, results, counters, replay, limits, sources, contribution, choices,
  properties: () => list(record.modelProperties) + `<p class="evidence-note">Not yet reached: ${prose(record.unfinishedProperty)}</p>`,
  decisions: () => list(record.humanDecisions),
  trust: () => list(record.trustedDependencies),
  upstream: () => `<p>${prose(record.upstreamStatus)} <a href="${esc(record.upstreamIssueUrl)}" target="_blank" rel="noopener">View filed issue<span class="visually-hidden"> (opens in a new tab)</span></a></p>`,
  studies: () => `<ol class="study-index">${studies.map((s) => `<li><a class="study-index-title" href="${s.path}">${prose(s.title)}</a><p class="study-index-kind">${prose(s.kind)} · <time datetime="${s.date}">${s.date}</time></p><p>${prose(s.line)}</p></li>`).join('')}</ol>`,
  ...specKitParts,
};

// The public origin. Every absolute URL the site publishes is built from it, and check-public-site.mjs refuses a deployment that names another.
const origin = 'https://crosscheck.harrynicholls.com/';
const x = 'harrynicholls';
// Pages by their path inside site/. A forwarding page shares its target's metadata, so a preview of an old link matches where it lands.
const everest = 'studies/everest-ocpp/index.html';
const pages = {
  'index.html': { path: '', type: 'website', kind: 'home', card: 'cards/home.png', cardAlt: 'Crosscheck: studies of code checked with deterministic evidence.' },
  [everest]: { path: 'studies/everest-ocpp/', type: 'article', kind: 'everest', record: 'everest-study.json', card: 'cards/study.png', cardAlt: 'Crosscheck: EVerest and OCPP 1.6, a formal study. EVerest restarts a daily charging schedule one second early.' },
  'studies/everest-ocpp/glossary.html': { path: 'studies/everest-ocpp/glossary.html', type: 'website', kind: 'glossary', card: 'cards/glossary.png', cardAlt: 'Crosscheck: the glossary of the EVerest and OCPP 1.6 study.' },
  'studies/spec-kit/index.html': { path: 'studies/spec-kit/', type: 'article', kind: 'spec-kit', record: 'spec-kit-study.json', card: 'cards/spec-kit.png', cardAlt: `Crosscheck: Spec Kit, 28 reported defects. ${kit.merged} have a merged fix.` },
  'studies/spec-kit-expressions/index.html': { path: 'studies/spec-kit-expressions/', type: 'article', kind: 'spec-kit', record: '../spec-kit/spec-kit-study.json', card: 'cards/spec-kit-expressions.png', cardAlt: 'Crosscheck: Spec Kit’s workflow expressions. Comparisons that answered differently from Jinja2.' },
  'everest.html': { sameAs: everest },
  'glossary.html': { sameAs: 'studies/everest-ocpp/glossary.html' },
};
const read = (name) => readFileSync(resolve(root, 'site', name), 'utf8');
const unesc = (value) => value.replace(/&(amp|lt|gt|quot|#39);/g, (_, e) => ({ amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'" })[e]);
const text = (html) => unesc(html.replace(/<[^>]+>/g, '')).trim();
const describe = (name) => {
  const html = read(name);
  return { title: text(html.match(/<title>([\s\S]*?)<\/title>/)[1]), description: unesc(html.match(/<meta name="description" content="([^"]*)">/)[1]) };
};
const glossaryTerms = () => [...read('studies/everest-ocpp/glossary.html').matchAll(/<div id="([a-z-]+)"><dt><dfn>([\s\S]*?)<\/dfn><\/dt><dd id="def-\1">([\s\S]*?)<\/dd>/g)]
  .map(([, id, name, definition]) => ({ id, name: text(name), definition: text(definition) }));
const json = (value) => JSON.stringify(value).replace(/</g, '\\u003c');
// The relative path from a page to the site root, so shared icons resolve at any depth.
const up = (name) => relative(dirname(resolve(root, 'site', name)), resolve(root, 'site')).replace(/\\/g, '/');
const toRoot = (name) => (up(name) ? `${up(name)}/` : '');

function structuredData(name, url, title, description) {
  const { kind, card } = pages[name];
  const website = { '@type': 'WebSite', '@id': `${origin}#website`, url: origin, name: 'Crosscheck' };
  const person = { '@type': 'Person', '@id': `${origin}#author`, name: record.author.name, url: record.author.profile, sameAs: [`https://x.com/${x}`, record.author.profile] };
  const article = (extra) => ({ '@type': 'TechArticle', headline: title, author: { '@id': person['@id'] }, image: origin + card, ...extra });
  const page = {
    home: { '@type': 'CollectionPage', hasPart: studies.map((s) => ({ '@type': 'TechArticle', headline: s.title, url: origin + s.path })) },
    everest: article({ citation: record.essay, isBasedOn: `${record.repository}/tree/${record.studyCommit}`, about: { '@type': 'SoftwareApplication', name: 'EVerest', url: 'https://github.com/EVerest/EVerest' } }),
    'spec-kit': article({ isBasedOn: `${specKit.subject.repository}/tree/${specKit.filingCommit}`, about: { '@type': 'SoftwareApplication', name: specKit.subject.name, url: specKit.subject.repository } }),
    glossary: { '@type': 'DefinedTermSet', hasDefinedTerm: glossaryTerms().map((t) => ({ '@type': 'DefinedTerm', name: t.name, description: t.definition, url: `${url}#${t.id}` })) },
  }[kind];
  return { '@context': 'https://schema.org', '@graph': [website, person, { ...page, '@id': `${url}#page`, url, name: title, description, inLanguage: 'en', isPartOf: { '@id': website['@id'] } }] };
}

function head(name) {
  const own = pages[name].sameAs ?? name;
  const { path, type, card, cardAlt, record: data } = pages[own];
  const { title, description } = describe(own);
  const url = origin + path;
  const base = toRoot(name);
  const dataHref = data && relative(dirname(resolve(root, 'site', name)), resolve(root, 'site', dirname(own), data)).replace(/\\/g, '/');
  return [
    `<link rel="canonical" href="${url}">`,
    `<link rel="icon" href="${base}favicon.ico" sizes="32x32">`,
    `<link rel="icon" href="${base}favicon.svg" type="image/svg+xml">`,
    `<link rel="apple-touch-icon" href="${base}apple-touch-icon.png">`,
    ...(data && !pages[name].sameAs ? [`<link rel="alternate" type="application/json" href="${dataHref}" title="Publication record">`] : []),
    `<meta name="author" content="${esc(record.author.name)}">`,
    `<meta property="og:type" content="${type}">`,
    '<meta property="og:site_name" content="Crosscheck">',
    `<meta property="og:title" content="${esc(title)}">`,
    `<meta property="og:description" content="${esc(description)}">`,
    `<meta property="og:url" content="${url}">`,
    `<meta property="og:image" content="${origin}${card}">`,
    '<meta property="og:image:width" content="1200">',
    '<meta property="og:image:height" content="630">',
    `<meta property="og:image:alt" content="${esc(cardAlt)}">`,
    '<meta name="twitter:card" content="summary_large_image">',
    `<meta name="twitter:creator" content="@${x}">`,
    ...(pages[name].sameAs ? [] : [`<script type="application/ld+json">${json(structuredData(name, url, title, description))}</script>`]),
  ].join('\n');
}

const sourceLabels = {spec: 'Written rules', model: 'Model and proofs', review: 'Correspondence review', report: 'Test report', driver: 'Driver and shims', build: 'Build inputs', harness: 'Test harness', proofCheck: 'Proof check'};
const listed = Object.entries(pages).filter(([, page]) => !page.sameAs);

function llms() {
  const home = describe('index.html');
  const study = describe(everest);
  const glossary = describe('studies/everest-ocpp/glossary.html');
  const kitPage = describe('studies/spec-kit/index.html');
  const exprPage = describe('studies/spec-kit-expressions/index.html');
  return `# Crosscheck studies

> ${home.description}

Every page is plain HTML that reads in full without script. Each study marks its evidence as Proved, Tested, Decided by Harry, Assumed or Not yet reached, and only deterministic checks and human decisions count as evidence.

## ${study.title}

> ${study.description}

${record.status}. ${record.publicationNote}

What the evidence says, by strength:

${record.claims.map((c) => `- ${c.strength}: ${c.statement}. ${c.support}. ${c.boundary}`).join('\n')}

${record.upstreamStatus}

## ${kitPage.title}

> ${kitPage.description}

${specKit.status}. ${kit.total} reports filed on ${specKit.filedOn} against Spec Kit ${specKit.subject.version} at ${specKit.filingCommit}. By ${specKit.checkedOn}: ${kit.merged} with a merged fix, ${kit.openFix} with a fix open, ${kit.noFix} with neither; ${kit.closed} closed by the maintainers.

## ${exprPage.title}

> ${exprPage.description}

## Pages

- [${home.title}](${origin}): ${home.description}
- [${study.title}](${origin}${pages[everest].path}): ${study.description}
- [${glossary.title}](${origin}${pages['studies/everest-ocpp/glossary.html'].path}): ${glossary.description}
- [${kitPage.title}](${origin}studies/spec-kit/): ${kitPage.description}
- [${exprPage.title}](${origin}studies/spec-kit-expressions/): ${exprPage.description}

## Data

- [EVerest publication record (JSON)](${origin}studies/everest-ocpp/everest-study.json): The editorial snapshot the EVerest study page is built from: claims, test totals, agreement counters, the witness case, limits and replay commands.
- [Spec Kit publication record (JSON)](${origin}studies/spec-kit/spec-kit-study.json): The editorial snapshot both Spec Kit pages are built from: each report, what it contradicts, its upstream state and fixes, and the limits.

## Sources

- [Crosscheck repository](${record.repository}): The tool behind the studies.
- [Review Evidence, Not Code](${record.essay}): The essay the EVerest study is a worked example of.
- [EVerest study pull request](${record.pullRequest}): The change that added the EVerest study.
- [EVerest issue #3323](${record.upstreamIssueUrl}): The EVerest finding, reported upstream.
${Object.entries(sourceLabels).map(([key, label]) => `- [${label}](${record.repository}/blob/${record.studyCommit}/${record.sources[key]}): Pinned at the EVerest study commit.`).join('\n')}
- [Spec Kit issues #${specKit.issues[0].number} to #${specKit.issues.at(-1).number}](${specKit.subject.repository}/issues?q=is%3Aissue+author%3A${specKit.reporter}): The Spec Kit reports.

## Glossary of the EVerest study

${glossaryTerms().map((t) => `- [${t.name}](${origin}${pages['studies/everest-ocpp/glossary.html'].path}#${t.id}): ${t.definition}`).join('\n')}
`;
}

const files = {
  'robots.txt': () => `User-agent: *\nAllow: /\n\nSitemap: ${origin}sitemap.xml\n`,
  'sitemap.xml': () => `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${listed.map(([, page]) => `  <url><loc>${origin}${page.path}</loc></url>`).join('\n')}\n</urlset>\n`,
  'llms.txt': llms,
};

let stale = false;
const update = (name, before, after, what) => {
  if (before === after) return;
  if (process.argv.includes('--check')) { console.error(`${name}: ${what} need regeneration`); stale = true; }
  else writeFileSync(resolve(root, 'site', name), after);
};
for (const name of Object.keys(pages)) {
  const before = read(name);
  update(name, before, before.replace(/<!-- publication:([a-z-]+) -->[\s\S]*?<!-- \/publication:\1 -->/g,
    (_, key) => `<!-- publication:${key} -->\n${key === 'head' ? head(name) : fragments[key]()}\n<!-- /publication:${key} -->`), 'publication fragments');
}
for (const [name, render] of Object.entries(files)) {
  let before = '';
  try { before = read(name); } catch {}
  update(name, before, render(), 'generated contents');
}
if (stale) process.exitCode = 1;
