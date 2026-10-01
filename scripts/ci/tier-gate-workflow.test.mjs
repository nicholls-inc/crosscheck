// Tests for the `Run tier gate` step of .github/workflows/tier-gate.yml.
// Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (TG-14, TG-1a, see
// intent/2026-10-01-tier-gate-workflow-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(HERE, '../..');
const WORKFLOW = '.github/workflows/tier-gate.yml';
const STEP = 'Run tier gate';
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};
const PR_BODY = 'Tier: 1\nIntent: intent/i.md\n';

// The step's `run:` script, read by indentation so no YAML parser is needed.
function stepScript() {
  const lines = readFileSync(join(REPO_ROOT, WORKFLOW), 'utf8').split('\n');
  const start = lines.findIndex((l) => l.trim() === `- name: ${STEP}`);
  assert.notEqual(start, -1, `${WORKFLOW} has a step named "${STEP}"`);
  const stepIndent = lines[start].search(/\S/);
  for (let i = start + 1; i < lines.length; i += 1) {
    const indent = lines[i].search(/\S/);
    if (indent !== -1 && indent <= stepIndent) break;
    const m = lines[i].match(/^(\s*)run:\s*(.*)$/);
    if (!m) continue;
    if (m[2] !== '|') return m[2];
    const body = [];
    for (let j = i + 1; j < lines.length; j += 1) {
      const bodyIndent = lines[j].search(/\S/);
      if (bodyIndent !== -1 && bodyIndent <= m[1].length) break;
      body.push(lines[j]);
    }
    const pad = Math.min(...body.filter((l) => l.trim()).map((l) => l.search(/\S/)));
    return body.map((l) => l.slice(pad)).join('\n');
  }
  assert.fail(`the "${STEP}" step in ${WORKFLOW} has a run: script`);
}

function sh(cwd, args) {
  return execFileSync('git', args, { cwd, env: GIT_ENV, encoding: 'utf8' });
}

function write(dir, files) {
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), content);
  }
}

// A clone of a scratch remote whose main holds the gate, the rules file, an
// intent, and `baseFiles`, checked out on a branch that applies `change`.
function scratchPr(baseFiles, change) {
  const root = mkdtempSync(join(tmpdir(), 'tier-gate-workflow-test-'));
  const remote = join(root, 'remote.git');
  const work = join(root, 'work');
  sh(root, ['init', '-q', '--bare', '-b', 'main', remote]);
  sh(root, ['init', '-q', '-b', 'main', work]);
  for (const path of ['scripts/ci/tier-gate.mjs', '.claude/rules/protected-surfaces.md']) {
    mkdirSync(dirname(join(work, path)), { recursive: true });
    copyFileSync(join(REPO_ROOT, path), join(work, path));
  }
  write(work, { 'intent/i.md': '# Intent\n', ...baseFiles });
  sh(work, ['add', '.']);
  sh(work, ['commit', '-q', '-m', 'base']);
  sh(work, ['remote', 'add', 'origin', remote]);
  sh(work, ['push', '-q', 'origin', 'main']);
  sh(work, ['switch', '-q', '-c', 'pr']);
  change(work);
  sh(work, ['add', '-A']);
  sh(work, ['commit', '-q', '-m', 'change']);
  return work;
}

function runStep(cwd, baseRef = 'main') {
  const r = spawnSync('bash', ['--noprofile', '--norc', '-eo', 'pipefail', '-c', stepScript()], {
    cwd,
    env: { ...GIT_ENV, BASE_REF: baseRef, PR_BODY, PR_LABELS: '' },
    encoding: 'utf8',
  });
  return { code: r.status, out: r.stdout + r.stderr };
}

test('TG-14: files named EOF and a<<EOF do not hide a protected file from the gate', () => {
  const work = scratchPr({}, (dir) =>
    write(dir, { EOF: 'x\n', 'docs/assurance/a<<EOF': 'x\n', 'docs/assurance/zz.md': 'x\n' })
  );
  const { code, out } = runStep(work);
  assert.equal(code, 1, out);
  assert.match(out, /force a floor of Tier 3, but the PR declares Tier 1: .*docs\/assurance\/zz\.md/);
});

test('TG-14: a file named EOF next to an unprotected file passes at Tier 1 and names the base', () => {
  const work = scratchPr({}, (dir) => write(dir, { EOF: 'x\n', 'notes.md': 'x\n' }));
  const { code, out } = runStep(work);
  assert.equal(code, 0, out);
  assert.match(out, /tier-gate: PASS — Tier 1 artefacts present \(declared: 1, floor: none, base: main\)\./);
});

test('TG-1a: a protected file moved out of a protected path keeps the Tier 3 floor', () => {
  const work = scratchPr({ 'docs/assurance/x.md': 'x\n' }, (dir) => sh(dir, ['mv', 'docs/assurance/x.md', 'notes.md']));
  const { code, out } = runStep(work);
  assert.equal(code, 1, out);
  assert.match(out, /force a floor of Tier 3, but the PR declares Tier 1: docs\/assurance\/x\.md/);
});

test('TG-14: a base ref that names no branch fails the step before the gate runs', () => {
  const work = scratchPr({}, (dir) => write(dir, { 'notes.md': 'x\n' }));
  const { code, out } = runStep(work, 'no-such-branch');
  assert.notEqual(code, 0, out);
  assert.doesNotMatch(out, /tier-gate:/);
});

test('TG-14: a base that fetches but shares no history with the PR fails the step before the gate runs', () => {
  const work = scratchPr({}, (dir) => write(dir, { 'notes.md': 'x\n' }));
  sh(work, ['switch', '-q', '--orphan', 'unrelated']);
  write(work, { 'other.md': 'x\n' });
  sh(work, ['add', 'other.md']);
  sh(work, ['commit', '-q', '-m', 'unrelated']);
  sh(work, ['push', '-q', 'origin', 'unrelated']);
  sh(work, ['switch', '-q', 'pr']);
  const { code, out } = runStep(work, 'unrelated');
  assert.notEqual(code, 0, out);
  assert.match(out, /no merge base/);
  assert.doesNotMatch(out, /tier-gate:/);
});
