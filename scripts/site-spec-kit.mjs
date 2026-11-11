// Fragments for the two Spec Kit study pages. Every count and table renders from spec-kit-study.json.
const esc = (value) => String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const prose = (value) => esc(String(value).replace(/"([^"]*)"/g, '“$1”').replace(/'/g, '’'));
const newTab = '<span class="visually-hidden"> (opens in a new tab)</span>';
// Report titles use Markdown code spans; the code keeps its straight quotes, the prose around it gets curly ones.
const withCode = (value) => String(value).split('`').map((part, i) => (i % 2 ? `<code>${esc(part)}</code>` : prose(part))).join('');

export function specKitCounts(record) {
  const issues = record.issues;
  const fixedBy = (i) => i.fixes.some((f) => f.state === 'merged');
  const count = (test) => issues.filter(test).length;
  return {
    total: issues.length,
    closed: count((i) => i.state === 'closed'),
    open: count((i) => i.state === 'open'),
    merged: count(fixedBy),
    openFix: count((i) => !fixedBy(i) && i.fixes.some((f) => f.state === 'open')),
    noFix: count((i) => !i.fixes.some((f) => f.state === 'merged' || f.state === 'open')),
    triage: Object.fromEntries(Object.keys(record.triageLabels).map((label) => [label, count((i) => i.triage === label)])),
    inferred: count((i) => i.expectationInferred),
    automatedInvalid: count((i) => i.automatedInvalid),
  };
}

