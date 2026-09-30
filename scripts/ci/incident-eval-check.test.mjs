// Tests for incident-eval-check.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (IE-*, see
// intent/2026-09-30-incident-eval-range-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), 'incident-eval-check.mjs');
const SKIPPED = 'no incident reference — skipped\n';
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};

function git(cwd, ...args) {
  return execFileSync('git', args, { cwd, env: GIT_ENV, encoding: 'utf8' }).trim();
}

let counter = 0;
function commit(cwd, message) {
  counter += 1;
  writeFileSync(join(cwd, `file-${counter}.txt`), `${counter}\n`);
  git(cwd, 'add', '.');
  git(cwd, 'commit', '-q', '-m', message);
}

// Pull request #1 is squash-merged into main. Its head is published only as
// refs/pull/1/head, as GitHub does once the branch is deleted, so a fresh
// clone does not contain it.
function squashMergedPr({ branchMessages, mergedFromMain }) {
  const root = mkdtempSync(join(tmpdir(), 'incident-eval-test-'));
  const remote = join(root, 'remote.git');
  const work = join(root, 'work');
  const checkout = join(root, 'checkout');
  git(root, 'init', '-q', '--bare', '-b', 'main', remote);
  git(root, 'clone', '-q', remote, work);
  commit(work, 'init');
  git(work, 'push', '-q', 'origin', 'main');
  git(work, 'switch', '-q', '-c', 'feat');
  if (mergedFromMain) {
    git(work, 'switch', '-q', 'main');
    commit(work, mergedFromMain);
    git(work, 'push', '-q', 'origin', 'main');
    git(work, 'switch', '-q', 'feat');
    git(work, 'merge', '-q', '--no-ff', '-m', 'Merge branch main into feat', 'main');
  }
  for (const message of branchMessages) commit(work, message);
  const head = git(work, 'rev-parse', 'HEAD');
  git(work, 'push', '-q', 'origin', 'HEAD:refs/pull/1/head');
  git(work, 'switch', '-q', 'main');
  git(work, 'merge', '-q', '--squash', 'feat');
  git(work, 'commit', '-q', '-m', 'feat: the change (#1)');
  git(work, 'push', '-q', 'origin', 'main');
  git(root, 'clone', '-q', '--no-local', remote, checkout);
  assert.throws(() => git(checkout, 'cat-file', '-e', head), 'fresh clone lacks the PR head');
  return { checkout, head };
}

function run(checkout, head) {
  return spawnSync(process.execPath, [SCRIPT], {
    cwd: checkout,
    encoding: 'utf8',
    env: { ...GIT_ENV, PR_NUMBER: '1', BASE_REF: 'main', HEAD_SHA: head, PR_BODY: '', PR_LABELS: '' },
  });
}

test('IE-1: reads an incident id from a commit of a squash-merged PR whose branch is gone', () => {
  const { checkout, head } = squashMergedPr({
    branchMessages: ['fix: first step', 'fix: second step\n\nFixes-Incident: INC-7'],
  });
  const result = run(checkout, head);
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stdout, /No eval under evals\/ names or references incident "INC-7"\./);
});

test('IE-5: a squash-merged PR with no incident reference is skipped', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, SKIPPED);
});

test('IE-1: a base-branch commit merged into the PR branch is not read', () => {
  const { checkout, head } = squashMergedPr({
    mergedFromMain: 'fix: another PR\n\nFixes-Incident: INC-9',
    branchMessages: ['fix: plain change'],
  });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, SKIPPED);
});

test('IE-2: a head the remote does not have exits 2 and is never skipped', () => {
  const { checkout } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  const result = run(checkout, '0'.repeat(40));
  assert.equal(result.status, 2);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, /^incident-eval-check: could not read the pull request's commits: git log /);
});
