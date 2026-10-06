// Tests for tier-gate.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (TG-*, see
// intent/2026-09-29-deterministic-evidence-spec.md; for TG-1, TG-8 and
// TG-11, intent/2026-09-30-tier-anchor-spec.md; for TG-12 and TG-13,
// intent/2026-09-30-citation-rule-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { evaluate } from './tier-gate.mjs';

const RULES = [
  '# Protected surfaces',
  '',
  '## Machine-readable path list',
  '',
  '```',
  'crosscheck/skills/*/SKILL.md',
  'scripts/ci/**',
  'cgv/prover/ContractGraph/BehaviorModel.lean',
  'cgv/prover/protected-statements.txt',
  'cgv/prover/scripts/ProtectedStatements.lean',
  '```',
  '',
].join('\n');

const SKILL = 'crosscheck/skills/reason/SKILL.md';
const NOTE = '.assurance/protected-surface-amend/reason-2026-09-29.md';
const STALE_NOTE = '.assurance/protected-surface-amend/old-2026-01-01.md';

// Build a throwaway repository containing `files` (path -> content).
function repo(files = {}, dir = mkdtempSync(join(tmpdir(), 'tier-gate-test-'))) {
  const all = { '.claude/rules/protected-surfaces.md': RULES, ...files };
  for (const [path, content] of Object.entries(all)) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), content);
  }
  return dir;
}

function run(cwd, { body = '', labels = [], changed = [] } = {}) {
  return evaluate({ prBody: body, prLabels: labels, changedFiles: changed, cwd });
}

const out = (result) => result.lines.join('\n');

// A Tier 3 skill edit with every artefact changed in the PR.
function tier3Repo(extra = {}) {
  return repo({ [SKILL]: 'x', 'plan.md': '# Plan', [NOTE]: `names ${SKILL}`, ...extra });
}
const TIER3_CHANGED = [SKILL, 'plan.md', NOTE];

// ---- TG-1: declaration and floor -------------------------------------------

test('TG-1: no declared tier and no protected path fails', () => {
  const r = run(repo({ 'README.md': 'x' }), { changed: ['README.md'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Declare this pull request's tier/);
});

test('TG-1: "Tier:" in the middle of a line declares nothing', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { body: 'This is Tier: 1 work', changed: ['intent/a.md'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Declare this pull request's tier/);
});

test('TG-1: a mid-line "Tier:" does not override a label', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), {
    body: 'This is Tier: 2 work',
    labels: ['tier:1'],
    changed: ['intent/a.md'],
  });
  assert.equal(r.pass, true);
  assert.match(out(r), /declared: 1,/);
});

for (const line of ['Tier: 1', '  Tier: 1', '\tTier: 1', 'tier: 1', 'Tier:1', 'Tier: 1  ', 'Tier: 1\r']) {
  test(`TG-1: ${JSON.stringify(line)} declares Tier 1`, () => {
    const r = run(repo({ 'intent/a.md': 'x' }), { body: `${line}\n`, changed: ['intent/a.md'] });
    assert.equal(r.pass, true);
    assert.match(out(r), /declared: 1,/);
  });
}

for (const line of ['- Tier: 1', '* Tier: 1', '+ Tier: 1', '> Tier: 1', '  > Tier: 1']) {
  test(`TG-1: quoted or list-item ${JSON.stringify(line)} declares nothing`, () => {
    const r = run(repo({ 'intent/a.md': 'x' }), { body: `${line}\n`, changed: ['intent/a.md'] });
    assert.equal(r.pass, false);
    assert.match(out(r), /Declare this pull request's tier/);
  });
}

test('TG-1: a quoted "Tier:" line does not override a label', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { body: '> Tier: 2', labels: ['tier:1'], changed: ['intent/a.md'] });
  assert.equal(r.pass, true);
  assert.match(out(r), /declared: 1,/);
});

test('TG-1: the first "Tier:" line wins', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { body: 'Tier: 1\nTier: 2', changed: ['intent/a.md'] });
  assert.equal(r.pass, true);
  assert.match(out(r), /declared: 1,/);
});

