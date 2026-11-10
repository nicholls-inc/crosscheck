// Publication and navigation checks; no dependencies or network access.
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { specKitCounts } from './site-spec-kit.mjs';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const site = resolve(root, 'site');
const everestDir = resolve(site, 'studies/everest-ocpp');
const record = JSON.parse(readFileSync(resolve(everestDir, 'everest-study.json'), 'utf8'));
const specKit = JSON.parse(readFileSync(resolve(site, 'studies/spec-kit/spec-kit-study.json'), 'utf8'));
// Every page and generated file in the release, by its path inside site/. Maintenance notes are not published.
const walk = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => e.isDirectory() ? walk(resolve(dir, e.name)) : [relative(site, resolve(dir, e.name))]);
const published = walk(site).filter((name) => !/(^|\/)(README|JOURNAL|LAUNCH)\.md$/.test(name));
const htmlPages = published.filter((name) => name.endsWith('.html'));
// A link to a directory serves its index.html.
const fileFor = (path) => (path.endsWith('/') || existsSync(path) && statSync(path).isDirectory() ? resolve(path, 'index.html') : path);
const problems = [];
const requireThat = (condition, message) => { if (!condition) problems.push(message); };
const definitions = (html) => new Map([...html.matchAll(/<(?:p|dd) id="(def-[a-z-]+)">([\s\S]*?)<\/(?:p|dd)>/g)].map((m) => [m[1], m[2].trim()]));

requireThat(record.format === 'crosscheck-publication-snapshot/1', 'record must be an editorial publication snapshot');
requireThat(/^[a-f0-9]{40}$/.test(record.studyCommit), 'study revision must be a full immutable commit');
requireThat(/^[a-f0-9]{40}$/.test(record.everestCommit), 'EVerest revision must be a full immutable commit');
requireThat(record.commands.includes(`git checkout --detach ${record.studyCommit}`), 'replay must check out the publication revision');
requireThat(record.classificationCaveat.includes('1,081'), 'retain the extra-limit classification caveat');
const STRENGTHS = ['Proved', 'Tested', 'Decided by Harry', 'Assumed', 'Not yet reached'];
for (const c of record.claims) requireThat(STRENGTHS.includes(c.strength), `claim strength must be one of the five strengths: ${c.strength}`);
for (const r of record.results) {
  requireThat([r.pass, r.warn, r.fail].every(Number.isSafeInteger), `${r.configuration}: counts must be integers`);
  requireThat(r.pass + r.warn + r.fail === r.cases, `${r.configuration}: totals must equal case count`);
}
for (const c of Object.values(record.counters)) {
  requireThat(c.exact + c.below + c.aboveOrMissing === c.modelHasLimit, 'limited-instant counters must balance');
  requireThat(c.modelHasLimit + c.extraLimit + c.neither === c.instants, 'all instant counters must balance');
}

// The Spec Kit record: pinned commits, one entry per report, and the counts the hand-written answers repeat.
requireThat(specKit.format === 'crosscheck-publication-snapshot/1', 'Spec Kit record must be an editorial publication snapshot');
for (const key of ['filingCommit', 'mainCommit']) requireThat(/^[a-f0-9]{40}$/.test(specKit[key]), `Spec Kit ${key} must be a full immutable commit`);
const numbers = specKit.issues.map((i) => i.number);
requireThat(new Set(numbers).size === numbers.length, 'Spec Kit record: duplicate report numbers');
for (const i of specKit.issues) {
  requireThat(Object.hasOwn(specKit.areas, i.area), `#${i.number}: unknown area ${i.area}`);
  requireThat(Object.hasOwn(specKit.triageLabels, i.triage), `#${i.number}: unknown triage label ${i.triage}`);
  requireThat(['open', 'closed'].includes(i.state) && (i.state === 'closed') === Boolean(i.closed), `#${i.number}: state and closing date disagree`);
  for (const f of i.fixes) requireThat(['merged', 'open', 'closed'].includes(f.state) && (f.state === 'merged') === Boolean(f.date), `#${i.number}: fix #${f.number} state and merge date disagree`);
}
for (const n of specKit.expressions.issues) requireThat(numbers.includes(n), `expressions study names #${n}, which the record does not hold`);
const kit = specKitCounts(specKit);
const said = {
  'studies/spec-kit/index.html': [`${kit.merged} now have a merged fix`, `${kit.merged} reports had a merged fix and ${kit.openFix} had a fix open`, `${kit.closed} issues are closed`,
    ...Object.entries(kit.triage).map(([label, n]) => `${n} as “${label.replace('triage-', '').replace(/-/g, ' ')}”`), `${kit.total} defect reports`],
  'index.html': [`${kit.merged} have a merged fix`],
};
for (const [name, phrases] of Object.entries(said)) {
  const page = readFileSync(resolve(site, name), 'utf8').replace(/<[^>]+>/g, '');
  for (const phrase of phrases) requireThat(page.includes(phrase), `${name}: hand-written count no longer matches the record: expected "${phrase}"`);
}

