// Tests for task-queue.mjs. Run: node --test scripts/ci/*.test.mjs
// Each test names the requirement it covers (QP-*, NX-*, QC-*, TT-*, see
// intent/2026-10-01-queue-check-spec.md).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkQueue, nextTask, parseItemIds, parseQueue, parseTaskLine } from './task-queue.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, 'task-queue.mjs');
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

const HEADER = '| Task | Status | What | Depends on | Issue | Record |\n|---|---|---|---|---|---|\n';

// rows: [id, status, what, dependsOn]
function queueMd(rows) {
  const body = rows.map(([id, status, what, deps = '']) => `| ${id} | ${status} | ${what} | ${deps} | | |`);
  return `# Task queue\n\n## Columns\n\n- Task. Not a table.\n\n## Queue\n\n${HEADER}${body.join('\n')}\n`;
}

const ROADMAP = '**PB-1 — Status: In progress.** x\n\n**ER-1 — Status: Not started.** y\n';
const ITEMS = new Set(['PB-1', 'ER-1']);

function check(rowsMd, { baseMd = rowsMd, prBody = '' } = {}) {
  return checkQueue({
    rows: parseQueue(rowsMd),
    baseRows: parseQueue(baseMd) ?? [],
    itemIds: ITEMS,
    prBody,
  });
}

// ---- QP: reading the files -------------------------------------------------

test('QP-1: cells split on an unescaped pipe and are trimmed', () => {
  const md = `## Queue\n\n${HEADER}| PB-1.1 | blocked |  needs a \\| b  | PB-1.0 | #5 | \`intent/x.md\` |\n`;
  assert.deepEqual(parseQueue(md), [
    { id: 'PB-1.1', status: 'blocked', what: 'needs a | b', dependsOn: ['PB-1.0'], issue: '#5', record: '`intent/x.md`' },
  ]);
});

test('QP-1: the queue is the table after the Queue heading, not an earlier table', () => {
  const md = `| a | b |\n|---|---|\n| 1 | 2 |\n\n## Queue\n\n${HEADER}| PB-1.1 | todo | w | | | |\n`;
  assert.deepEqual(parseQueue(md).map((r) => r.id), ['PB-1.1']);
});

test('QP-1: a missing heading or a different header means no queue', () => {
  assert.equal(parseQueue(`${HEADER}| PB-1.1 | todo | w | | | |\n`), null);
  assert.equal(parseQueue('## Queue\n\n| Task | Status |\n|---|---|\n| PB-1.1 | todo |\n'), null);
});

test('QP-2: Depends on is a comma-separated list, backticks ignored, empty means none', () => {
  const rows = parseQueue(queueMd([['PB-1.1', 'todo', 'w', '`PB-1.2`, ER-1.1 ,'], ['PB-1.2', 'todo', 'w']]));
  assert.deepEqual(rows.map((r) => r.dependsOn), [['PB-1.2', 'ER-1.1'], []]);
});

test('QP-3: a roadmap item is a line that starts "**<ID> — Status:"', () => {
  const md = '**PB-1 — Status: In progress.**\nSee **XX-9 — Status: Done** in prose.\n**ER-1 — Status: Not started.**\n';
  assert.deepEqual([...parseItemIds(md)], ['PB-1', 'ER-1']);
});

// ---- NX: next ----------------------------------------------------------------

test('NX-1: the first todo row with every dependency done and no claim is chosen', () => {
  const rows = parseQueue(
    queueMd([
      ['T-1.1', 'done', 'a'],
      ['T-1.2', 'todo', 'claimed row'],
      ['T-1.3', 'todo', 'waits', 'T-1.4'],
      ['T-1.4', 'blocked', 'b'],
      ['T-1.5', 'todo', 'ready', 'T-1.1'],
      ['T-1.6', 'todo', 'also ready'],
    ])
  );
  assert.deepEqual(nextTask(rows, ['T-1.2']), { id: 'T-1.5' });
});

test('NX-2: with no row ready, every row that is not done gets a reason, in table order', () => {
  const rows = parseQueue(
    queueMd([
      ['T-1.1', 'done', 'a'],
      ['T-1.2', 'todo', 'claimed row'],
      ['T-1.3', 'todo', 'waits', 'T-1.1, T-1.4, T-9.9'],
      ['T-1.4', 'blocked', 'needs a decision'],
    ])
  );
  assert.deepEqual(nextTask(rows, ['T-1.2']), {
    reasons: [
      { id: 'T-1.2', reason: 'claimed by task/T-1.2' },
      { id: 'T-1.3', reason: 'waiting on T-1.4, T-9.9' },
      { id: 'T-1.4', reason: 'blocked: needs a decision' },
    ],
  });
});

