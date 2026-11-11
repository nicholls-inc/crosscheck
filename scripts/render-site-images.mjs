// Draw the favicon set and link-preview cards into site/. Needs ImageMagick and Google Chrome; rerun after changing the mark, a title or the palette.
// --cards-only redraws just the cards. CHROME names the browser binary, and CHROME_NO_SANDBOX=1 lets it run as root; MAGICK names ImageMagick 7's magick, or ImageMagick 6's convert.
import { mkdirSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';

const root = fileURLToPath(new URL('../', import.meta.url));
const site = resolve(root, 'site');
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const ground = '#f8f7f4';
const ink = '#18191b';
const mark = 'M0 0H6V8H10.5V13H6V24H0ZM24 0H18V11H13.5V16H18V24H24Z';
const magick = process.env.MAGICK ?? 'magick';
const cardsOnly = process.argv.includes('--cards-only');
const work = mkdtempSync(join(tmpdir(), 'crosscheck-images-'));

if (!cardsOnly) {
// Browser tabs can be dark, so the vector icon swaps to the paper colour there.
writeFileSync(resolve(site, 'favicon.svg'), `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><style>path{fill:${ink}}@media (prefers-color-scheme:dark){path{fill:${ground}}}</style><path d="${mark}" fill-rule="evenodd"/></svg>\n`);

// Raster icons sit on paper so they read on any tab or home screen; iOS masks the touch icon's corners itself.
const tile = (size, inset, radius) => {
  const path = join(work, `tile-${size}.svg`);
  const scale = (size - 2 * inset) / 24;
  writeFileSync(path, `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><rect width="${size}" height="${size}" rx="${radius}" fill="${ground}"/><path transform="translate(${inset} ${inset}) scale(${scale})" d="${mark}" fill="${ink}" fill-rule="evenodd"/></svg>`);
  return path;
};
const raster = (svg, out) => execFileSync(magick, ['-background', 'none', svg, out]);
const icoSizes = [16, 32, 48].map((size) => {
  const out = join(work, `icon-${size}.png`);
  raster(tile(size, size / 8, size / 6), out);
  return out;
});
execFileSync(magick, [...icoSizes, resolve(site, 'favicon.ico')]);
raster(tile(180, 38, 0), resolve(site, 'apple-touch-icon.png'));
}

const fonts = pathToFileURL(resolve(site, 'fonts')).href;
const card = ({ title, line }) => `<!doctype html><meta charset="utf-8"><style>
@font-face { font-family: Newsreader; src: url("${fonts}/newsreader-roman.woff2"); font-weight: 200 800; }
@font-face { font-family: "Public Sans"; src: url("${fonts}/public-sans.woff2"); font-weight: 400 700; }
html, body { margin: 0; width: 1200px; height: 630px; overflow: hidden; background: ${ground}; color: ${ink}; }
body { box-sizing: border-box; padding: 72px 88px 64px; display: flex; flex-direction: column; font-family: Newsreader, Georgia, serif; }
.lockup { display: flex; align-items: center; gap: 18px; font-size: 40px; font-weight: 520; font-variation-settings: "opsz" 24; letter-spacing: -0.004em; }
.lockup svg { width: 36px; height: 36px; }
h1 { margin: 64px 0 0; font-size: 84px; line-height: 1.06; font-weight: 320; font-variation-settings: "opsz" 72; letter-spacing: -0.03em; max-width: 980px; }
p { margin: 28px 0 0; font-size: 36px; line-height: 1.3; font-weight: 380; color: #25282e; max-width: 960px; }
mark { background: #f8eab4; color: inherit; padding: 0 0.08em; }
footer { margin-top: auto; border-top: 1.5px solid #7d808a; padding-top: 22px; font: 500 22px/1 "Public Sans", Arial, sans-serif; letter-spacing: 0.01em; color: #4f535b; }
</style>
<div class="lockup"><svg viewBox="0 0 24 24"><path d="${mark}" fill="${ink}" fill-rule="evenodd"/></svg>Crosscheck</div>
<h1>${title}</h1><p>${line}</p>
<footer>Proved · Tested · Decided by Harry · Assumed · Not yet reached</footer>`;

const cards = {
  'cards/home.png': { title: 'Studies of code checked with evidence', line: 'Proofs, tests and reproductions, each marked with <mark>how strong it is</mark> and what it does not reach.' },
  'cards/study.png': { title: 'EVerest and OCPP 1.6: a formal study', line: 'EVerest restarts a daily charging schedule <mark>one second early</mark>. Each answer shows its evidence.' },
  'cards/glossary.png': { title: 'Glossary', line: 'Plain-English explanations of the specialist terms in the EVerest and OCPP 1.6 study.' },
  'cards/spec-kit.png': { title: 'Spec Kit: 28 reported defects', line: 'Each with a reproduction and the line of Spec Kit’s own documentation it contradicts. <mark>19 have a merged fix.</mark>' },
  'cards/spec-kit-expressions.png': { title: 'Spec Kit’s workflow expressions', line: 'With n = 50, <mark>0 &lt;= n &lt;= 10</mark> came out true, and the workflow took the wrong branch.' },
};
// PLAYWRIGHT=1 draws the cards with an installed Playwright instead of Chrome's command line, which does not finish in some containers.
const browser = process.env.PLAYWRIGHT ? await createRequire(import.meta.url)('playwright').chromium.launch() : null;
const tab = browser && await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 });
for (const [out, content] of Object.entries(cards)) {
  const page = join(work, 'card.html');
  writeFileSync(page, card(content));
  mkdirSync(resolve(site, 'cards'), { recursive: true });
  if (tab) {
    await tab.goto(pathToFileURL(page).href);
    await tab.evaluate(() => document.fonts.ready);
    await tab.screenshot({ path: resolve(site, out) });
    continue;
  }
  execFileSync(chrome, [...(process.env.CHROME_NO_SANDBOX ? ['--no-sandbox'] : []), '--headless', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files',
    `--user-data-dir=${join(work, 'chrome')}`, '--virtual-time-budget=3000', '--window-size=1200,630', `--screenshot=${resolve(site, out)}`, pathToFileURL(page).href], { stdio: 'ignore' });
}
await browser?.close();
rmSync(work, { recursive: true, force: true });
console.log(`Wrote ${cardsOnly ? '' : 'favicon.svg, favicon.ico, apple-touch-icon.png and '}${Object.keys(cards).join(', ')}`);
