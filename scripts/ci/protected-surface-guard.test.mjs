// Tests for .claude/hooks/protected-surface-guard.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (PG-*, see
// intent/2026-09-30-merged-notes-unlock-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HOOK = join(dirname(fileURLToPath(import.meta.url)), '../../.claude/hooks/protected-surface-guard.mjs');
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};

const GATE_MESSAGE =
  '**Action needed: run /protected-surface-amend before editing**\n' +
  'You are being asked to allow this edit to a protected surface because the file is a protected surface with no governance-note block that is new on this branch. Approving means generating the block via /protected-surface-amend then re-editing; declining means the file stays unchanged. Full explanation: docs/gates/protected-surface-hook.md.\n';

const NOTE_DIR = '.assurance/protected-surface-amend';
const NOTE = `${NOTE_DIR}/old.md`;
const RULES = '# Rules\n\n## Machine-readable path list\n\n```\nprotected/**\n```\n';

function git(cwd, ...args) {
  return execFileSync('git', args, { cwd, env: GIT_ENV, encoding: 'utf8' }).trim();
}

function write(cwd, path, text) {
  mkdirSync(dirname(join(cwd, path)), { recursive: true });
  writeFileSync(join(cwd, path), text);
}

function block(...files) {
  return `## Protected-Surface Amendment\n\nTarget file(s): ${files.join(', ')}\n`;
}

function commitAll(cwd, message) {
  git(cwd, 'add', '.');
  git(cwd, 'commit', '-q', '-m', message);
}

// A bare remote whose default branch is `branch`, holding `files`, and a fresh
// clone of it. `work` is a second clone used to move the remote on.
function scratch({ branch = 'main', files = {} } = {}) {
  // realpath: the hook resolves the repo root through git, which resolves symlinks.
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'protected-surface-guard-test-')));
  const remote = join(root, 'remote.git');
  const work = join(root, 'work');
  const checkout = join(root, 'checkout');
  git(root, 'init', '-q', '--bare', '-b', branch, remote);
  git(root, 'clone', '-q', remote, work);
  git(work, 'switch', '-q', '-c', branch);
  write(work, 'protected/a.txt', 'a\n');
  write(work, 'protected/b.txt', 'b\n');
  write(work, 'README.md', 'readme\n');
  for (const [path, text] of Object.entries(files)) write(work, path, text);
  commitAll(work, 'init');
  git(work, 'push', '-q', 'origin', branch);
  git(root, 'clone', '-q', remote, checkout);
  const rules = join(root, 'rules.md');
  writeFileSync(rules, RULES);
  return { root, work, checkout, rules, branch };
}

function runHook(repo, file) {
  return spawnSync(process.execPath, [HOOK], {
    cwd: repo.checkout,
    encoding: 'utf8',
    env: { ...GIT_ENV, CROSSCHECK_PROTECTED_RULES: repo.rules },
    input: JSON.stringify({ tool_name: 'Edit', tool_input: { file_path: join(repo.checkout, file) } }),
  });
}

function assertBlocked(result) {
  assert.equal(result.status, 2);
  assert.equal(result.stderr, GATE_MESSAGE);
}

function assertAllowed(result) {
  assert.equal(result.status, 0, result.stderr);
}

test('PG-3: a note on the default branch does not unlock a file', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-2, PG-3: an untracked note unlocks the file it names', () => {
  const repo = scratch();
  write(repo.checkout, NOTE, block('protected/a.txt'));
  assertAllowed(runHook(repo, 'protected/a.txt'));
});

test('PG-2, PG-3: a staged note unlocks the file it names', () => {
  const repo = scratch();
  write(repo.checkout, NOTE, block('protected/a.txt'));
  git(repo.checkout, 'add', NOTE);
  assertAllowed(runHook(repo, 'protected/a.txt'));
});

test('PG-2, PG-3: a note committed on the branch but not pushed unlocks the file', () => {
  const repo = scratch();
  git(repo.checkout, 'switch', '-q', '-c', 'feat');
  write(repo.checkout, NOTE, block('protected/a.txt'));
  commitAll(repo.checkout, 'note');
  assertAllowed(runHook(repo, 'protected/a.txt'));
});

test('PG-2: a new block appended to a note on the default branch unlocks only what it names', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  write(repo.checkout, NOTE, `${block('protected/a.txt')}\n${block('protected/b.txt')}`);
  assertAllowed(runHook(repo, 'protected/b.txt'));
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-2: a merged block copied to a new note file does not unlock a file', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  write(repo.checkout, `${NOTE_DIR}/copy.md`, block('protected/a.txt'));
  write(repo.checkout, '.assurance/add-session-x/copy.md', block('protected/a.txt'));
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-2: when git cannot read a note on the default branch, a new note does not unlock a file', () => {
  const repo = scratch();
  // A submodule entry at a note path: git lists it, and git show fails on it.
  git(repo.work, 'update-index', '--add', '--cacheinfo', `160000,${'1'.repeat(40)},${NOTE_DIR}/sub.md`);
  git(repo.work, 'commit', '-q', '-m', 'gitlink');
  git(repo.work, 'push', '-q', 'origin', repo.branch);
  git(repo.checkout, 'fetch', '-q', 'origin');
  write(repo.checkout, `${NOTE_DIR}/new.md`, block('protected/a.txt'));
  const blocked = runHook(repo, 'protected/a.txt');
  assert.equal(blocked.status, 2);
  assert.match(blocked.stderr, /could not list or read the governance notes on the default branch/);
});

