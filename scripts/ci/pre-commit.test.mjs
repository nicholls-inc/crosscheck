// Tests for .husky/pre-commit and scripts/ci/pre-commit.mjs (PC-1 to PC-8,
// see intent/2026-10-06-pre-commit-hooks-spec.md). Each case makes a real
// `git commit` in a scratch clone whose core.hooksPath is .husky.
// Run: node --test scripts/ci/*.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(HERE, '../..');
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};
const COPIED = [
  '.husky/pre-commit',
  'scripts/ci/pre-commit.mjs',
  'scripts/ci/tier-gate.mjs',
  'scripts/ci/task-queue.mjs',
  '.claude/rules/protected-surfaces.md',
];
const HEADER = '| Task | Status | What | Depends on | Issue | Record |';
const SEPARATOR = '| --- | --- | --- | --- | --- | --- |';
const ROW_1 = '| PB-1.1 | done | first | | | |';
const ROW_2 = '| PB-1.2 | todo | second | PB-1.1 | | |';
const tasks = ({ row2 = ROW_2, separator = SEPARATOR } = {}) =>
  ['# Tasks', '', '## Queue', '', HEADER, separator, ROW_1, row2, ''].join('\n');
const ROADMAP = '# Roadmap\n\n**PB-1 — Status: In progress.**\n';
const PROTECTED = 'docs/assurance/x.md';
const NOTE = '.assurance/protected-surface-amend/x-2026-10-06.md';

function git(cwd, args) {
  return execFileSync('git', args, { cwd, env: GIT_ENV, encoding: 'utf8' }).trim();
}

function write(dir, files) {
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), content);
  }
}

// Files every scratch repository starts with: the hook, the scripts it
// imports, the rules file, and a minimal roadmap and queue.
function seed(dir, extra = {}) {
  for (const path of COPIED) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    copyFileSync(join(REPO_ROOT, path), join(dir, path));
  }
  chmodSync(join(dir, '.husky/pre-commit'), 0o755);
  write(dir, { 'docs/assurance/ROADMAP.md': ROADMAP, 'docs/TASKS.md': tasks(), 'README.md': 'x\n', ...extra });
}

function enableHook(dir) {
  git(dir, ['config', 'core.hooksPath', '.husky']);
}

// A clone of a scratch bare remote whose main holds `seed` plus `extra`, on a
// new branch, with the hook enabled. The setup commit runs before the hook is.
function scratchClone(extra = {}) {
  const root = mkdtempSync(join(tmpdir(), 'pre-commit-test-'));
  const remote = join(root, 'remote.git');
  const work = join(root, 'work');
  git(root, ['init', '-q', '--bare', '-b', 'main', remote]);
  git(root, ['init', '-q', '-b', 'main', work]);
  seed(work, extra);
  git(work, ['add', '.']);
  git(work, ['commit', '-q', '-m', 'base']);
  git(work, ['remote', 'add', 'origin', remote]);
  git(work, ['push', '-q', 'origin', 'main']);
  git(work, ['remote', 'set-head', 'origin', 'main']);
  enableHook(work);
  git(work, ['checkout', '-q', '-b', 'task/test']);
  return work;
}

// A repository with no remote at all.
function scratchNoOrigin() {
  const work = mkdtempSync(join(tmpdir(), 'pre-commit-test-no-origin-'));
  git(work, ['init', '-q', '-b', 'main']);
  seed(work);
  git(work, ['add', '.']);
  git(work, ['commit', '-q', '--no-verify', '-m', 'base']);
  enableHook(work);
  return work;
}

