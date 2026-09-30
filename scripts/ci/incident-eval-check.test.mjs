// Tests for incident-eval-check.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (IE-*, see
// intent/2026-09-30-incident-eval-range-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), 'incident-eval-check.mjs');
const WORKFLOW = join(dirname(fileURLToPath(import.meta.url)), '../../.github/workflows/incident-eval-check.yml');
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

function run(checkout, head, overrides = {}) {
  return spawnSync(process.execPath, [SCRIPT], {
    cwd: checkout,
    encoding: 'utf8',
    env: { ...GIT_ENV, PR_NUMBER: '1', BASE_REF: 'main', HEAD_SHA: head, PR_BODY: '', PR_LABELS: '', ...overrides },
  });
}

// The failure report the script prints, with the link it builds for a remote
// that is not on GitHub.
function failureOutput(items) {
  return [
    '**Action needed: add an eval and candidate invariant**',
    'You are being asked to add a regression eval and a candidate invariant for this incident because every production incident must leave both artefacts in the suite. Approving means the incident becomes a permanent regression check and a documented invariant; declining means the change stays blocked until both artefacts are added. Full explanation: docs/gates/README.md.',
    '',
    ...items.map((item) => `- ${item}`),
    '',
  ].join('\n');
}

const noEval = (id) => `No eval under evals/ names or references incident "${id}". Add one so this incident stays a permanent regression test.`;
const noInvariant = (id) => `No candidate invariant under docs/invariants/ or crosscheck/docs/invariants/ references incident "${id}". Add or amend one to capture what the incident revealed.`;
const passed = (id) => `incident-eval-check: PASS — incident "${id}" has an eval and a candidate invariant.\n`;

function addFiles(checkout, files) {
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(checkout, path)), { recursive: true });
    writeFileSync(join(checkout, path), content);
  }
}

const READ_FAILED = /^incident-eval-check: could not read the pull request's commits: /;

function assertExit2(result, pattern) {
  assert.equal(result.status, 2, result.stderr);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, READ_FAILED);
  assert.match(result.stderr, pattern);
}

test('IE-1: reads an incident id from the oldest of several commits of a squash-merged PR whose branch is gone', () => {
  const { checkout, head } = squashMergedPr({
    branchMessages: ['fix: first step\n\nFixes-Incident: INC-7', 'fix: second step', 'fix: third step'],
  });
  const result = run(checkout, head);
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noEval('INC-7'), noInvariant('INC-7')]));
});

test('IE-5: a squash-merged PR with no incident reference is skipped', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, SKIPPED);
});

test('IE-5: an incident line in the PR body applies the check', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  const result = run(checkout, head, { PR_BODY: 'Why\n\nFixes-Incident: INC-8' });
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noEval('INC-8'), noInvariant('INC-8')]));
});

test('IE-5: the body is read before the commits, and trailing punctuation is dropped', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nFixes-Incident: INC-9'] });
  const result = run(checkout, head, { PR_BODY: 'Fixes-Incident: INC-8.' });
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noEval('INC-8'), noInvariant('INC-8')]));
});

test('IE-5: the incident line is matched without regard to case', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nfixes-incident: inc-3'] });
  const result = run(checkout, head);
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noEval('inc-3'), noInvariant('inc-3')]));
});

test('IE-5: an eval named for the incident and an invariant citing it pass', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nFixes-Incident: INC-7'] });
  addFiles(checkout, { 'evals/INC-7.yaml': 'case: x\n', 'docs/invariants/m.md': 'Anchored in INC-7.\n' });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, passed('INC-7'));
});

test('IE-5: an eval citing the incident and an invariant under crosscheck/docs pass', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nFixes-Incident: INC-7'] });
  addFiles(checkout, { 'evals/regression.yaml': 'incident: INC-7\n', 'crosscheck/docs/invariants/m.md': 'Anchored in INC-7.\n' });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, passed('INC-7'));
});

test('IE-5: an eval without an invariant fails, and files outside evals/ and the invariant dirs do not count', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nFixes-Incident: INC-7'] });
  addFiles(checkout, { 'evals/INC-7.yaml': 'incident: INC-7\n', 'docs/INC-7.md': 'INC-7\n' });
  const result = run(checkout, head);
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noInvariant('INC-7')]));
});

test('IE-5: an invariant without an eval fails, and a file outside evals/ named for the incident does not count', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: a\n\nFixes-Incident: INC-7'] });
  addFiles(checkout, { 'docs/invariants/m.md': 'Anchored in INC-7.\n', 'other/INC-7.md': 'INC-7\n' });
  const result = run(checkout, head);
  assert.equal(result.status, 1, result.stderr);
  assert.equal(result.stdout, failureOutput([noEval('INC-7')]));
});

test('IE-5: the incident label with no incident id fails', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  const result = run(checkout, head, { PR_LABELS: 'bug, Incident ' });
  assert.equal(result.status, 1, result.stderr);
  assert.equal(
    result.stdout,
    failureOutput(['The "incident" label is set but no incident id was found. Add a "Fixes-Incident: <id>" line to the PR body or a commit message.'])
  );
});

test('IE-5: an incident line with no value does not take the next line as the id', () => {
  const { checkout, head } = squashMergedPr({
    branchMessages: ['fix: plain change\n\nFixes-Incident:\nSigned-off-by: someone'],
  });
  const result = run(checkout, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, SKIPPED);
});

test('IE-7: a base-branch commit merged into the PR branch is not read', () => {
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
  assertExit2(result, /git log .*\nfatal: /);
});

test('IE-2: a PR number the remote has no head ref for exits 2', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  assertExit2(run(checkout, head, { PR_NUMBER: '2' }), /git fetch .*refs\/pull\/2\/head.*\nfatal: /);
});

test('IE-2: malformed inputs exit 2 and name the inputs', () => {
  const { checkout, head } = squashMergedPr({ branchMessages: ['fix: plain change'] });
  for (const overrides of [{ PR_NUMBER: '' }, { PR_NUMBER: '1;x' }, { HEAD_SHA: 'abc' }, { BASE_REF: '' }]) {
    assertExit2(run(checkout, head, overrides), /PR_NUMBER, BASE_REF and HEAD_SHA must be set/);
  }
});

test('IE-2: a head already on the base branch leaves an empty range and exits 2', () => {
  const { checkout } = squashMergedPr({ branchMessages: ['fix: plain change\n\nFixes-Incident: INC-7'] });
  const onBase = git(checkout, 'rev-parse', 'origin/main~1');
  assertExit2(run(checkout, onBase), /no commits in origin\/main\.\./);
});

test('IE-3: git runs without a shell', () => {
  const source = readFileSync(SCRIPT, 'utf8');
  assert.doesNotMatch(source, /\bexecSync\b|\bexec\(|shell:\s*true/);
});

test('IE-4: the workflow passes inputs as env and interpolates nothing into run:', () => {
  const lines = readFileSync(WORKFLOW, 'utf8').split('\n');
  const runLines = lines.filter((l) => /^\s*run:/.test(l));
  assert.deepEqual(runLines.map((l) => l.trim()), ['run: node scripts/ci/incident-eval-check.mjs']);
  assert.doesNotMatch(lines.join('\n'), /GITHUB_OUTPUT/);
  for (const name of ['PR_NUMBER', 'BASE_REF', 'HEAD_SHA', 'PR_BODY', 'PR_LABELS']) {
    assert.ok(lines.some((l) => new RegExp(`^\\s+${name}: \\$\\{\\{`).test(l)), `env ${name}`);
  }
});
