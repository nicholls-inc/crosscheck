// Publication and navigation checks; no dependencies or network access.
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const site = resolve(root, 'site');
const record = JSON.parse(readFileSync(resolve(site, 'everest-study.json'), 'utf8'));
const problems = [];
const requireThat = (condition, message) => { if (!condition) problems.push(message); };
const definitions = (html) => new Map([...html.matchAll(/<(?:p|dd) id="(def-[a-z-]+)">([\s\S]*?)<\/(?:p|dd)>/g)].map((m) => [m[1], m[2].trim()]));

requireThat(record.format === 'crosscheck-publication-snapshot/1', 'record must be an editorial publication snapshot');
requireThat(/^[a-f0-9]{40}$/.test(record.studyCommit), 'study revision must be a full immutable commit');
requireThat(/^[a-f0-9]{40}$/.test(record.everestCommit), 'EVerest revision must be a full immutable commit');
requireThat(record.commands.includes(`git checkout --detach ${record.studyCommit}`), 'replay must check out the publication revision');
requireThat(record.classificationCaveat.includes('1,081'), 'retain the extra-limit classification caveat');
for (const r of record.results) {
  requireThat([r.pass, r.warn, r.fail].every(Number.isSafeInteger), `${r.configuration}: counts must be integers`);
  requireThat(r.pass + r.warn + r.fail === r.cases, `${r.configuration}: totals must equal case count`);
}
for (const c of Object.values(record.counters)) {
  requireThat(c.exact + c.below + c.aboveOrMissing === c.modelHasLimit, 'limited-instant counters must balance');
  requireThat(c.modelHasLimit + c.extraLimit + c.neither === c.instants, 'all instant counters must balance');
}

const glossary = definitions(readFileSync(resolve(site, 'glossary.html'), 'utf8'));
requireThat(glossary.size > 0, 'glossary must contain definitions');
for (const name of readdirSync(site).filter((name) => name.endsWith('.html'))) {
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
    const target = resolve(dirname(path), local || name);
    const file = local === './' ? resolve(site, 'index.html') : target;
    requireThat(!relative(site, file).startsWith('..'), `${name}: link escapes site: ${value}`);
    requireThat(existsSync(file), `${name}: missing local target: ${value}`);
    if (anchor && existsSync(file)) {
      const content = readFileSync(file, 'utf8');
      requireThat(content.includes(`id="${anchor}"`), `${name}: missing anchor: ${value}`);
    }
  }
  if (name === 'glossary.html') continue;
  const tips = definitions(html);
  requireThat(tips.size === glossary.size, `${name}: glossary definition count differs`);
  for (const [id, text] of glossary) requireThat(tips.get(id) === text, `${name}: definition ${id} differs from glossary`);
  for (const [, term] of html.matchAll(/data-term="([a-z-]+)"/g)) requireThat(tips.has(`def-${term}`), `${name}: undefined term ${term}`);
  for (const [, target] of html.matchAll(/data-copy="([^"]+)"/g)) requireThat(ids.includes(target), `${name}: missing copy target ${target}`);
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