// Stage `files`, run `git commit`, and report what happened. git commit exits
// 1 for any failing hook, so a failed commit's `hook` is the exit code of the
// same command rerun by hand (PC-2); it is null when the commit went through.
function commit(work, files) {
  write(work, files);
  git(work, ['add', '--', ...Object.keys(files)]);
  const before = git(work, ['rev-parse', 'HEAD']);
  const started = Date.now();
  const r = spawnSync('git', ['commit', '-m', 'x'], { cwd: work, env: GIT_ENV, encoding: 'utf8' });
  const elapsed = Date.now() - started;
  const after = git(work, ['rev-parse', 'HEAD']);
  assert.ok(elapsed < 5000, `the commit took ${elapsed} ms, over the 5000 ms budget (PC-7)`);
  const rerun = r.status === 0 ? null : spawnSync('node', ['scripts/ci/pre-commit.mjs'], { cwd: work, env: GIT_ENV, encoding: 'utf8' });
  if (rerun) assert.equal(`${rerun.stdout}${rerun.stderr}`, `${r.stdout}${r.stderr}`.replace(/^[\s\S]*?(?=pre-commit:)/, ''), 'the rerun prints what the hook printed');
  return { status: r.status, hook: rerun && rerun.status, moved: before !== after, output: `${r.stdout}${r.stderr}` };
}

test('PC-3, PC-5: a staged protected path with no note fails and says how to fix it', () => {
  const r = commit(scratchClone(), { [PROTECTED]: 'x\n' });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.equal(r.moved, false);
  assert.match(r.output, /pre-commit: governance notes/);
  assert.match(r.output, /^- docs\/assurance\/x\.md/m);
  assert.match(r.output, /^Fix: .*\/crosscheck:protected-surface-amend/m);
  assert.match(r.output, /git add \.assurance\/protected-surface-amend\/<note>\.md/);
  assert.match(r.output, /git restore --staged docs\/assurance\/x\.md/);
  assert.match(r.output, /^Then rerun: node scripts\/ci\/pre-commit\.mjs$/m);
  assert.match(r.output, /Full explanation: docs\/gates\/tier-layer-gate\.md/);
});

test('PC-3: the same commit with a new note that names the path passes', () => {
  const r = commit(scratchClone(), { [PROTECTED]: 'x\n', [NOTE]: `Amends ${PROTECTED}\n` });
  assert.equal(r.status, 0);
  assert.equal(r.hook, null);
  assert.equal(r.moved, true);
  assert.doesNotMatch(r.output, /pre-commit:/);
});

test('PC-3: a note committed earlier on the branch counts', () => {
  const work = scratchClone();
  const first = commit(work, { [NOTE]: `Amends ${PROTECTED}\n` });
  assert.equal(first.status, 0);
  const r = commit(work, { [PROTECTED]: 'x\n' });
  assert.equal(r.status, 0);
  assert.equal(r.hook, null);
  assert.equal(r.moved, true);
});

test('PC-3: a note on the default branch, unchanged on the branch, does not count', () => {
  const r = commit(scratchClone({ [NOTE]: `Amends ${PROTECTED}\n` }), { [PROTECTED]: 'x\n' });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.equal(r.moved, false);
  assert.match(r.output, /^- docs\/assurance\/x\.md/m);
});

test('PC-3: a note that names only another path leaves the unnamed path failing', () => {
  const r = commit(scratchClone(), {
    [PROTECTED]: 'x\n',
    'docs/assurance/y.md': 'y\n',
    [NOTE]: 'Amends docs/assurance/y.md\n',
  });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.match(r.output, /^- docs\/assurance\/x\.md/m);
  assert.doesNotMatch(r.output, /^- docs\/assurance\/y\.md/m);
  assert.match(r.output, /git restore --staged docs\/assurance\/x\.md$/m);
});

test('PC-3, PC-5: a staged protected path with no origin/HEAD or origin/main exits 2 and says to fetch', () => {
  const r = commit(scratchNoOrigin(), { [PROTECTED]: 'x\n' });
  assert.equal(r.status, 1, 'git commit exits 1 whatever the hook exits with');
  assert.equal(r.hook, 2);
  assert.equal(r.moved, false);
  assert.match(r.output, /^Fix: git fetch origin$/m);
});

