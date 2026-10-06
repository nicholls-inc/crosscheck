#!/usr/bin/env node
// pre-commit.mjs runs the checks that need no PR body when a commit is made.
//
// Requirement IDs (PC-*) refer to intent/2026-10-06-pre-commit-hooks-spec.md.
// The hook (.husky/pre-commit) runs it from the repository root.
//
// Inputs (env):
//   CROSSCHECK_PROTECTED_RULES - optional override path to the protected-surfaces
//                        rules file (default: .claude/rules/protected-surfaces.md)
//
// It reads the index and the default branch as last fetched, and never fetches
// (PC-3). It prints nothing when every check that applies passes.
//
// No dependencies. Node ESM. Exit codes (PC-2): 0 pass, 1 findings, 2 could not read.

import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { globToRegExp, isGovernanceNotePath, loadProtectedGlobs, unnamedProtectedFiles } from './tier-gate.mjs';
import { checkRows, parseItemIds, parseQueue } from './task-queue.mjs';

const DEFAULT_RULES_PATH = '.claude/rules/protected-surfaces.md';
const TASKS_PATH = 'docs/TASKS.md';
const ROADMAP_PATH = 'docs/assurance/ROADMAP.md';
const RERUN = 'node scripts/ci/pre-commit.mjs';

class CannotRead extends Error {
  constructor(message, fix = []) {
    super(message);
    this.fix = fix;
  }
}

function git(args) {
  try {
    return execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (err) {
    throw new CannotRead(`git ${args.join(' ')}: ${String(err.stderr || err.message).trim()}`);
  }
}

const splitZ = (out) => out.split('\0').filter((p) => p.length > 0);

// Quote a path for the shell only when it needs it, so a printed command runs as shown.
const shellQuote = (path) => (/^[A-Za-z0-9._/@%+=:,-]+$/.test(path) ? path : `'${path.replace(/'/g, `'\\''`)}'`);

// PC-3. The same rule as the PreToolUse hook: origin/HEAD, else origin/main.
function defaultBranch() {
  for (const ref of ['origin/HEAD', 'origin/main']) {
    try {
      return git(['rev-parse', '--verify', '--quiet', `${ref}^{commit}`]).trim();
    } catch {
      // try the next ref
    }
  }
  throw new CannotRead('neither origin/HEAD nor origin/main resolves, so the branch set cannot be computed', [
    'Fix: git fetch origin',
  ]);
}

function checkGovernanceNotes({ staged, matchesProtected }) {
  const branchSet = splitZ(git(['diff', '--cached', '--name-only', '--no-renames', '-z', '--merge-base', defaultBranch()]));
  const protectedPaths = branchSet.filter(matchesProtected);
  const candidates = branchSet.filter(isGovernanceNotePath);
  const inIndex =
    candidates.length === 0 ? [] : splitZ(git(['--literal-pathspecs', 'ls-files', '-z', '--cached', '--', ...candidates]));
  const noteText = inIndex.map((path) => git(['show', `:${path}`])).join('\n');
  const unnamed = unnamedProtectedFiles(protectedPaths, noteText);
  const problems = unnamed.map((path) => `${path}: protected, and no governance note changed on this branch names it`);
  const fix = [
    'Fix: run /crosscheck:protected-surface-amend in Claude Code to write a note under .assurance/protected-surface-amend/ that names each path above, then git add .assurance/protected-surface-amend/<note>.md',
  ];
  const mistaken = unnamed.filter((path) => staged.includes(path));
  if (mistaken.length > 0) fix.push(`Or, if staged by mistake: git restore --staged ${mistaken.map(shellQuote).join(' ')}`);
  return { problems, fix };
}

function checkTaskQueue() {
  const rows = (() => {
    try {
      return parseQueue(git(['show', `:${TASKS_PATH}`]), TASKS_PATH);
    } catch (err) {
      throw err instanceof CannotRead ? err : new CannotRead(err.message);
    }
  })();
  const fix = [`Fix: edit ${TASKS_PATH} as each line above says, then git add ${TASKS_PATH}`];
  if (!rows) {
    return {
      problems: [`${TASKS_PATH} has no queue: expected a "## Queue" heading followed by a table of tasks`],
      fix,
    };
  }
  const itemIds = parseItemIds(git(['show', `:${ROADMAP_PATH}`]));
  return { problems: checkRows({ rows, itemIds }), fix };
}

// PC-3, PC-4, PC-6. A check runs only when its `applies` is true.
export const CHECKS = [
  {
    name: 'governance notes',
    doc: 'tier-layer-gate.md',
    applies: (staged, matchesProtected) => staged.some(matchesProtected),
    run: checkGovernanceNotes,
  },
  {
    name: 'task queue',
    doc: 'task-queue-check.md',
    applies: (staged) => staged.includes(TASKS_PATH) || staged.includes(ROADMAP_PATH),
    run: checkTaskQueue,
  },
];

function block(check, problems, fix) {
  return [
    `pre-commit: ${check.name}`,
    ...problems.map((p) => `- ${p}`),
    ...fix,
    `Then rerun: ${RERUN}`,
    `Full explanation: docs/gates/${check.doc}`,
  ];
}

// Returns the exit code, and prints one block per failing check to stderr.
export function main(rulesPath) {
  const lines = [];
  let code = 0;
  let staged = [];
  let matchesProtected = () => false;
  let rulesError = null;
  try {
    staged = splitZ(git(['diff', '--cached', '--name-only', '--no-renames', '-z']));
  } catch (err) {
    console.error(block(CHECKS[0], [err.message], []).join('\n'));
    return 2;
  }
  const { globs, missing } = loadProtectedGlobs(resolve(rulesPath));
  if (missing) {
    rulesError = `could not read the protected-surface glob list from "${rulesPath}" (expected a "## Machine-readable path list" fenced block)`;
  } else {
    const regexes = globs.map(globToRegExp);
    matchesProtected = (path) => regexes.some((re) => re.test(path));
  }
  for (const check of CHECKS) {
    if (check === CHECKS[0] && rulesError) {
      lines.push(
        ...block(check, [rulesError], [
          `Fix: restore the "## Machine-readable path list" block in ${rulesPath} (git restore ${shellQuote(rulesPath)} if the file was edited or deleted)`,
        ]),
      );
      code = 2;
      continue;
    }
    if (!check.applies(staged, matchesProtected)) continue;
    try {
      const { problems, fix } = check.run({ staged, matchesProtected });
      if (problems.length > 0) {
        lines.push(...block(check, problems, fix));
        code = Math.max(code, 1);
      }
    } catch (err) {
      if (!(err instanceof CannotRead)) throw err;
      lines.push(...block(check, [err.message], err.fix));
      code = 2;
    }
  }
  if (lines.length > 0) console.error(lines.join('\n'));
  return code;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.env.CROSSCHECK_PROTECTED_RULES || DEFAULT_RULES_PATH);
}