// ---- QC: check ---------------------------------------------------------------

test('QC-1: a task ID must be <item ID>.<n> and name a roadmap item', () => {
  const md = queueMd([
    ['PB-1.1', 'todo', 'ok'],
    ['XX-1.1', 'todo', 'unknown item'],
    ['PB-1', 'todo', 'no number'],
    ['PB-1.0', 'todo', 'zero'],
    ['pb-1.2', 'todo', 'lowercase'],
  ]);
  assert.deepEqual(check(md), [
    'XX-1.1: XX-1 is not an item in docs/assurance/ROADMAP.md',
    'PB-1: the task ID is not of the form <item ID>.<n>, such as PB-1.6',
    'PB-1.0: the task ID is not of the form <item ID>.<n>, such as PB-1.6',
    'pb-1.2: the task ID is not of the form <item ID>.<n>, such as PB-1.6',
  ]);
});

test('QC-2: two rows with one task ID fail, naming the ID', () => {
  const md = queueMd([['PB-1.1', 'todo', 'a'], ['PB-1.1', 'todo', 'b']]);
  assert.deepEqual(check(md), ['PB-1.1: the queue has more than one row with this task ID']);
});

test('QC-3: a status other than todo, blocked or done fails, naming the row', () => {
  const md = queueMd([['PB-1.1', 'in progress', 'a'], ['PB-1.2', 'Todo', 'b']]);
  assert.deepEqual(check(md), [
    'PB-1.1: the status "in progress" is not todo, blocked or done',
    'PB-1.2: the status "Todo" is not todo, blocked or done',
  ]);
});

test('QC-4: a dependency that is not a row fails, naming the row', () => {
  const md = queueMd([['PB-1.1', 'todo', 'a', 'PB-1.2, PB-1.9'], ['PB-1.2', 'todo', 'b']]);
  assert.deepEqual(check(md), ['PB-1.1: depends on PB-1.9, which is not a row of the queue']);
});

test('QC-5: a row that becomes done passes when the Task: line names it', () => {
  const base = queueMd([['PB-1.1', 'done', 'a'], ['PB-1.2', 'todo', 'b']]);
  const head = queueMd([['PB-1.1', 'done', 'a'], ['PB-1.2', 'done', 'b']]);
  assert.deepEqual(check(head, { baseMd: base, prBody: 'Intro\n\nTask: PB-1.2\nTier: 3' }), []);
});

test('QC-5: a row that becomes done fails when the PR body has no Task: line', () => {
  const base = queueMd([['PB-1.1', 'todo', 'a']]);
  const head = queueMd([['PB-1.1', 'done', 'a']]);
  assert.deepEqual(check(head, { baseMd: base }), [
    'PB-1.1: the row is newly done, but the PR body has no "Task:" line. Only the row named in the "Task:" line may become done',
  ]);
});

test('QC-5: a row that becomes done fails when the Task: line names another row', () => {
  const base = queueMd([['PB-1.1', 'todo', 'a'], ['PB-1.2', 'todo', 'b']]);
  const head = queueMd([['PB-1.1', 'done', 'a'], ['PB-1.2', 'todo', 'b']]);
  assert.deepEqual(check(head, { baseMd: base, prBody: 'Task: PB-1.2' }), [
    'PB-1.1: the row is newly done, but the PR body has the "Task:" line "PB-1.2". Only the row named in the "Task:" line may become done',
  ]);
});

test('QC-5: a "- Task:" list item or a "> Task:" quote does not count', () => {
  const base = queueMd([['PB-1.1', 'todo', 'a']]);
  const head = queueMd([['PB-1.1', 'done', 'a']]);
  for (const prBody of ['- Task: PB-1.1', '> Task: PB-1.1', '* Task: PB-1.1']) {
    assert.equal(check(head, { baseMd: base, prBody }).length, 1, prBody);
  }
  assert.equal(parseTaskLine('  task:  PB-1.1  \r\nmore'), 'PB-1.1');
  assert.equal(parseTaskLine('see Task: PB-1.1'), null);
  assert.equal(parseTaskLine('Task: A-1.1\nTask: B-1.1'), 'A-1.1');
});