test('PC-3: a rules file with no machine-readable list exits 2', () => {
  const work = scratchClone();
  write(work, { '.claude/rules/protected-surfaces.md': '# no list here\n' });
  const r = commit(work, { 'README.md': 'y\n' });
  assert.equal(r.status, 1, 'git commit exits 1 whatever the hook exits with');
  assert.equal(r.hook, 2);
  assert.equal(r.moved, false);
  assert.match(r.output, /Machine-readable path list/);
  assert.match(r.output, /^Fix: restore the .* \(git restore \.claude\/rules\/protected-surfaces\.md /m, 'a Fix line names the rules file');
});

test('PC-5: a printed git restore quotes a staged path that has a space', () => {
  const r = commit(scratchClone(), { 'docs/assurance/a b.md': 'x\n' });
  assert.equal(r.status, 1);
  assert.match(r.output, /git restore --staged 'docs\/assurance\/a b\.md'$/m);
});

test('PC-4, PC-5: a staged queue with an unknown status fails and names the row', () => {
  const r = commit(scratchClone(), { 'docs/TASKS.md': tasks({ row2: '| PB-1.2 | wip | second | PB-1.1 | | |' }) });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.equal(r.moved, false);
  assert.match(r.output, /pre-commit: task queue/);
  assert.match(r.output, /^- PB-1\.2: the status "wip" is not todo, blocked or done$/m);
  assert.match(r.output, /^Fix: edit docs\/TASKS\.md.*git add docs\/TASKS\.md$/m);
  assert.match(r.output, /^Then rerun: node scripts\/ci\/pre-commit\.mjs$/m);
  assert.match(r.output, /Full explanation: docs\/gates\/task-queue-check\.md/);
});

test('PC-4: a staged queue that sets a row to done with no Task: line passes', () => {
  const r = commit(scratchClone(), { 'docs/TASKS.md': tasks({ row2: '| PB-1.2 | done | second | PB-1.1 | | |' }) });
  assert.equal(r.status, 0);
  assert.equal(r.hook, null);
  assert.equal(r.moved, true);
});

test('PC-4: a staged queue with no separator row exits 2', () => {
  const r = commit(scratchClone(), { 'docs/TASKS.md': tasks({ separator: '' }) });
  assert.equal(r.status, 1, 'git commit exits 1 whatever the hook exits with');
  assert.equal(r.hook, 2);
  assert.equal(r.moved, false);
  assert.match(r.output, /the queue table has no separator row/);
});

test('PC-4: a roadmap change is checked against the staged queue', () => {
  const r = commit(scratchClone(), {
    'docs/assurance/ROADMAP.md': '# Roadmap\n',
    [NOTE]: 'Amends docs/assurance/ROADMAP.md\n',
  });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.match(r.output, /^- PB-1\.1: PB-1 is not an item in docs\/assurance\/ROADMAP\.md$/m);
});

test('PC-2: every failing check prints its block before the commit fails', () => {
  const r = commit(scratchClone(), {
    [PROTECTED]: 'x\n',
    'docs/TASKS.md': tasks({ row2: '| PB-1.2 | wip | second | PB-1.1 | | |' }),
  });
  assert.equal(r.status, 1);
  assert.equal(r.hook, 1);
  assert.match(r.output, /pre-commit: governance notes/);
  assert.match(r.output, /pre-commit: task queue/);
});

test('PC-2: exit 2 wins over exit 1 when one check cannot read and another fails', () => {
  const r = commit(scratchNoOrigin(), {
    [PROTECTED]: 'x\n',
    'docs/TASKS.md': tasks({ row2: '| PB-1.2 | wip | second | PB-1.1 | | |' }),
  });
  assert.equal(r.status, 1, 'git commit exits 1 whatever the hook exits with');
  assert.equal(r.hook, 2);
  assert.match(r.output, /^Fix: git fetch origin$/m);
  assert.match(r.output, /^- PB-1\.2: the status "wip"/m);
});

test('PC-6: an unrelated commit in a clone with no origin passes without output', () => {
  const r = commit(scratchNoOrigin(), { 'README.md': 'y\n' });
  assert.equal(r.status, 0);
  assert.equal(r.hook, null);
  assert.equal(r.moved, true);
  assert.doesNotMatch(r.output, /pre-commit/);
});
