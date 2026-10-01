#!/usr/bin/env node
// task-queue.mjs prints the next task and checks the task queue in CI.
//
// Requirement IDs (QP-*, NX-*, QC-*) refer to intent/2026-10-01-queue-check-spec.md.
//
//   node scripts/ci/task-queue.mjs next    reads origin/main and the task/* branches
//   node scripts/ci/task-queue.mjs check   reads the working tree and the base queue
//
// check inputs (env):
//   BASE_REF  - the PR's base branch; the base queue is origin/$BASE_REF:docs/TASKS.md
//   PR_BODY   - full pull request description; its "Task:" line names the one
//               row the PR may set to done
//
// `next` never fetches (NX-4): run `git fetch origin` first.
//
// No dependencies. Node ESM. Exit codes: 0 pass, 1 findings, 2 could not read.

import { execFileSync, execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const GATE_DOC = 'task-queue-check.md';
const TASKS_PATH = 'docs/TASKS.md';
const ROADMAP_PATH = 'docs/assurance/ROADMAP.md';
const HEADER = ['Task', 'Status', 'What', 'Depends on', 'Issue', 'Record'];
const STATUSES = ['todo', 'blocked', 'done'];
const TASK_ID = /^([A-Z]+-\d+)\.[1-9]\d*$/;

// A cell may hold an escaped pipe, so split only on a pipe with no backslash before it.
function splitCells(line) {
  return line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split(/(?<!\\)\|/)
    .map((cell) => cell.replace(/\\\|/g, '|').trim());
}

// QP-1, QP-2, QP-4. Returns null when there is no queue, and throws when the
// queue's table has no separator row, since its first row could not be told
// from a separator.
export function parseQueue(markdown, source = TASKS_PATH) {
  const lines = (markdown || '').split(/\r?\n/);
  const heading = lines.findIndex((l) => /^##[ \t]+Queue[ \t]*$/.test(l));
  if (heading === -1) return null;
  const start = lines.findIndex((l, i) => i > heading && l.trim().startsWith('|'));
  if (start === -1) return null;
  const header = splitCells(lines[start]);
  if (header.length !== HEADER.length || header.some((h, i) => h !== HEADER[i])) return null;
  const separator = (lines[start + 1] || '').trim().startsWith('|') ? splitCells(lines[start + 1]) : [];
  if (separator.length !== HEADER.length || !separator.every((c) => /^:?-+:?$/.test(c))) {
    throw new Error(
      `${source}: the queue table has no separator row under its header; expected | ${HEADER.map(() => '---').join(' | ')} |`
    );
  }
  const rows = [];
  for (const line of lines.slice(start + 2)) {
    if (!line.trim().startsWith('|')) break;
    const [id, status, what, dependsOn, issue, record] = splitCells(line);
    rows.push({
      id,
      status,
      what,
      dependsOn: (dependsOn || '')
        .split(',')
        .map((d) => d.replace(/`/g, '').trim())
        .filter((d) => d.length > 0),
      issue,
      record,
    });
  }
  return rows;
}

// QP-3.
export function parseItemIds(roadmapMarkdown) {
  const ids = new Set();
  for (const m of (roadmapMarkdown || '').matchAll(/^\*\*([A-Z]+-\d+) — Status:/gm)) ids.add(m[1]);
  return ids;
}

// QC-5. Same anchor as the tier gate's "Tier:" line: a list or quote marker
// before the keyword is not whitespace, so it does not match.
export function parseTaskLine(prBody) {
  const line = (prBody || '').match(/^[ \t]*Task:(.*)$/im);
  return line ? line[1].trim() : null;
}

// NX-1, NX-2.
export function nextTask(rows, claimedIds) {
  const claimed = new Set(claimedIds);
  const done = new Set(rows.filter((r) => r.status === 'done').map((r) => r.id));
  const waitingOn = (row) => row.dependsOn.filter((d) => !done.has(d));
  const ready = rows.find(
    (r) => r.status === 'todo' && waitingOn(r).length === 0 && !claimed.has(r.id)
  );
  if (ready) return { id: ready.id };
  const reasons = rows
    .filter((r) => r.status !== 'done')
    .map((r) => {
      if (r.status !== 'todo') return { id: r.id, reason: `blocked: ${r.what}` };
      if (claimed.has(r.id)) return { id: r.id, reason: `claimed by task/${r.id}` };
      return { id: r.id, reason: `waiting on ${waitingOn(r).join(', ')}` };
    });
  return { reasons };
}

export function newlyDoneIds(rows, baseRows) {
  const doneAtBase = new Set(baseRows.filter((r) => r.status === 'done').map((r) => r.id));
  return rows.filter((r) => r.status === 'done' && !doneAtBase.has(r.id)).map((r) => r.id);
}

// QC-1 to QC-5. An empty array means the queue passes.
export function checkQueue({ rows, baseRows, itemIds, prBody }) {
  const problems = [];
  const ids = new Set(rows.map((r) => r.id));
  const seen = new Set();
  for (const row of rows) {
    const match = row.id.match(TASK_ID);
    if (!match) {
      problems.push(`${row.id}: the task ID is not of the form <item ID>.<n>, such as PB-1.6`);
    } else if (!itemIds.has(match[1])) {
      problems.push(`${row.id}: ${match[1]} is not an item in ${ROADMAP_PATH}`);
    }
    if (seen.has(row.id)) problems.push(`${row.id}: the queue has more than one row with this task ID`);
    seen.add(row.id);
    if (!STATUSES.includes(row.status)) {
      problems.push(`${row.id}: the status "${row.status}" is not todo, blocked or done`);
    }
    for (const dep of row.dependsOn) {
      if (!ids.has(dep)) problems.push(`${row.id}: depends on ${dep}, which is not a row of the queue`);
    }
  }
  const taskLine = parseTaskLine(prBody);
  for (const id of newlyDoneIds(rows, baseRows)) {
    if (id !== taskLine) {
      const named = taskLine === null ? 'has no "Task:" line' : `has the "Task:" line "${taskLine}"`;
      problems.push(`${id}: the row is newly done, but the PR body ${named}. Only the row named in the "Task:" line may become done`);
    }
  }
  return problems;
}

function ownerRepoFromRemote(url) {
  let m = url.match(/git@github\.com:([^/]+)\/(.+?)(\.git)?$/);
  if (m) return `${m[1]}/${m[2]}`;
  m = url.match(/https:\/\/github\.com\/([^/]+)\/([^/]+?)(\.git)?\/?$/);
  if (m) return `${m[1]}/${m[2]}`;
  return '';
}

function gateDocLink(cwd) {
  let url = '';
  try {
    url = execSync('git remote get-url origin', { cwd, stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim();
  } catch {
    url = '';
  }
  const ownerRepo = ownerRepoFromRemote(url);
  if (!ownerRepo) return `docs/gates/${GATE_DOC}`;
  return `https://github.com/${ownerRepo}/blob/main/docs/gates/${GATE_DOC}`;
}

// QC-6.
function failureLines(problems, cwd) {
  return [
    '**Action needed: fix the task queue**',
    'You are being asked to correct docs/TASKS.md, or the "Task:" line of the pull request body, because the queue breaks a rule of the pick-up procedure.',
    'Approving means the pick-up procedure reads a valid queue and only the pull request that does a task marks it done; declining leaves this check red.',
    `Full explanation: ${gateDocLink(cwd)}.`,
    '',
    ...problems.map((p) => `- ${p}`),
  ];
}

class GitFailure extends Error {}

function git(args) {
  try {
    return execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (err) {
    const detail = String(err.stderr || err.message).trim();
    throw new GitFailure(`git ${args.join(' ')}: ${detail}`);
  }
}

function runNext() {
  const queue = parseQueue(git(['show', `origin/main:${TASKS_PATH}`]), `origin/main:${TASKS_PATH}`);
  if (!queue) throw new GitFailure(`origin/main:${TASKS_PATH} has no queue table`);
  const claimed = git(['ls-remote', '--heads', 'origin', 'task/*'])
    .split('\n')
    .map((l) => l.match(/\trefs\/heads\/task\/(.+)$/))
    .filter(Boolean)
    .map((m) => m[1]);
  const result = nextTask(queue, claimed);
  if (result.id) {
    console.log(result.id);
    return 0;
  }
  console.log('no task ready');
  for (const { id, reason } of result.reasons) console.log(`${id}: ${reason}`);
  return 1;
}

// QC-5, QC-7. Only a missing file means an empty base queue.
function readBaseRows(baseRef) {
  const source = `origin/${baseRef}:${TASKS_PATH}`;
  let markdown;
  try {
    markdown = git(['show', source]);
  } catch (err) {
    if (/does not exist|exists on disk, but not in/.test(err.message)) return [];
    throw err;
  }
  const rows = parseQueue(markdown, source);
  if (!rows) {
    throw new Error(
      `${source} exists but has no queue, so the check cannot tell which rows are newly done: expected a "## Queue" heading followed by a table whose header is | ${HEADER.join(' | ')} |`
    );
  }
  return rows;
}

function runCheck() {
  const baseRef = process.env.BASE_REF;
  if (!baseRef) throw new GitFailure('BASE_REF is not set');
  const baseRows = readBaseRows(baseRef);
  const rows = parseQueue(readFileSync(TASKS_PATH, 'utf8'));
  const itemIds = parseItemIds(readFileSync(ROADMAP_PATH, 'utf8'));
  const problems = rows
    ? checkQueue({ rows, baseRows, itemIds, prBody: process.env.PR_BODY || '' })
    : [`${TASKS_PATH} has no queue: expected a "## Queue" heading followed by a table whose header is | ${HEADER.join(' | ')} |`];
  if (problems.length > 0) {
    for (const line of failureLines(problems, process.cwd())) console.log(line);
    return 1;
  }
  console.log(`task-queue: PASS - ${rows.length} row(s) checked, ${newlyDoneIds(rows, baseRows).length} newly done.`);
  return 0;
}

function main() {
  const command = process.argv[2];
  if (command !== 'next' && command !== 'check') {
    console.error('usage: task-queue.mjs next|check');
    process.exit(2);
  }
  try {
    process.exit(command === 'next' ? runNext() : runCheck());
  } catch (err) {
    console.error(err.message);
    process.exit(2);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
