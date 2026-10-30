// Render publication fragments into checked-in HTML. No browser fetch is needed.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const record = JSON.parse(readFileSync(resolve(root, 'site/everest-study.json'), 'utf8'));
const esc = (value) => String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
// Prose gets typographic quotes and apostrophes before escaping; commands, paths and attributes use esc() alone.
const prose = (value) => esc(String(value).replace(/"([^"]*)"/g, '\u201C$1\u201D').replace(/'/g, '\u2019'));
const number = (value) => value.toLocaleString('en-GB');
const source = (key, label) => `<a href="${record.repository}/blob/${record.studyCommit}/${record.sources[key]}" target="_blank" rel="noopener">${prose(label)}<span class="visually-hidden"> (opens in a new tab)</span></a>`;
const list = (items) => `<ul class="study-list">${items.map((item) => `<li>${prose(item)}</li>`).join('')}</ul>`;

function claims() {
  return `<table class="ledger"><caption>What the evidence says</caption><thead><tr><th scope="col">What was checked</th><th scope="col">How it was checked</th><th scope="col">Strength</th><th scope="col">Result and boundary</th></tr></thead><tbody>${record.claims.map((c) => `<tr><td data-label="What was checked">${prose(c.statement)}</td><td data-label="How it was checked">${prose(c.support)}</td><td class="strength" data-label="Strength">${esc(c.strength)}</td><td data-label="Result and boundary">${prose(c.boundary)}</td></tr>`).join('')}</tbody></table>
<p class="evidence-scope">Proofs cover the reference program. Tests compare EVerest with it on the inputs tried. The study reports these results; this website did not rerun them.</p>`;
}

function witness() {
  const w = record.witness;
  return `<figure class="witness"><figcaption>One daily default schedule, first started at <time datetime="${w.anchor}">11:00:00 UTC on 14 September 2016</time>.
<span class="witness-time">Read at <time datetime="${w.readAt}">10:59:59 UTC on 13 August 2026</time></span></figcaption><dl>
<div><dt>The reference program</dt><dd><span class="reading">${number(w.modelWatts)}&nbsp;W</span><span class="why">The second period still applies. Restart is at ${esc(w.expectedRestart)}.</span></dd></div>
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

const fragments = {claims, witness, results, counters, replay, limits, sources, contribution, choices,
  properties: () => list(record.modelProperties) + `<p class="evidence-note">Not yet reached: ${prose(record.unfinishedProperty)}</p>`,
  decisions: () => list(record.humanDecisions),
  trust: () => list(record.trustedDependencies),
  upstream: () => `<p>${prose(record.upstreamStatus)} <a href="${esc(record.upstreamIssueUrl)}" target="_blank" rel="noopener">View filed issue<span class="visually-hidden"> (opens in a new tab)</span></a></p>`,
};

let stale = false;
for (const name of ['index.html']) {
  const path = resolve(root, 'site', name);
  const before = readFileSync(path, 'utf8');
  const after = before.replace(/<!-- publication:([a-z]+) -->[\s\S]*?<!-- \/publication:\1 -->/g,
    (_, key) => `<!-- publication:${key} -->\n${fragments[key]()}\n<!-- /publication:${key} -->`);
  if (before === after) continue;
  if (process.argv.includes('--check')) { console.error(`${name}: publication fragments need regeneration`); stale = true; }
  else writeFileSync(path, after);
}
if (stale) process.exitCode = 1;
