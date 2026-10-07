// Tests for .husky/commit-msg. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (CM-*, see
// intent/2026-10-06-commit-msg-names-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HOOK = join(dirname(fileURLToPath(import.meta.url)), '../../.husky/commit-msg');
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};

// Stages `path` in a fresh repository and runs the hook on `message`. The
// stubbed npx keeps commitlint out of the run, so only the type check decides.
function runHook(path, message) {
  const root = mkdtempSync(join(tmpdir(), 'commit-msg-test-'));
  const bin = join(root, 'bin');
  const repo = join(root, 'repo');
  mkdirSync(bin);
  writeFileSync(join(bin, 'npx'), '#!/bin/sh\nexit 0\n');
  chmodSync(join(bin, 'npx'), 0o755);
  mkdirSync(repo);
  execFileSync('git', ['init', '-q'], { cwd: repo, env: GIT_ENV });
  mkdirSync(dirname(join(repo, path)), { recursive: true });
  writeFileSync(join(repo, path), 'x\n');
  execFileSync('git', ['add', '-A'], { cwd: repo, env: GIT_ENV });
  writeFileSync(join(root, 'msg'), `${message}\n`);
  const result = spawnSync('sh', [HOOK, join(root, 'msg')], {
    cwd: repo,
    env: { ...GIT_ENV, PATH: `${bin}:${process.env.PATH}` },
    encoding: 'utf8',
  });
  return { status: result.status, output: result.stdout + result.stderr };
}

const BLOCKED = 'ERROR: commits touching behavioral artifacts must not use the';

for (const path of [
  'crosscheck/skills/x/SKILL.md',
  'crosscheck/skills/é/SKILL.md',
  'crosscheck/agents/a"b.md',
  'crosscheck/skills/n\nl/SKILL.md',
  'crosscheck/agents/x\ny.md',
  'crosscheck/skills/a\\nb/SKILL.md',
]) {
  test(`CM-2, CM-4: docs: blocks ${JSON.stringify(path)} and names it`, () => {
    const { status, output } = runHook(path, 'docs: x');
    assert.equal(status, 1, output);
    assert.ok(output.includes(`${BLOCKED} 'docs:' prefix.`), output);
    const listed = path.split('\n').map((line) => `    ${line}`).join('\n');
    assert.ok(output.includes(listed), output);
  });
}

test('CM-1: refactor(scope): blocks a quoted SKILL.md name', () => {
  const { status, output } = runHook('crosscheck/skills/é/SKILL.md', 'refactor(crosscheck): x');
  assert.equal(status, 1, output);
  assert.ok(output.includes(`${BLOCKED} 'refactor:' prefix.`), output);
});

test('CM-1: fix: passes with a quoted SKILL.md name', () => {
  const { status, output } = runHook('crosscheck/skills/é/SKILL.md', 'fix(crosscheck): x');
  assert.equal(status, 0, output);
  assert.equal(output, '');
});

for (const path of ['docs/é.md', 'crosscheck/skills/SKILL.md\nx', 'crosscheck/agents/x.txt']) {
  test(`CM-2: docs: passes for ${JSON.stringify(path)}, which is not behavioural`, () => {
    const { status, output } = runHook(path, 'docs: x');
    assert.equal(status, 0, output);
    assert.equal(output, '');
  });
}