export function specKitFragments(record) {
  const repo = record.subject.repository;
  const counts = specKitCounts(record);
  const issueLink = (n) => `<a href="${repo}/issues/${n}" target="_blank" rel="noopener">#${n}${newTab}</a>`;
  const prLink = (f) => `<a href="${repo}/pull/${f.number}" target="_blank" rel="noopener">#${f.number}${newTab}</a>`;
  const blob = (path) => `${repo}/blob/${record.filingCommit}/${path}`;
  const outcome = (i) => {
    const merged = i.fixes.filter((f) => f.state === 'merged');
    const open = i.fixes.filter((f) => f.state === 'open');
    const closedUnmerged = i.fixes.filter((f) => f.state === 'closed');
    const parts = [];
    if (merged.length) parts.push(`Fixed by ${merged.map(prLink).join(' and ')}.`);
    if (open.length) parts.push(`${merged.length ? 'Another fix' : 'A fix'}, ${open.map(prLink).join(' and ')}, is open.`);
    if (closedUnmerged.length && !merged.length && !open.length) parts.push(`A fix, ${closedUnmerged.map(prLink).join(' and ')}, was closed without merging.`);
    if (!i.fixes.length) parts.push('No fix proposed.');
    parts.push(i.state === 'closed' ? `Issue closed ${i.closed}.` : 'Issue open.');
    return parts.join(' ');
  };
  const triageName = (label) => label.replace('triage-', '').replace(/-/g, ' ');
  const percent = (n) => `${n} of ${counts.total}`;

  const findings = (area) => {
    const rows = record.issues.filter((i) => i.area === area);
    return `<table class="ledger ledger-reports"><caption class="visually-hidden">${esc(record.areas[area])}: ${rows.length} ${rows.length === 1 ? 'report' : 'reports'}</caption><thead><tr><th scope="col">Report</th><th scope="col">What goes wrong</th><th scope="col">What it contradicts</th><th scope="col">Upstream</th></tr></thead><tbody>${rows.map((i) => `<tr id="i${i.number}"><td data-label="Report">${issueLink(i.number)}</td><td data-label="What goes wrong">${withCode(i.title)}</td><td data-label="What it contradicts">${prose(i.contradicts)}</td><td data-label="Upstream">${outcome(i)} Triaged “${esc(triageName(i.triage))}”.</td></tr>`).join('')}</tbody></table>`;
  };

  return {
    'spec-kit-summary': () => `<table class="ledger ledger-numeric"><caption>Where the ${counts.total} reports stand on ${esc(record.checkedOn)}</caption><thead><tr><th scope="col">Upstream state</th><th scope="col">Reports</th></tr></thead><tbody>
<tr><td data-label="Upstream state">A fix has merged</td><td data-label="Reports">${counts.merged}</td></tr>
<tr><td data-label="Upstream state">A fix is proposed and open</td><td data-label="Reports">${counts.openFix}</td></tr>
<tr><td data-label="Upstream state">No fix merged or open</td><td data-label="Reports">${counts.noFix}</td></tr>
<tr><td data-label="Upstream state">Issue closed by the maintainers</td><td data-label="Reports">${counts.closed}</td></tr>
${Object.entries(counts.triage).map(([label, n]) => `<tr><td data-label="Upstream state">Triaged “${esc(triageName(label))}”</td><td data-label="Reports">${n}</td></tr>`).join('\n')}
</tbody></table>
<p class="evidence-note">Spec Kit’s maintainers give each report a triage verdict. ${Object.entries(record.triageLabels).map(([label, text]) => `“${esc(triageName(label))}” reads: ${prose(text)}.`).join(' ')} None of the ${counts.total} was triaged out of scope.</p>`,
    ...Object.fromEntries(Object.keys(record.areas).map((area) => [`spec-kit-area-${area}`, () => findings(area)])),
    'spec-kit-cgv': () => {
      const m = record.method;
      const t = m.cgvTriage;
      return `<table class="ledger ledger-cgv"><caption>CGV on Spec Kit at <code>${record.filingCommit.slice(0, 7)}</code></caption><thead><tr><th scope="col">Scope</th><th scope="col">Functions</th><th scope="col">Edges</th><th scope="col">Exit</th><th scope="col">Errors</th><th scope="col">Unverified</th></tr></thead><tbody>${m.cgv.map((c) => `<tr><td data-label="Scope"><code>${esc(c.scope)}</code></td><td data-label="Functions">${c.functions.toLocaleString('en-GB')}</td><td data-label="Edges">${c.edges.toLocaleString('en-GB')}</td><td data-label="Exit">${c.exit}</td><td data-label="Errors">${c.errors}</td><td data-label="Unverified">${c.unverified.toLocaleString('en-GB')}</td></tr>`).join('')}</tbody></table>
<table class="ledger ledger-numeric"><caption>Triage of the ${t.errors} errors, at ${t.sites} distinct sites</caption><thead><tr><th scope="col">Verdict</th><th scope="col">Sites</th></tr></thead><tbody><tr><td data-label="Verdict">A real bug</td><td data-label="Sites">${t.real}</td></tr><tr><td data-label="Verdict">Only the type annotation is wrong</td><td data-label="Sites">${t.annotationOnly}</td></tr><tr><td data-label="Verdict">False positive</td><td data-label="Sites">${t.falsePositive}</td></tr></tbody></table>
<p class="evidence-note">${prose(m.cgvNote)} An exit code of 1 means only that the extracted graph has these errors.</p>`;
    },
    'spec-kit-method': () => {
      const m = record.method;
      return `<p class="study-prose">${prose(m.session)} The requests that started it read:</p>
<ul class="study-list">${m.requests.map((q) => `<li>“${esc(q)}”</li>`).join('')}</ul>
<p class="study-prose">CGV ran first. Then ${m.slices.length === 8 ? 'eight' : m.slices.length} subagents each searched one slice of Spec Kit’s source for defects: ${prose(m.slices.map((x, i) => (i ? x.toLowerCase() : x)).join('; '))}.</p>
<p class="study-prose">${prose(m.rule)} ${prose(m.reasonNote)}</p>
<p class="study-prose">${prose(m.rerun)} ${prose(m.mergedNote)}</p>
<p class="study-prose">${prose(m.duplicateSearch)}</p>
<p class="evidence-note">${prose(m.notRun)}</p>`;
    },
    'spec-kit-held': () => `<table class="ledger ledger-reports"><caption class="visually-hidden">Findings not filed</caption><thead><tr><th scope="col">Existing</th><th scope="col">Finding</th><th scope="col">Why it was not filed</th></tr></thead><tbody>${record.heldBack.map((h) => `<tr><td data-label="Existing"><a href="${repo}/${h.kind === 'issue' ? 'issues' : 'pull'}/${h.existing}" target="_blank" rel="noopener">#${h.existing}${newTab}</a></td><td data-label="Finding">${prose(h.title)}</td><td data-label="Why it was not filed">${prose(h.note)}</td></tr>`).join('')}</tbody></table>
<p class="evidence-note">These were reproduced like the others. They are not counted in the ${counts.total}.</p>
<p class="evidence-note">${prose(record.method.privateReport)}</p>`,
    'spec-kit-invalid': () => `<p>${prose(record.automatedInvalidNote)}</p>`,
    'spec-kit-environments': () => `<ul class="study-list">${record.environments.map((e) => `<li>${prose(e)}</li>`).join('')}</ul>
<p class="evidence-note">Every report names Spec Kit ${esc(record.subject.version)} at <a href="${repo}/tree/${record.filingCommit}" target="_blank" rel="noopener"><code>${record.filingCommit.slice(0, 7)}</code>${newTab}</a> and ends: “${prose(record.disclosure)}”</p>`,
    'spec-kit-limits': () => `<table class="ledger"><caption>Not yet reached by this study</caption><thead><tr><th scope="col">Behaviour</th><th scope="col">Blocking property</th><th scope="col">Open question</th></tr></thead><tbody>${record.notYetReached.map((r) => `<tr><td data-label="Behaviour">${prose(r.behavior)}</td><td data-label="Blocking property">${prose(r.blocker)}</td><td data-label="Open question">${prose(r.question)}</td></tr>`).join('')}</tbody></table>`,
    'spec-kit-replay': () => `<div class="commands"><pre id="replay-commands" tabindex="0" aria-label="Commands to repeat one reproduction"><code>${[
      `git clone ${repo}`,
      'cd spec-kit',
      `git checkout --detach ${record.filingCommit}`,
      'uv sync',
      '# Save the script from the report’s Steps to Reproduce as repro.py, then:',
      'uv run python -I repro.py',
    ].map((c) => `<span class="cmd">${esc(c).replace(/\//g, '/<wbr>').replace(/\b([a-f0-9]{40})\b/g, '<span class="hash">$1</span>')}</span>`).join('\n')}</code></pre><button type="button" class="copy" data-copy="replay-commands">Copy replay commands</button><span class="copy-status" role="status" aria-live="polite"></span></div>
<p class="evidence-note">Each report gives its own steps and its expected output under Actual Behavior. A few reports drive the command line instead of a Python script; follow the steps in the report. To see whether a defect is fixed, run the same steps on Spec Kit’s main branch.</p>`,
    'spec-kit-contribution': () => `<dl class="contribution"><div><dt>AI assistance</dt><dd>${prose(record.contribution.agents)}</dd></div>
<div><dt>Harry</dt><dd>${prose(record.contribution.human)}</dd></div>
<div><dt>What the checks establish</dt><dd>${prose(record.contribution.checks)}</dd></div></dl>`,
    'spec-kit-counts': () => `<p>${counts.total} reports were filed on ${esc(record.filedOn)}. By ${esc(record.checkedOn)}, ${percent(counts.merged)} had a merged fix and ${counts.openFix} more had a fix open for review. ${counts.noFix === 1 ? 'One had' : `${counts.noFix} had`} neither.</p>`,

    'expr-witnesses': () => `<table class="ledger ledger-witness"><caption>Expressions from the reports, at <code>${record.filingCommit.slice(0, 7)}</code></caption><thead><tr><th scope="col">Report</th><th scope="col">Expression</th><th scope="col">Spec Kit gave</th><th scope="col">Jinja2 gives</th><th scope="col">Note</th></tr></thead><tbody>${record.expressions.witnesses.map((w) => `<tr><td data-label="Report">${issueLink(w.issue)}</td><td data-label="Expression"><code>${esc(w.expression)}</code>${w.context ? `<br><span class="evidence-note">${esc(w.context)}</span>` : ''}</td><td data-label="Spec Kit gave"><code>${esc(w.specKit)}</code></td><td data-label="Jinja2 gives"><code>${esc(w.jinja2)}</code></td><td data-label="Note">${prose(w.note)}</td></tr>`).join('')}</tbody></table>
<p class="evidence-note">The Spec Kit column is the output recorded in each report. The Jinja2 column is what Jinja2 and Python give for the same expression; this page did not run them. ${prose(record.expressions.contract)} <a href="${blob('src/specify_cli/workflows/expressions.py')}#L3" target="_blank" rel="noopener">Read the docstring at the filing commit${newTab}</a></p>`,
    'expr-upstream': () => `<dl class="contribution">${record.expressions.upstream.map((u) => `<div><dt>${esc(u.date)}</dt><dd>${prose(u.change).replace(/#(\d{4})/g, (_, n) => `<a href="${repo}/${record.issues.some((i) => i.number === Number(n)) ? 'issues' : 'pull'}/${n}" target="_blank" rel="noopener">#${n}${newTab}</a>`)}</dd></div>`).join('')}</dl>`,
    'expr-reading': () => `<table class="ledger ledger-reports"><caption>A reading of main at <code>${record.mainCommit.slice(0, 7)}</code>, not yet run</caption><thead><tr><th scope="col">Report</th><th scope="col">Case</th><th scope="col">What the source suggests</th></tr></thead><tbody>${record.expressions.reading.map((r) => `<tr><td data-label="Report">${issueLink(r.issue)}</td><td data-label="Case"><code>${esc(r.case)}</code></td><td data-label="What the source suggests">${prose(r.reading)}</td></tr>`).join('')}</tbody></table>
<p class="evidence-note">${prose(record.expressions.readingNote)}</p>`,
  };
}
