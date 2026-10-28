// Run against the deployed release, or a local origin, before sharing its URL.
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const site = fileURLToPath(new URL('../site/', import.meta.url));

async function responseAt(base, path) {
  const response = await fetch(new URL(path, base), { redirect: 'error', signal: AbortSignal.timeout(10000) });
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return response;
}

export async function checkPublicSite(input) {
  const base = new URL(input);
  const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(base.hostname);
  if (base.protocol !== 'https:' && !(base.protocol === 'http:' && loopback)) throw new Error('Use HTTPS for a public URL; HTTP is allowed only on localhost.');
  if (base.username || base.password || base.search || base.hash) throw new Error('Supply a site URL without credentials, query or fragment.');
  if (!base.pathname.endsWith('/')) base.pathname += '/';

  const study = await (await responseAt(base, './')).text();
  if (!study.includes('EVerest restarts a daily charging schedule one second early.') || !study.includes('id="questions"')) throw new Error('Home page is missing the current case-study entry or question filter.');
  for (const id of ['contribution', 'decisions', 'witness', 'results', 'limits', 'replay']) {
    if (!study.includes(`id="${id}"`)) throw new Error(`Study is missing ${id}.`);
  }
  const moved = await (await responseAt(base, 'everest.html')).text();
  if (!moved.includes('url=./')) throw new Error('everest.html no longer forwards old links to the study.');
  const remote = await (await responseAt(base, 'everest-study.json')).json();
  const expected = JSON.parse(readFileSync(resolve(site, 'everest-study.json'), 'utf8'));
  if (!isDeepStrictEqual(remote, expected)) throw new Error('Publication snapshot differs from this release. Check the origin worktree or edge cache.');
  if (!study.includes(expected.studyCommit) || !study.includes('git checkout --detach')) throw new Error('Study is missing pinned replay instructions.');

  for (const path of ['glossary.html', 'styles.css', 'help.js', 'fonts/newsreader-roman.woff2', 'fonts/public-sans.woff2', 'fonts/ibm-plex-mono.woff2']) {
    const deployed = Buffer.from(await (await responseAt(base, path)).arrayBuffer());
    if (!deployed.equals(readFileSync(resolve(site, path)))) throw new Error(`${path}: deployed file differs from this release.`);
  }
  return [`Release matches at ${base.href}`, `Study: ${base.href}`, `Snapshot: ${expected.studyCommit}`, 'Contribution, decisions, replay, old-link forwarding, record, glossary, styles, script and fonts passed.'];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (!process.argv[2]) throw new Error('Usage: node scripts/check-public-site.mjs https://YOUR-HOSTNAME/');
    console.log((await checkPublicSite(process.argv[2])).join('\n'));
  } catch (error) {
    console.error(`Public site check failed: ${error.message}`);
    process.exitCode = 1;
  }
}