const glossaryPage = resolve(everestDir, 'glossary.html');
const glossary = definitions(readFileSync(glossaryPage, 'utf8'));
requireThat(glossary.size > 0, 'glossary must contain definitions');
for (const name of htmlPages) {
  const path = resolve(site, name);
  const html = readFileSync(path, 'utf8');
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]);
  requireThat(new Set(ids).size === ids.length, `${name}: duplicate HTML ids`);
  for (const [, value] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (/^https?:\/\//.test(value)) {
      const url = new URL(value);
      if (url.hostname === 'github.com' && url.pathname.startsWith('/nicholls-inc/crosscheck/blob/')) {
        const [, , , , revision, ...parts] = url.pathname.split('/');
        const artifact = parts.join('/');
        if (artifact.startsWith('formal-verification/')) {
          requireThat(revision === record.studyCommit, `${name}: moving or mismatched study source: ${value}`);
          requireThat(Object.values(record.sources).includes(artifact), `${name}: unlisted study artifact: ${artifact}`);
        }
      }
      continue;
    }
    const [local, anchor] = value.split('#');
    const file = local ? fileFor(resolve(dirname(path), local)) : path;
    requireThat(!relative(site, file).startsWith('..'), `${name}: link escapes site: ${value}`);
    requireThat(existsSync(file), `${name}: missing local target: ${value}`);
    if (anchor && existsSync(file)) {
      const content = readFileSync(file, 'utf8');
      requireThat(content.includes(`id="${anchor}"`), `${name}: missing anchor: ${value}`);
    }
  }
  // The glossary is the source of the definitions; a forwarding page has no content to check. Only pages that explain terms carry the definitions.
  if (path === glossaryPage || html.includes('http-equiv="refresh"') || !html.includes('id="definitions"')) continue;
  const tips = definitions(html);
  requireThat(tips.size === glossary.size, `${name}: glossary definition count differs`);
  for (const [id, text] of glossary) requireThat(tips.get(id) === text, `${name}: definition ${id} differs from glossary`);
  for (const [, term] of html.matchAll(/data-term="([a-z-]+)"/g)) requireThat(tips.has(`def-${term}`), `${name}: undefined term ${term}`);
  for (const [, target] of html.matchAll(/data-copy="([^"]+)"/g)) requireThat(ids.includes(target), `${name}: missing copy target ${target}`);
}
// Without script the answers are the only copy of each answer a reader sees, so each one must sit under the question it answers.
const studyPages = htmlPages.filter((name) => readFileSync(resolve(site, name), 'utf8').includes('id="questions"'));
requireThat(studyPages.length >= 3, 'expected a question filter on every study page');
for (const name of studyPages) {
  const page = readFileSync(resolve(site, name), 'utf8');
  const questions = new Map([...page.matchAll(/<a href="#q-([a-z-]+)" data-question="\1"><span>(.+?)<\/span><\/a>/g)].map((m) => [m[1], m[2].replace(/<[^>]+>/g, '')]));
  const inlineQuestions = [...page.matchAll(/<p class="answer-inline-q">([^\n]*?)<\/p>\s*<div data-answer="([a-z-]+)">/g)].map((m) => ({ key: m[2], text: m[1].replace(/<[^>]+>/g, '') }));
  const answerKeys = [...page.matchAll(/data-answer="([a-z-]+)"/g)].map((m) => m[1]);
  requireThat(questions.size > 0, `${name}: no questions found`);
  for (const [key, text] of questions) {
    requireThat(answerKeys.filter((k) => k === key).length === 1, `${name}: question ${key} must have exactly one answer`);
    const inline = inlineQuestions.filter((q) => q.key === key);
    requireThat(inline.length === 1 && inline[0].text === text, `${name}: the question above the ${key} answer must read "${text}"`);
  }
  for (const key of answerKeys) requireThat(questions.has(key), `${name}: answer ${key} has no question`);
  for (const [, strengths] of page.matchAll(/data-strength="([a-z ]+)"/g)) for (const s of strengths.split(' ')) requireThat(['proved', 'tested', 'decided', 'assumed', 'unreached'].includes(s), `${name}: unknown strength ${s}`);
}