test('QC-5: a row already done at the base is not newly done', () => {
  const md = queueMd([['PB-1.1', 'done', 'a']]);
  assert.deepEqual(check(md, { prBody: '' }), []);
});

test('QC-5: with no queue at the base, every done row is newly done', () => {
  const head = queueMd([['PB-1.1', 'done', 'a'], ['PB-1.2', 'done', 'b']]);
  const problems = checkQueue({
    rows: parseQueue(head),
    baseRows: [],
    itemIds: ITEMS,
    prBody: 'Task: PB-1.2',
  });
  assert.deepEqual(problems, [
    'PB-1.1: the row is newly done, but the PR body has the "Task:" line "PB-1.2". Only the row named in the "Task:" line may become done',
  ]);
});

// ---- TT-1: the real queue, and check end to end ------------------------------

test('TT-1: check passes on the queue as committed', () => {
  const rows = parseQueue(readFileSync(join(REPO_ROOT, 'docs/TASKS.md'), 'utf8'));
  const itemIds = parseItemIds(readFileSync(join(REPO_ROOT, 'docs/assurance/ROADMAP.md'), 'utf8'));
  assert.equal(rows.find((r) => r.id === 'PB-1.1').status, 'done');
  assert.deepEqual(checkQueue({ rows, baseRows: rows, itemIds, prBody: '' }), []);
});

function sh(cwd, cmd, args) {
  return execFileSync(cmd, args, { cwd, env: GIT_ENV, encoding: 'utf8' }).trim();
}

function write(dir, files) {
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), content);
  }
}

// A bare remote whose main holds `files`, plus a function that clones it.
function scratchRemote(files) {
  const root = mkdtempSync(join(tmpdir(), 'task-queue-test-'));
  const remote = join(root, 'remote.git');
  const seed = join(root, 'seed');
  sh(root, 'git', ['init', '-q', '--bare', '-b', 'main', remote]);
  sh(root, 'git', ['init', '-q', '-b', 'main', seed]);
  write(seed, files);
  sh(seed, 'git', ['add', '.']);
  sh(seed, 'git', ['commit', '-q', '-m', 'init']);
  sh(seed, 'git', ['push', '-q', remote, 'main']);
  let n = 0;
  const clone = () => {
    n += 1;
    const dir = join(root, `clone-${n}`);
    sh(root, 'git', ['clone', '-q', remote, dir]);
    return dir;
  };
  return { root, remote, seed, clone };
}