test('TG-1: a quoted "Tier:" line is skipped when finding the first line', () => {
  const r = run(tier3Repo(), { body: '> Tier: 1\nTier: 3', changed: TIER3_CHANGED });
  assert.equal(r.pass, true);
  assert.match(out(r), /declared: 3,/);
});

for (const body of [
  'Tier: 4',
  'Tier: 0',
  'Tier: 1.5',
  'Tier: 12',
  'Tier: one',
  'Tier:',
  'Tier: 1 (routine)',
  'Tier: 1, see below',
  'Tier: 4\nTier: 2',
]) {
  test(`TG-1: an invalid first "Tier:" line fails the gate: ${JSON.stringify(body)}`, () => {
    const r = run(repo({ 'intent/a.md': 'x' }), { body, labels: ['tier:2'], changed: ['intent/a.md'] });
    assert.equal(r.pass, false);
    assert.match(out(r), /does not declare Tier 1, 2 or 3/);
  });
}

test('TG-1: a "Tier:" line and a tier:N label that agree pass', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { body: 'Tier: 1', labels: ['tier:1'], changed: ['intent/a.md'] });
  assert.equal(r.pass, true);
  assert.match(out(r), /declared: 1,/);
});

test('TG-1: a "Tier:" line and a tier:N label that disagree fail', () => {
  const r = run(repo({ 'intent/a.md': 'x', 'spec.md': 'x' }), {
    body: 'Tier: 2',
    labels: ['tier:1'],
    changed: ['intent/a.md', 'spec.md'],
  });
  assert.equal(r.pass, false);
  assert.match(out(r), /declares Tier 2, but the label says Tier 1\. They must agree/);
});

test('TG-1: two tier:N labels that disagree fail', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { labels: ['tier:1', 'tier:2'], changed: ['intent/a.md'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /labels tier:1 and tier:2 disagree/);
});

test('TG-1: a tier:N label declares the tier', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { labels: ['tier:1'], changed: ['intent/a.md'] });
  assert.equal(r.pass, true);
});

test('TG-1: declaring below the protected floor fails', () => {
  const r = run(tier3Repo(), { body: 'Tier: 1', changed: TIER3_CHANGED });
  assert.equal(r.pass, false);
  assert.match(out(r), /force a floor of Tier 3, but the PR declares Tier 1/);
});

test('TG-1: a protected path with no declaration is treated as Tier 3', () => {
  const r = run(tier3Repo(), { changed: TIER3_CHANGED });
  assert.equal(r.pass, true);
  assert.match(out(r), /Tier 3 artefacts present/);
});

// ---- TG-2: Tier 1 ------------------------------------------------------------

test('TG-2: Tier 1 passes with a changed intent file', () => {
  const r = run(repo({ 'intent/2026-09-29-x.md': 'x' }), { body: 'Tier: 1', changed: ['intent/2026-09-29-x.md'] });
  assert.equal(r.pass, true);
});

test('TG-2: Tier 1 does not count intent/README.md or TEMPLATE.md', () => {
  const r = run(repo({ 'intent/README.md': 'x', 'intent/TEMPLATE.md': 'x' }), {
    body: 'Tier: 1',
    changed: ['intent/README.md', 'intent/TEMPLATE.md'],
  });
  assert.equal(r.pass, false);
});

test('TG-2: Tier 1 passes with an Intent: citation of an existing file', () => {
  const r = run(repo({ 'intent/old.md': 'x', 'README.md': 'x' }), {
    body: 'Tier: 1\nIntent: intent/old.md',
    changed: ['README.md'],
  });
  assert.equal(r.pass, true);
});

test('TG-2: Tier 1 fails when the cited intent does not exist', () => {
  const r = run(repo({ 'README.md': 'x' }), { body: 'Tier: 1\nIntent: intent/missing.md', changed: ['README.md'] });
  assert.equal(r.pass, false);
});