test('PG-2: a CRLF checkout of a merged note does not unlock a file', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  write(repo.checkout, NOTE, block('protected/a.txt').replace(/\n/g, '\r\n'));
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-3: editing a merged block unlocks only the paths the edit adds', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  write(repo.checkout, NOTE, block('protected/a.txt', 'protected/b.txt'));
  assertAllowed(runHook(repo, 'protected/b.txt'));
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-3: a path a note file already named on the default branch needs a new note file', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  write(repo.checkout, NOTE, `${block('protected/a.txt')}\n${block('protected/a.txt', 'again')}`);
  assertBlocked(runHook(repo, 'protected/a.txt'));
  write(repo.checkout, `${NOTE_DIR}/new.md`, block('protected/a.txt', 'again'));
  assertAllowed(runHook(repo, 'protected/a.txt'));
});

test('PG-2: a squash-merged note stops counting once the default branch is fetched', () => {
  const repo = scratch();
  git(repo.checkout, 'switch', '-q', '-c', 'feat');
  write(repo.checkout, NOTE, block('protected/a.txt'));
  commitAll(repo.checkout, 'note');
  assertAllowed(runHook(repo, 'protected/a.txt'));
  // The squash commit on main carries the same note file content.
  write(repo.work, NOTE, block('protected/a.txt'));
  commitAll(repo.work, 'feat (#1)');
  git(repo.work, 'push', '-q', 'origin', 'main');
  git(repo.checkout, 'fetch', '-q', 'origin');
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-1: a remote whose default branch is trunk, with no main, is read', () => {
  const repo = scratch({ branch: 'trunk', files: { [NOTE]: block('protected/a.txt') } });
  assert.equal(git(repo.checkout, 'rev-parse', '--abbrev-ref', 'origin/HEAD'), 'origin/trunk');
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-1: with origin/HEAD unset, origin/main is the default branch', () => {
  const repo = scratch({ files: { [NOTE]: block('protected/a.txt') } });
  git(repo.checkout, 'remote', 'set-head', 'origin', '-d');
  assert.throws(() => git(repo.checkout, 'rev-parse', '--verify', '--quiet', 'origin/HEAD'));
  assertBlocked(runHook(repo, 'protected/a.txt'));
  write(repo.checkout, NOTE, `${block('protected/a.txt')}\n${block('protected/b.txt')}`);
  assertAllowed(runHook(repo, 'protected/b.txt'));
});

test('PG-2: a merged note larger than 1 MiB does not unlock a file', () => {
  const big = `${block('protected/a.txt')}\n${'x'.repeat(2 * 1024 * 1024)}\n`;
  const repo = scratch({ files: { [NOTE]: big } });
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-6: a new note under .assurance/add-session-<name>/ unlocks the file', () => {
  const repo = scratch();
  write(repo.checkout, '.assurance/add-session-x/note.md', block('protected/a.txt'));
  assertAllowed(runHook(repo, 'protected/a.txt'));
});

test('PG-2, PG-6: a note under .assurance/add-session-<name>/ on the default branch does not unlock a file', () => {
  const repo = scratch({ files: { '.assurance/add-session-x/note.md': block('protected/a.txt') } });
  assertBlocked(runHook(repo, 'protected/a.txt'));
});

test('PG-4: with no remote, a protected file is blocked and stderr names the fix', () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'protected-surface-guard-test-')));
  const checkout = join(root, 'checkout');
  mkdirSync(checkout);
  git(checkout, 'init', '-q', '-b', 'main');
  write(checkout, 'protected/a.txt', 'a\n');
  write(checkout, 'README.md', 'readme\n');
  write(checkout, NOTE, block('protected/a.txt'));
  commitAll(checkout, 'init');
  const rules = join(root, 'rules.md');
  writeFileSync(rules, RULES);
  const repo = { checkout, rules };
  const blocked = runHook(repo, 'protected/a.txt');
  assert.equal(blocked.status, 2);
  assert.match(blocked.stderr, /origin\/HEAD/);
  assert.match(blocked.stderr, /origin\/main/);
  assert.match(blocked.stderr, /git fetch origin/);
  assert.match(blocked.stderr, /git remote set-head origin --auto/);
  assertAllowed(runHook(repo, 'README.md'));
});

test('PG-6: an unprotected file with no note is allowed', () => {
  const repo = scratch();
  assertAllowed(runHook(repo, 'README.md'));
});

test('PG-9: ** matches a file name that holds a newline', () => {
  const repo = scratch();
  assertBlocked(runHook(repo, 'protected/n\nl.txt'));
});

test('PG-9: ** matches a directory name and a file below it that hold a newline', () => {
  const repo = scratch();
  assertBlocked(runHook(repo, 'protected/x\ny/n\nl.txt'));
});

test('PG-9: a new note that names a path with a newline unlocks it', () => {
  const repo = scratch();
  assertBlocked(runHook(repo, 'protected/n\nl.txt'));
  write(repo.checkout, NOTE, block('protected/n\nl.txt'));
  assertAllowed(runHook(repo, 'protected/n\nl.txt'));
});

test('PG-5: the hook source runs git without a shell', () => {
  const source = readFileSync(HOOK, 'utf8');
  assert.ok(!source.includes('execSync'));
  assert.ok(!source.includes('exec('));
  assert.ok(!source.includes('shell: true'));
});
