import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { checkPublicSite, originMismatch } from './check-public-site.mjs';

const site = new URL('../site/', import.meta.url);
let server;
let base;
let mode = 'current';
before(async () => {
  server = createServer((req, res) => {
    const path = req.url === '/' ? 'index.html' : req.url.slice(1);
    try {
      let data = readFileSync(fileURLToPath(new URL(path, site)));
      if (mode === 'old-home' && path === 'index.html') data = Buffer.from('<h1>Old site</h1>');
      if (mode === 'old-record' && path === 'everest-study.json') data = Buffer.from('{}');
      if (mode === 'old-font' && path.endsWith('.woff2')) data = Buffer.from('wrong font');
      if (mode === 'old-card' && path === 'cards/study.png') data = Buffer.from('wrong card');
      if (mode === 'missing-replay' && path === 'index.html') data = Buffer.from(data.toString().replace('id="replay"', 'id="old-replay"'));
      res.end(data);
    } catch { res.writeHead(404).end(); }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  base = `http://127.0.0.1:${server.address().port}/`;
});
after(() => new Promise((resolve) => server.close(resolve)));

test('current release passes through HTTP routes', async () => {
  mode = 'current';
  assert.match((await checkPublicSite(base)).join('\n'), /Contribution, decisions, replay/);
});

test('old worktree, mismatched record, wrong assets and missing replay fail', async () => {
  for (const [next, message] of [['old-home', /current case-study entry/], ['old-record', /snapshot differs/], ['old-font', /deployed file differs/], ['old-card', /cards\/study.png: deployed file differs/], ['missing-replay', /missing replay/]]) {
    mode = next;
    await assert.rejects(checkPublicSite(base), message);
  }
});

test('public plaintext URLs are rejected before a network call', async () => {
  await assert.rejects(checkPublicSite('http://example.com/'), /Use HTTPS/);
});

test('a release built for another origin is refused at the public URL', () => {
  const home = readFileSync(fileURLToPath(new URL('index.html', site)), 'utf8');
  const canonical = home.match(/<link rel="canonical" href="([^"]+)">/)[1];
  assert.equal(originMismatch(home, new URL(canonical)), null);
  assert.match(originMismatch(home, new URL('https://study.example.org/')), /names .* as its address, not https:\/\/study.example.org\//);
});