// ---- TG-3: Tier 2 spec --------------------------------------------------------

test('TG-3: an unchanged root spec.md does not satisfy Tier 2', () => {
  const r = run(repo({ 'spec.md': 'old', 'src/a.ts': 'x' }), { body: 'Tier: 2', changed: ['src/a.ts'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 2 requires a spec artefact/);
});

test('TG-3: a root spec.md changed in the PR satisfies Tier 2', () => {
  const r = run(repo({ 'spec.md': 'new', 'src/a.ts': 'x' }), { body: 'Tier: 2', changed: ['src/a.ts', 'spec.md'] });
  assert.equal(r.pass, true);
});

test('TG-3: a Spec: citation (a CGV Lean file) satisfies Tier 2', () => {
  const lean = 'cgv/prover/ContractGraph/Checker.lean';
  const r = run(repo({ [lean]: 'x' }), { body: `Tier: 2\nSpec: \`${lean}\``, changed: [lean] });
  assert.equal(r.pass, true);
});

test('TG-3: a deleted root spec.md does not count', () => {
  const r = run(repo({ 'src/a.ts': 'x' }), { body: 'Tier: 2', changed: ['src/a.ts', 'spec.md'] });
  assert.equal(r.pass, false);
});

// ---- TG-4: Tier 3 plan --------------------------------------------------------

test('TG-4: a root plan.md from an earlier change does not satisfy Tier 3', () => {
  const r = run(tier3Repo(), { changed: [SKILL, NOTE] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 3 requires a build plan/);
});

test('TG-4: a Plan: citation of an existing file satisfies the plan requirement', () => {
  const r = run(tier3Repo({ 'intent/p.md': '# Plan' }), { body: 'Plan: intent/p.md', changed: [SKILL, NOTE] });
  assert.equal(r.pass, true);
});

test('TG-4: "plan:" in the middle of a line is not a citation', () => {
  const r = run(tier3Repo({ 'intent/p.md': '# Plan' }), {
    body: 'See the build plan: intent/p.md',
    changed: [SKILL, NOTE],
  });
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 3 requires a build plan/);
});

for (const line of ['- Plan: intent/p.md', '* Plan: intent/p.md', '> Plan: intent/p.md', 'Plan: intent/p.md\r']) {
  test(`TG-4: ${JSON.stringify(line)} is a citation`, () => {
    const r = run(tier3Repo({ 'intent/p.md': '# Plan' }), { body: `Tier: 3\n${line}\n`, changed: [SKILL, NOTE] });
    assert.equal(r.pass, true);
  });
}

test('TG-4: an indented Plan: line on its own is a citation', () => {
  const r = run(tier3Repo({ 'intent/p.md': '# Plan' }), {
    body: 'Tier: 3\n  Plan: intent/p.md',
    changed: [SKILL, NOTE],
  });
  assert.equal(r.pass, true);
});

// ---- TG-12: citations -----------------------------------------------------

// A Tier 3 repository at <parent>/repo, with <parent>/outside.md beside it.
function nestedTier3Repo(extra = {}) {
  const parent = mkdtempSync(join(tmpdir(), 'tier-gate-test-'));
  writeFileSync(join(parent, 'outside.md'), '# Plan');
  const dir = repo({ [SKILL]: 'x', [NOTE]: `names ${SKILL}`, 'intent/p.md': '# Plan', ...extra }, join(parent, 'repo'));
  return { parent, dir };
}

const planResult = (dir, body) => run(dir, { body: `Tier: 3\n${body}`, changed: [SKILL, NOTE] });

test('TG-12: "+ Plan:" is a citation', () => {
  assert.equal(planResult(nestedTier3Repo().dir, '+ Plan: intent/p.md').pass, true);
});

for (const line of ['-Plan: intent/p.md', '>> Plan: intent/p.md', '1. Plan: intent/p.md', '- [ ] Plan: intent/p.md']) {
  test(`TG-12: ${JSON.stringify(line)} is not a citation`, () => {
    const r = planResult(nestedTier3Repo().dir, line);
    assert.equal(r.pass, false);
    assert.match(out(r), /Tier 3 requires a build plan/);
  });
}

test('TG-12: a valid citation after an invalid one counts', () => {
  assert.equal(planResult(nestedTier3Repo().dir, 'Plan: TBD\nPlan: intent/p.md').pass, true);
});

test('TG-12: a valid citation before an invalid one counts', () => {
  assert.equal(planResult(nestedTier3Repo().dir, 'Plan: intent/p.md\nPlan: TBD').pass, true);
});

test('TG-12: two invalid citations fail', () => {
  assert.equal(planResult(nestedTier3Repo().dir, 'Plan: intent/missing.md\nPlan: intent').pass, false);
});

test('TG-12: a directory is not a cited file', () => {
  const r = planResult(nestedTier3Repo().dir, 'Plan: intent');
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 3 requires a build plan/);
});

test('TG-12: a relative path out of the repository does not count', () => {
  assert.equal(planResult(nestedTier3Repo().dir, 'Plan: ../outside.md').pass, false);
});

test('TG-12: an absolute path out of the repository does not count', () => {
  const { parent, dir } = nestedTier3Repo();
  assert.equal(planResult(dir, `Plan: ${join(parent, 'outside.md')}`).pass, false);
});

test('TG-12: a symlink to a file outside the repository does not count', () => {
  const { parent, dir } = nestedTier3Repo();
  symlinkSync(join(parent, 'outside.md'), join(dir, 'link.md'));
  assert.equal(planResult(dir, 'Plan: link.md').pass, false);
});

test('TG-12: a symlink to a file inside the repository counts', () => {
  const { dir } = nestedTier3Repo();
  symlinkSync(join(dir, 'intent/p.md'), join(dir, 'link.md'));
  assert.equal(planResult(dir, 'Plan: link.md').pass, true);
});

test('TG-12: a file whose name starts with two dots counts', () => {
  assert.equal(planResult(nestedTier3Repo({ '..plan.md': '# Plan' }).dir, 'Plan: ..plan.md').pass, true);
});

test('TG-12: a valid Intent: citation after an invalid one satisfies Tier 1', () => {
  const r = run(repo({ 'intent/old.md': 'x', 'README.md': 'x' }), {
    body: 'Tier: 1\nIntent: TBD\nIntent: intent/old.md',
    changed: ['README.md'],
  });
  assert.equal(r.pass, true);
});

test('TG-12: a Spec: citation of a directory does not satisfy Tier 2', () => {
  const r = run(repo({ 'intent/s.md': 'x', 'src/a.ts': 'x' }), { body: 'Tier: 2\nSpec: intent', changed: ['src/a.ts'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 2 requires a spec artefact/);
});

// ---- TG-5: governance notes ------------------------------------------------

test('TG-5: a governance note already in the tree does not count', () => {
  const dir = repo({ [SKILL]: 'x', 'plan.md': 'x', [STALE_NOTE]: `names ${SKILL}` });
  const r = run(dir, { changed: [SKILL, 'plan.md'] });
  assert.equal(r.pass, false);
  assert.match(out(r), /governance note changed in this pull request/);
});

test('TG-5: a changed note must name every changed protected file', () => {
  const other = 'scripts/ci/x.mjs';
  const r = run(tier3Repo({ [other]: 'x' }), { changed: [...TIER3_CHANGED, other] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Not named: scripts\/ci\/x\.mjs/);
});

test('TG-5: an add-session note counts', () => {
  const session = '.assurance/add-session-abc/protected-surface-amend-reason.md';
  const dir = repo({ [SKILL]: 'x', 'plan.md': 'x', [session]: `names ${SKILL}` });
  const r = run(dir, { changed: [SKILL, 'plan.md', session] });
  assert.equal(r.pass, true);
});

test('TG-5: a note outside the governance directories does not count', () => {
  const stray = 'docs/notes.md';
  const dir = repo({ [SKILL]: 'x', 'plan.md': 'x', [stray]: `names ${SKILL}` });
  const r = run(dir, { changed: [SKILL, 'plan.md', stray] });
  assert.equal(r.pass, false);
});

// ---- TG-6: no attestation ---------------------------------------------------

test('TG-6: Tier 3 passes with no intent-check attestation anywhere', () => {
  const r = run(tier3Repo(), { changed: TIER3_CHANGED });
  assert.equal(r.pass, true);
  assert.doesNotMatch(out(r), /attestation/i);
});

test('TG-6: a passing attestation does not stand in for a missing plan', () => {
  const att = 'crosscheck/mcp-server/.assurance/intent-check-attestation.json';
  const dir = tier3Repo({ [att]: '{"verdict":"pass"}' });
  const r = run(dir, { changed: [SKILL, NOTE, att] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Tier 3 requires a build plan/);
});

// ---- TG-7: CGV proof surfaces ----------------------------------------------

const MANIFEST = 'cgv/prover/protected-statements.txt';

function cgvRepo() {
  return repo({ [MANIFEST]: 'x', 'plan.md': 'x', [NOTE]: `names ${MANIFEST}` });
}
const CGV_CHANGED = [MANIFEST, 'plan.md', NOTE];

test('TG-7: a manifest change without the protected-surface change section fails', () => {
  const r = run(cgvRepo(), { body: 'Tier: 3', changed: CGV_CHANGED });
  assert.equal(r.pass, false);
  assert.match(out(r), /Protected-surface change/);
});

test('TG-7: the section heading satisfies the check, case-insensitively', () => {
  const r = run(cgvRepo(), { body: 'Tier: 3\n\n### protected-surface CHANGE\n\nWhat changed…', changed: CGV_CHANGED });
  assert.equal(r.pass, true);
});

test('TG-7: the phrase outside a heading does not satisfy the check', () => {
  const r = run(cgvRepo(), { body: 'Tier: 3\nNo Protected-surface change here.', changed: CGV_CHANGED });
  assert.equal(r.pass, false);
});

test('TG-7: BehaviorModel.lean also requires the section', () => {
  const bm = 'cgv/prover/ContractGraph/BehaviorModel.lean';
  const dir = repo({ [bm]: 'x', 'plan.md': 'x', [NOTE]: `names ${bm}` });
  const r = run(dir, { body: 'Tier: 3', changed: [bm, 'plan.md', NOTE] });
  assert.equal(r.pass, false);
});

test('TG-7: the manifest generator is protected and requires the section', () => {
  const gen = 'cgv/prover/scripts/ProtectedStatements.lean';
  const floor = run(repo({ [gen]: 'x', 'intent/i.md': 'x' }), { body: 'Tier: 1', changed: [gen, 'intent/i.md'] });
  assert.equal(floor.pass, false);
  assert.match(out(floor), /force a floor of Tier 3/);
  const dir = repo({ [gen]: 'x', 'plan.md': 'x', [NOTE]: `names ${gen}` });
  const r = run(dir, { body: 'Tier: 3', changed: [gen, 'plan.md', NOTE] });
  assert.equal(r.pass, false);
  assert.match(out(r), /Protected-surface change/);
});

// ---- TG-8 / TG-9: pass report ------------------------------------------------

const EVIDENCE_HEADER = 'Deterministic evidence for this diff:';

function reportRows(result) {
  const start = result.lines.indexOf(EVIDENCE_HEADER) + 1;
  const end = result.lines.indexOf('', start);
  return result.lines.slice(start, end);
}

test('TG-8: the pass report names the evidence for each class of changed file', () => {
  const files = { 'cgv/src/main.rs': 'x', 'crosscheck/mcp-server/src/a.ts': 'x', 'intent/a.md': 'x' };
  const r = run(repo(files), { body: 'Tier: 1', changed: Object.keys(files) });
  assert.equal(r.pass, true);
  assert.deepEqual(reportRows(r), [
    '- CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check): 1 file(s), e.g. cgv/src/main.rs',
    '- CI workflow (npm test, including the property tests): 1 file(s), e.g. crosscheck/mcp-server/src/a.ts',
    '- not yet reached: 1 file(s), e.g. intent/a.md. Blocking property: prose has no executable meaning, so no check reads what it claims. Open question: which claims in a prose document, such as cited paths and commands, a deterministic check can verify.',
  ]);
});

test('TG-8: skill edits are reported as not yet reached', () => {
  const r = run(tier3Repo(), { changed: TIER3_CHANGED });
  assert.ok(
    reportRows(r).includes(
      '- not yet reached: 1 file(s), e.g. crosscheck/skills/reason/SKILL.md. Blocking property: their behaviour is prompt text that an agent interprets. Open question: what a replayable behavioural eval of a prompt artefact looks like.'
    )
  );
});

test('TG-8: a pass whose changed files hit every class reports one line per class, in order', () => {
  const changed = [
    'cgv/src/main.rs',
    'crosscheck/mcp-server/src/a.ts',
    'crosscheck/conformance/main.go',
    '.claude/hooks/protected-surface-guard.mjs',
    'scripts/check-evidence-record.mjs',
    'evals/a.json',
    'docs/assurance/ROADMAP.md',
    '.github/workflows/ci.yml',
    'docs/invariants/tier-gate.md',
    'intent/a.md',
    'package.json',
  ];
  const r = run(repo(Object.fromEntries(changed.map((f) => [f, 'x']))), { body: 'Tier: 1', changed });
  assert.equal(r.pass, true);
  assert.deepEqual(reportRows(r), [
    '- CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check): 1 file(s), e.g. cgv/src/main.rs',
    '- CI workflow (npm test, including the property tests): 1 file(s), e.g. crosscheck/mcp-server/src/a.ts',
    '- CI workflow, conformance job (go vet, go test, go run . ..): 1 file(s), e.g. crosscheck/conformance/main.go',
    '- Tier Gate workflow (node --test scripts/ci/*.test.mjs): 1 file(s), e.g. .claude/hooks/protected-surface-guard.mjs',
    '- Evidence Record workflow (node --test scripts/check-evidence-record.test.mjs): 1 file(s), e.g. scripts/check-evidence-record.mjs',
    '- Incident Eval Check workflow: 1 file(s), e.g. evals/a.json',
    '- not yet reached: 1 file(s), e.g. docs/assurance/ROADMAP.md. Blocking property: their behaviour is prompt text that an agent interprets. Open question: what a replayable behavioural eval of a prompt artefact looks like.',
    "- not yet reached: 1 file(s), e.g. .github/workflows/ci.yml. Blocking property: a workflow runs only on GitHub's runners, on GitHub's events. Open question: how to replay a workflow against recorded events before it merges.",
    '- not yet reached: 1 file(s), e.g. docs/invariants/tier-gate.md. Blocking property: no CI job maps these invariants to the tests that cover them. Open question: which test covers each invariant, and which workflow checks that mapping.',
    '- not yet reached: 1 file(s), e.g. intent/a.md. Blocking property: prose has no executable meaning, so no check reads what it claims. Open question: which claims in a prose document, such as cited paths and commands, a deterministic check can verify.',
    '- not yet reached: 1 file(s), e.g. package.json. Blocking property: no CI workflow runs a check on this path. Open question: which deterministic check this code needs, and which workflow runs it.',
  ]);
});

test('TG-8: a change to .husky/pre-commit is reported against the Tier Gate workflow', () => {
  const r = run(repo({ 'intent/old.md': 'x', '.husky/pre-commit': 'x' }), {
    body: 'Tier: 1\nIntent: intent/old.md',
    changed: ['.husky/pre-commit'],
  });
  assert.equal(r.pass, true);
  assert.deepEqual(reportRows(r), [
    '- Tier Gate workflow (node --test scripts/ci/*.test.mjs): 1 file(s), e.g. .husky/pre-commit',
  ]);
});

test('TG-8: the evidence record checker and its tests are reported against the Evidence Record workflow', () => {
  const changed = ['scripts/check-evidence-record.mjs', 'scripts/check-evidence-record.test.mjs', 'scripts/other.mjs'];
  const r = run(repo({ 'intent/old.md': 'x', ...Object.fromEntries(changed.map((f) => [f, 'x'])) }), {
    body: 'Tier: 1\nIntent: intent/old.md',
    changed,
  });
  assert.equal(r.pass, true);
  assert.deepEqual(reportRows(r), [
    '- Evidence Record workflow (node --test scripts/check-evidence-record.test.mjs): 2 file(s), e.g. scripts/check-evidence-record.mjs',
    '- not yet reached: 1 file(s), e.g. scripts/other.mjs. Blocking property: no CI workflow runs a check on this path. Open question: which deterministic check this code needs, and which workflow runs it.',
  ]);
});

test('TG-8: a change under a path no workflow checks is never reported as needing no evidence', () => {
  const r = run(repo({ 'intent/old.md': 'x', 'crosscheck/scripts/build.sh': 'x' }), {
    body: 'Tier: 1\nIntent: intent/old.md',
    changed: ['crosscheck/scripts/build.sh'],
  });
  assert.equal(r.pass, true);
  assert.doesNotMatch(out(r), /none required at this tier/);
  assert.deepEqual(reportRows(r), [
    '- not yet reached: 1 file(s), e.g. crosscheck/scripts/build.sh. Blocking property: no CI workflow runs a check on this path. Open question: which deterministic check this code needs, and which workflow runs it.',
  ]);
});

for (const file of ['CLAUDE.md', 'AGENTS.md', 'REVIEW.md', 'cgv/CLAUDE.md']) {
  test(`TG-8: agent and reviewer instructions in ${file} are reported as prompt text`, () => {
    const r = run(repo({ 'intent/old.md': 'x', [file]: 'x' }), { body: 'Tier: 1\nIntent: intent/old.md', changed: [file] });
    assert.equal(r.pass, true);
    assert.deepEqual(reportRows(r), [
      `- not yet reached: 1 file(s), e.g. ${file}. Blocking property: their behaviour is prompt text that an agent interprets. Open question: what a replayable behavioural eval of a prompt artefact looks like.`,
    ]);
  });
}

test('TG-8: no report line says no evidence is required', () => {
  const changed = ['README.md', 'docs/a.pdf', 'CLAUDE.md', 'docs/invariants/x.md', 'package.json'];
  const r = run(repo({ 'intent/old.md': 'x', ...Object.fromEntries(changed.map((f) => [f, 'x'])) }), {
    body: 'Tier: 1\nIntent: intent/old.md',
    changed,
  });
  assert.equal(r.pass, true);
  assert.doesNotMatch(out(r), /none required/);
  assert.ok(reportRows(r).every((line) => line.startsWith('- not yet reached: ')));
});

test('TG-9: the pass report states the merge is the human sign-off', () => {
  const r = run(repo({ 'intent/a.md': 'x' }), { body: 'Tier: 1', changed: ['intent/a.md'] });
  assert.match(out(r), /Human sign-off: the maintainer's merge/);
});

// ---- TG-10: failure message ------------------------------------------------

test('TG-10: failures carry the fixed gate message and explainer link', () => {
  const r = run(repo({ 'README.md': 'x' }), { changed: ['README.md'] });
  assert.equal(r.lines[0], '**Action needed: declare the correct tier and add missing artefacts**');
  assert.match(r.lines[1], /docs\/gates\/tier-layer-gate\.md\.$/);
});

test('missing rules file fails closed', () => {
  const dir = mkdtempSync(join(tmpdir(), 'tier-gate-test-'));
  const r = evaluate({ prBody: 'Tier: 1', changedFiles: [], cwd: dir });
  assert.equal(r.pass, false);
  assert.match(out(r), /Could not read the protected-surface glob list/);
});
