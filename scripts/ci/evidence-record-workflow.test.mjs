// Tests for the `Test the evidence record checker` step of
// .github/workflows/evidence-record.yml.
// Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (EC-1, see
// intent/2026-10-06-evidence-record-ci-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(HERE, '../..');
const WORKFLOW = '.github/workflows/evidence-record.yml';
const STEP = 'Test the evidence record checker';
const CHECKER = 'scripts/check-evidence-record.mjs';
const CHECKER_TESTS = 'scripts/check-evidence-record.test.mjs';

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

function scratchCopy(checkerSource) {
  const dir = mkdtempSync(join(tmpdir(), 'evidence-record-workflow-test-'));
  mkdirSync(join(dir, 'scripts'));
  writeFileSync(join(dir, CHECKER), checkerSource);
  writeFileSync(join(dir, CHECKER_TESTS), readFileSync(join(REPO_ROOT, CHECKER_TESTS)));
  return dir;
}

function runStep(cwd) {
  // A nested `node --test` that inherits NODE_TEST_CONTEXT reports to this
  // runner instead of printing its own summary, so the child must not see it.
  // Node 24 defaults to the spec reporter even off a TTY; TAP gives stable
  // `# pass N` / `# fail N` summary lines to assert on.
  const { NODE_TEST_CONTEXT, ...env } = process.env;
  env.NODE_OPTIONS = [env.NODE_OPTIONS, '--test-reporter=tap'].filter(Boolean).join(' ');
  const r = spawnSync('bash', ['-e', '-c', stepScript()], { cwd, env, encoding: 'utf8' });
  return { code: r.status, out: r.stdout + r.stderr };
}

test('EC-1: the step runs the committed checker tests and they pass', () => {
  const { code, out } = runStep(scratchCopy(readFileSync(join(REPO_ROOT, CHECKER), 'utf8')));
  assert.equal(code, 0, out);
  const pass = out.match(/^# pass (\d+)$/m);
  assert.ok(pass && Number(pass[1]) > 0, `TAP output reports passing tests:\n${out}`);
  assert.match(out, /^# fail 0$/m);
});

test('EC-1: the step fails when checkRecord accepts everything', () => {
  const signature = 'export function checkRecord(value) {';
  const source = readFileSync(join(REPO_ROOT, CHECKER), 'utf8');
  assert.ok(source.includes(signature), `${CHECKER} defines ${signature}`);
  const { code, out } = runStep(scratchCopy(source.replace(signature, `${signature}\n  return [];`)));
  assert.notEqual(code, 0, out);
});
