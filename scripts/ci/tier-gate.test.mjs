// Tests for tier-gate.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (TG-*, see
// intent/2026-09-29-deterministic-evidence-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
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
function repo(files = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'tier-gate-test-'));
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

test('TG-8: the pass report names the evidence for each class of changed file', () => {
  const files = { 'cgv/src/main.rs': 'x', 'crosscheck/mcp-server/src/a.ts': 'x', 'intent/a.md': 'x' };
  const r = run(repo(files), { body: 'Tier: 1', changed: Object.keys(files) });
  assert.equal(r.pass, true);
  assert.match(out(r), /CGV CI workflow/);
  assert.match(out(r), /CI workflow \(npm test/);
  assert.match(out(r), /none required at this tier: 1 file\(s\), e\.g\. intent\/a\.md/);
});

test('TG-8: skill edits are reported as not yet reached', () => {
  const r = run(tier3Repo(), { changed: TIER3_CHANGED });
  assert.match(out(r), /not yet reached: human review is the only evidence: 1 file\(s\), e\.g\. crosscheck\/skills\/reason\/SKILL\.md/);
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