// Absolute URLs into the site's own origin must resolve inside this release, and preview cards must be the size the pages declare.
const index = readFileSync(resolve(site, 'index.html'), 'utf8');
const origin = index.match(/<link rel="canonical" href="(https:\/\/[^"]+\/)">/)?.[1];
requireThat(origin, 'index.html: needs an absolute canonical URL ending in /');
for (const name of published.filter((name) => /\.(html|txt|xml)$/.test(name))) {
  const content = readFileSync(resolve(site, name), 'utf8');
  for (const [url] of origin ? content.matchAll(new RegExp(`${origin.replace(/[.]/g, '\\.')}[^"\\s)<#]*`, 'g')) : []) {
    const local = url.slice(origin.length);
    requireThat(existsSync(fileFor(resolve(site, local || './'))), `${name}: ${url} has no file in this release`);
  }
  if (!name.endsWith('.html')) continue;
  const card = content.match(/<meta property="og:image" content="([^"]+)">/)?.[1];
  requireThat(card?.startsWith(origin), `${name}: needs an og:image on the site's origin`);
  if (card?.startsWith(origin) && existsSync(resolve(site, card.slice(origin.length)))) {
    const png = readFileSync(resolve(site, card.slice(origin.length)));
    const declared = ['width', 'height'].map((side) => Number(content.match(new RegExp(`<meta property="og:image:${side}" content="(\\d+)">`))?.[1]));
    requireThat(png.readUInt32BE(16) === declared[0] && png.readUInt32BE(20) === declared[1], `${name}: ${card} is not the declared ${declared.join('x')}`);
  }
}

const css = readFileSync(resolve(site, 'styles.css'), 'utf8');
for (const [, asset] of css.matchAll(/url\("([^"]+)"\)/g)) requireThat(existsSync(resolve(site, asset)), `missing stylesheet asset: ${asset}`);

if (process.argv.includes('--verify-sources')) {
  // Cross-check editorial totals and input hashes against the immutable Git objects.
  const at = (key) => execFileSync('git', ['show', `${record.studyCommit}:${record.sources[key]}`], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  const report = at('report');
  for (const r of record.results) {
    requireThat(report.includes(`| ${r.pass} | ${r.warn} | ${r.fail} |`), `${r.configuration}: counts differ from pinned report`);
  }
  for (const hash of Object.values(record.environment.buildDownloadHashes)) requireThat(at('build').includes(hash), 'download hash differs from pinned build');
  for (const key of Object.keys(record.sources)) requireThat(at(key).length > 0, `empty source: ${key}`);
}
for (const problem of problems) console.error(problem);
if (problems.length) process.exitCode = 1;
else console.log('Site publication, source pins, local links, assets and glossary checks passed.');