function runScript(cwd, args, env = {}) {
  const r = spawnSync('node', [SCRIPT, ...args], { cwd, env: { ...GIT_ENV, ...env }, encoding: 'utf8' });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

test('TT-1: check, run as a script in a clone, reads the base from origin and sets exit codes', () => {
  const base = queueMd([['PB-1.1', 'todo', 'a'], ['PB-1.2', 'todo', 'b']]);
  const { clone } = scratchRemote({ 'docs/TASKS.md': base, 'docs/assurance/ROADMAP.md': ROADMAP });
  const work = clone();

  assert.deepEqual(runScript(work, ['check'], { BASE_REF: 'main', PR_BODY: '' }), {
    code: 0,
    out: 'task-queue: PASS - 2 row(s) checked, 0 newly done.\n',
    err: '',
  });

  write(work, { 'docs/TASKS.md': queueMd([['PB-1.1', 'done', 'a'], ['PB-1.2', 'todo', 'b']]) });
  const passing = runScript(work, ['check'], { BASE_REF: 'main', PR_BODY: 'Task: PB-1.1' });
  assert.deepEqual(passing, {
    code: 0,
    out: 'task-queue: PASS - 2 row(s) checked, 1 newly done.\n',
    err: '',
  });

  const failing = runScript(work, ['check'], { BASE_REF: 'main', PR_BODY: '' });
  assert.equal(failing.code, 1);
  assert.deepEqual(failing.out.split('\n'), [
    '**Action needed: fix the task queue**',
    'You are being asked to correct docs/TASKS.md, or the "Task:" line of the pull request body, because the queue breaks a rule of the pick-up procedure.',
    'Approving means the pick-up procedure reads a valid queue and only the pull request that does a task marks it done; declining leaves this check red.',
    'Full explanation: docs/gates/task-queue-check.md.',
    '',
    '- PB-1.1: the row is newly done, but the PR body has no "Task:" line. Only the row named in the "Task:" line may become done',
    '',
  ]);
});

test('QC-6: the Full explanation link names the gate explainer on the origin repository', () => {
  const { clone } = scratchRemote({
    'docs/TASKS.md': queueMd([['PB-1.1', 'todo', 'a']]),
    'docs/assurance/ROADMAP.md': ROADMAP,
  });
  const work = clone();
  sh(work, 'git', ['remote', 'set-url', 'origin', 'git@github.com:some-owner/some-repo.git']);
  write(work, { 'docs/TASKS.md': queueMd([['XX-1.1', 'todo', 'a']]) });
  const r = runScript(work, ['check'], { BASE_REF: 'main' });
  assert.equal(r.code, 1);
  assert.ok(
    r.out.includes('Full explanation: https://github.com/some-owner/some-repo/blob/main/docs/gates/task-queue-check.md.'),
    r.out
  );
});

test('TT-1: check exits 2 when BASE_REF is unset or its queue cannot be read', () => {
  const { clone } = scratchRemote({
    'docs/TASKS.md': queueMd([['PB-1.1', 'todo', 'a']]),
    'docs/assurance/ROADMAP.md': ROADMAP,
  });
  const work = clone();
  const unset = runScript(work, ['check'], { BASE_REF: '' });
  assert.equal(unset.code, 2);
  assert.equal(unset.err, 'BASE_REF is not set\n');
  assert.equal(runScript(work, ['check'], { BASE_REF: 'no-such-branch' }).code, 2);
});

test('QC-5: when the base branch has no docs/TASKS.md, every done row is newly done', () => {
  const { clone } = scratchRemote({ 'README.md': 'x', 'docs/assurance/ROADMAP.md': ROADMAP });
  const work = clone();
  write(work, { 'docs/TASKS.md': queueMd([['PB-1.1', 'done', 'a']]) });
  const withoutTask = runScript(work, ['check'], { BASE_REF: 'main', PR_BODY: '' });
  assert.equal(withoutTask.code, 1);
  assert.ok(withoutTask.out.includes('- PB-1.1: the row is newly done'), withoutTask.out);
  const withTask = runScript(work, ['check'], { BASE_REF: 'main', PR_BODY: 'Task: PB-1.1' });
  assert.equal(withTask.code, 0);
});

test('QP-1: check fails with exit 1 when docs/TASKS.md has no queue', () => {
  const { clone } = scratchRemote({ 'docs/TASKS.md': queueMd([['PB-1.1', 'todo', 'a']]), 'docs/assurance/ROADMAP.md': ROADMAP });
  const work = clone();
  write(work, { 'docs/TASKS.md': '# Task queue\n' });
  const r = runScript(work, ['check'], { BASE_REF: 'main' });
  assert.equal(r.code, 1);
  assert.ok(r.out.includes('- docs/TASKS.md has no queue:'), r.out);
});

// ---- TT-2: next against a scratch remote -------------------------------------

function pushBranch(work, name) {
  sh(work, 'git', ['push', '-q', 'origin', `HEAD:refs/heads/${name}`]);
}

test('TT-2: next skips a claimed row, a row waiting on a dependency and a blocked row, and prints the first ready one', () => {
  const { clone } = scratchRemote({
    'docs/TASKS.md': queueMd([
      ['T-1.1', 'done', 'finished'],
      ['T-1.2', 'todo', 'claimed', 'T-1.1'],
      ['T-1.3', 'todo', 'waits', 'T-1.5'],
      ['T-1.4', 'blocked', 'needs a decision'],
      ['T-1.5', 'todo', 'the first ready row', 'T-1.1'],
      ['T-1.6', 'todo', 'also ready'],
    ]),
  });
  const work = clone();
  pushBranch(work, 'task/T-1.2');
  sh(work, 'git', ['fetch', '-q', 'origin']);
  assert.deepEqual(runScript(work, ['next']), { code: 0, out: 'T-1.5\n', err: '' });
});

test('TT-2: next with no row ready exits 1 and prints a reason for each row', () => {
  const { clone } = scratchRemote({
    'docs/TASKS.md': queueMd([
      ['T-1.1', 'done', 'finished'],
      ['T-1.2', 'todo', 'claimed', 'T-1.1'],
      ['T-1.3', 'todo', 'waits', 'T-1.4'],
      ['T-1.4', 'blocked', 'needs a decision'],
    ]),
  });
  const work = clone();
  pushBranch(work, 'task/T-1.2');
  assert.deepEqual(runScript(work, ['next']), {
    code: 1,
    out: 'no task ready\nT-1.2: claimed by task/T-1.2\nT-1.3: waiting on T-1.4\nT-1.4: blocked: needs a decision\n',
    err: '',
  });
});

test('NX-3: next exits 2 and names the git command when git cannot read the queue', () => {
  const notARepo = mkdtempSync(join(tmpdir(), 'task-queue-norepo-'));
  const r = runScript(notARepo, ['next']);
  assert.equal(r.code, 2);
  assert.ok(r.err.startsWith('git show origin/main:docs/TASKS.md: '), r.err);
});

test('NX-4: next reads the local origin/main without fetching', () => {
  const { clone, seed, remote } = scratchRemote({ 'docs/TASKS.md': queueMd([['T-1.1', 'todo', 'old']]) });
  const work = clone();
  write(seed, { 'docs/TASKS.md': queueMd([['T-1.1', 'done', 'old'], ['T-1.2', 'todo', 'new']]) });
  sh(seed, 'git', ['commit', '-q', '-am', 'advance']);
  sh(seed, 'git', ['push', '-q', remote, 'main']);
  assert.equal(runScript(work, ['next']).out, 'T-1.1\n');
});

// ---- TT-3, TT-4: the claim snippet --------------------------------------------

function claimSnippet(taskId) {
  const doc = readFileSync(join(REPO_ROOT, 'docs/assurance/DEVELOPMENT-FRAMEWORK.md'), 'utf8');
  const at = doc.indexOf('2. **Claim.**');
  assert.notEqual(at, -1, 'the "2. **Claim.**" step is in DEVELOPMENT-FRAMEWORK.md');
  const block = doc.slice(at).match(/```bash\n([\s\S]*?)```/);
  assert.ok(block, 'a ```bash block follows the "2. **Claim.**" step');
  return block[1].replaceAll('<task ID>', taskId);
}

function runClaim(cwd, snippet) {
  return new Promise((resolve) => {
    const child = spawn('bash', ['-e', '-c', snippet], { cwd, env: GIT_ENV, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    child.stdout.on('data', (d) => (out += d));
    child.stderr.on('data', (d) => (out += d));
    child.on('close', (code) => resolve({ code, out }));
  });
}

const remoteSha = (remote, taskId) =>
  sh(remote, 'git', ['ls-remote', remote, `refs/heads/task/${taskId}`]).split('\t')[0];

test('TT-3: two claims racing for one task have exactly one winner, and the remote branch is the winner\'s HEAD', async () => {
  const taskId = 'T-9.1';
  const snippet = claimSnippet(taskId);
  for (let round = 1; round <= 5; round += 1) {
    const { remote, clone } = scratchRemote({ 'README.md': 'x' });
    const clones = [clone(), clone()];
    const results = await Promise.all(clones.map((dir) => runClaim(dir, snippet)));
    const winners = results.map((r, i) => i).filter((i) => results[i].code === 0);
    assert.equal(winners.length, 1, `round ${round}: exactly one exit 0\n${results.map((r) => r.out).join('\n--\n')}`);
    const [winner] = winners;
    const loser = 1 - winner;
    const sha = remoteSha(remote, taskId);
    assert.equal(sh(clones[winner], 'git', ['rev-parse', 'HEAD']), sha, `round ${round}: remote branch is the winner's HEAD`);
    assert.notEqual(results[loser].code, 0, `round ${round}: loser exits non-zero`);
    assert.notEqual(sh(clones[loser], 'git', ['rev-parse', 'HEAD']), sha, `round ${round}: loser's HEAD is not the remote branch`);
  }
});

test('TT-4: a claim after a claim exits non-zero and leaves the remote branch at the first SHA', async () => {
  const taskId = 'T-9.2';
  const snippet = claimSnippet(taskId);
  const { remote, clone } = scratchRemote({ 'README.md': 'x' });
  const [first, second] = [clone(), clone()];
  const a = await runClaim(first, snippet);
  assert.equal(a.code, 0, a.out);
  const sha = remoteSha(remote, taskId);
  assert.equal(sha, sh(first, 'git', ['rev-parse', 'HEAD']));
  const b = await runClaim(second, snippet);
  assert.notEqual(b.code, 0, b.out);
  assert.equal(remoteSha(remote, taskId), sha);
});
