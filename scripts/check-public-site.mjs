// Run against the deployed release, or a local origin, before sharing its URL.
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const site = fileURLToPath(new URL('../site/', import.meta.url));

// Canonical, preview and sitemap URLs are absolute, so a release built for another origin points readers and crawlers away from this one.
export function originMismatch(html, base) {
  const canonical = html.match(/<link rel="canonical" href="([^"]+)">/)?.[1];
  return canonical === base.href ? null : `The release names ${canonical ?? 'no canonical URL'} as its address, not ${base.href}. Set the origin in scripts/build-site.mjs, rebuild and redeploy.`;
}

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

  const home = await (await responseAt(base, './')).text();
  if (!home.includes('id="studies"') || !home.includes('href="studies/everest-ocpp/"') || !home.includes('href="studies/spec-kit/"')) throw new Error('Home page is missing the current study index.');
  const mismatch = originMismatch(home, base);
  if (mismatch && !loopback) throw new Error(mismatch);

  const study = await (await responseAt(base, 'studies/everest-ocpp/')).text();
  if (!study.includes('EVerest and OCPP 1.6: a formal study') || !study.includes('id="questions"')) throw new Error('The EVerest study is missing its current case-study entry or question filter.');
  for (const id of ['contribution', 'decisions', 'witness', 'results', 'limits', 'replay']) {
    if (!study.includes(`id="${id}"`)) throw new Error(`Study is missing ${id}.`);
  }
  for (const [old, target] of [['everest.html', 'studies/everest-ocpp/'], ['glossary.html', 'studies/everest-ocpp/glossary.html']]) {
    const moved = await (await responseAt(base, old)).text();
    if (!moved.includes(`url=${target}`)) throw new Error(`${old} no longer forwards old links to ${target}.`);
  }
  for (const [path, file] of [['studies/everest-ocpp/everest-study.json', 'studies/everest-ocpp/everest-study.json'], ['studies/spec-kit/spec-kit-study.json', 'studies/spec-kit/spec-kit-study.json']]) {
    const remote = await (await responseAt(base, path)).json();
    if (!isDeepStrictEqual(remote, JSON.parse(readFileSync(resolve(site, file), 'utf8')))) throw new Error(`${path}: publication snapshot differs from this release. Check the origin worktree or edge cache.`);
  }
  const expected = JSON.parse(readFileSync(resolve(site, 'studies/everest-ocpp/everest-study.json'), 'utf8'));
  const studyText = study.replace(/<[^>]+>/g, '');
  if (!studyText.includes(expected.studyCommit) || !studyText.includes('git checkout --detach')) throw new Error('Study is missing pinned replay instructions.');

  for (const path of ['studies/everest-ocpp/glossary.html', 'studies/spec-kit/index.html', 'studies/spec-kit-expressions/index.html', 'styles.css', 'help.js',
    'fonts/newsreader-roman.woff2', 'fonts/public-sans.woff2', 'fonts/ibm-plex-mono.woff2', 'favicon.ico', 'favicon.svg', 'apple-touch-icon.png',
    'cards/home.png', 'cards/study.png', 'cards/glossary.png', 'cards/spec-kit.png', 'cards/spec-kit-expressions.png', 'robots.txt', 'sitemap.xml', 'llms.txt']) {
    const deployed = Buffer.from(await (await responseAt(base, path)).arrayBuffer());
    if (!deployed.equals(readFileSync(resolve(site, path)))) throw new Error(`${path}: deployed file differs from this release.`);
  }
  return [`Release matches at ${base.href}`, `EVerest study: ${base.href}studies/everest-ocpp/`, `Spec Kit study: ${base.href}studies/spec-kit/`, `EVerest snapshot: ${expected.studyCommit}`, 'Study index, contribution, decisions, replay, old-link forwarding, both records, glossary, Spec Kit pages, styles, script, fonts, icons, preview cards, robots.txt, sitemap and llms.txt passed.'];
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
