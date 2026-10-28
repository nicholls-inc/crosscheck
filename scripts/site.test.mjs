import { test, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
let fixture;
beforeEach(() => {
  fixture = mkdtempSync(join(tmpdir(), 'crosscheck-site-'));
  cpSync(join(root, 'site'), join(fixture, 'site'), { recursive: true });
  for (const script of ['build-site.mjs', 'check-site.mjs']) cpSync(join(root, 'scripts', script), join(fixture, 'scripts', script), { recursive: true });
});
afterEach(() => rmSync(fixture, { recursive: true, force: true }));
const run = (script, ...args) => spawnSync(process.execPath, [join(fixture, 'scripts', script), ...args], { encoding: 'utf8' });
const edit = (name, change) => {
  const path = join(fixture, 'site', name);
  writeFileSync(path, change(readFileSync(path, 'utf8')));
};

test('checked-in publication is fresh and local navigation is intact', () => {
  assert.equal(run('build-site.mjs', '--check').status, 0);
  assert.equal(run('check-site.mjs').status, 0);
});

test('changing the publication seed requires regeneration', () => {
  edit('everest-study.json', (text) => {
    const record = JSON.parse(text);
    record.seed = 123;
    return JSON.stringify(record);
  });
  const stale = run('build-site.mjs', '--check');
  assert.equal(stale.status, 1);
  assert.match(stale.stderr, /index.html: publication fragments need regeneration/);
  assert.equal(run('build-site.mjs').status, 0);
  assert.equal(run('build-site.mjs', '--check').status, 0);
});

test('a moving artifact link cannot masquerade as the pinned study', () => {
  edit('index.html', (text) => text.replace(/\/blob\/[a-f0-9]{40}\//, '/blob/main/'));
  const result = run('check-site.mjs');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /moving or mismatched study source/);
});

test('broken replay navigation and glossary drift fail validation', () => {
  edit('index.html', (text) => text.replace('id="replay"', 'id="renamed-replay"').replace('An extra second sometimes added', 'A changed definition sometimes added'));
  const result = run('check-site.mjs');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /missing anchor:.*replay/);
  assert.match(result.stderr, /definition def-leap-second differs/);
});

test('case and instant totals must balance independently', () => {
  edit('everest-study.json', (text) => {
    const record = JSON.parse(text);
    record.results[0].pass -= 1;
    record.counters.older.extraLimit += 1;
    return JSON.stringify(record);
  });
  const result = run('check-site.mjs');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /totals must equal case count/);
  assert.match(result.stderr, /all instant counters must balance/);
});

test('author contribution is rendered from the record, not stale article copy', () => {
  edit('everest-study.json', (text) => {
    const record = JSON.parse(text);
    record.contribution.summary = 'A revised contribution account.';
    return JSON.stringify(record);
  });
  assert.equal(run('build-site.mjs', '--check').status, 1);
  assert.equal(run('build-site.mjs').status, 0);
  const article = readFileSync(join(fixture, 'site/index.html'), 'utf8');
  assert.ok(article.includes('A revised contribution account.'));
  assert.ok(article.includes('AI assistance'));
  assert.ok(article.includes('id="decisions"'));
});
