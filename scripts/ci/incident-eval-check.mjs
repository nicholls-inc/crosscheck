#!/usr/bin/env node
// incident-eval-check.mjs — deterministic CI gate enforcing that every
// incident-referencing PR ships a regression eval and a candidate invariant.
//
// Inputs (env):
//   PR_BODY    - full pull request description text
//   PR_LABELS  - comma-separated label list
//   PR_NUMBER  - pull request number; its commits are read from refs/pull/<n>/head
//   BASE_REF   - base branch name
//   HEAD_SHA   - the pull request's head commit
//
// No dependencies. Node ESM. Exits 0 when the check does not apply or passes,
// 1 when it applies and fails, and 2 when the commits cannot be read or the
// range holds none.
// Requirement IDs (IE-*) refer to intent/2026-09-30-incident-eval-range-spec.md.

import { readFileSync, readdirSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { execFileSync } from 'node:child_process';

const CWD = process.cwd();
const GATE_DOC = 'README.md';
const INVARIANT_DIRS = ['docs/invariants', 'crosscheck/docs/invariants'];
const EVAL_DIR = 'evals';
// Node's default is 1 MiB, which a long commit message or history can exceed.
// Output past this limit makes execFileSync throw ENOBUFS, which exits 2.
const GIT_MAX_BUFFER = 256 * 1024 * 1024;

function fail(detail) {
  console.error(`incident-eval-check: could not read the pull request's commits: ${detail}`);
  process.exit(2);
}

function gitOrFail(args) {
  try {
    return execFileSync('git', args, { cwd: CWD, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: GIT_MAX_BUFFER });
  } catch (err) {
    fail(`git ${args.join(' ')}\n${(err.stderr || err.message).trim()}`);
  }
}

// A squash merge leaves the head commits on no branch and deletes the branch,
// so the checkout lacks them. GitHub keeps refs/pull/<n>/head. The range starts
// at origin/<base>, which excludes base-branch commits merged into the PR branch.
// Returns one entry per line, so an incident line is matched within its own line.
function readCommitLines() {
  const prNumber = process.env.PR_NUMBER || '';
  const baseRef = process.env.BASE_REF || '';
  const headSha = process.env.HEAD_SHA || '';
  if (!/^[0-9]+$/.test(prNumber) || !/^[0-9a-f]{40}$/.test(headSha) || !baseRef) {
    fail(`PR_NUMBER, BASE_REF and HEAD_SHA must be set (got "${prNumber}", "${baseRef}", "${headSha}")`);
  }
  gitOrFail(['fetch', '--no-tags', '--quiet', 'origin', `+refs/pull/${prNumber}/head:refs/remotes/origin/pr/${prNumber}`]);
  const range = `origin/${baseRef}..${headSha}`;
  const messages = gitOrFail(['log', '-z', '--format=%B', range]).split('\0').filter((m) => m.trim().length > 0);
  // An empty range means the head is already on the base branch (a merge commit
  // or a rebase merge). Its commits cannot be told apart from the base's, so
  // fail rather than skip.
  if (messages.length === 0) {
    fail(`no commits in ${range}`);
  }
  return messages
    .flatMap((m) => m.split('\n'))
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}

function readEnvList(name, sep_) {
  const raw = process.env[name] || '';
  return raw
    .split(sep_)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function walk(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    if (entry.name === 'node_modules' || entry.name === '.git') continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(full, out);
    } else {
      out.push(full);
    }
  }
  return out;
}

function gitOriginUrl() {
  try {
    return execFileSync('git', ['remote', 'get-url', 'origin'], { cwd: CWD, encoding: 'utf8' }).trim();
  } catch {
    return '';
  }
}

function ownerRepoFromRemote(url) {
  let m = url.match(/git@github\.com:([^/]+)\/(.+?)(\.git)?$/);
  if (m) return `${m[1]}/${m[2]}`;
  m = url.match(/https:\/\/github\.com\/([^/]+)\/([^/]+?)(\.git)?\/?$/);
  if (m) return `${m[1]}/${m[2]}`;
  return '';
}

function gateDocLink() {
  const ownerRepo = ownerRepoFromRemote(gitOriginUrl());
  if (!ownerRepo) return `docs/gates/${GATE_DOC}`;
  return `https://github.com/${ownerRepo}/blob/main/docs/gates/${GATE_DOC}`;
}

// IE-9. Only a line that is the trigger and one id is a reference, so prose
// that quotes the trigger, lists it, or wraps onto it does not fire the check.
const INCIDENT_LINE = /^[ \t]*Fixes-Incident:[ \t]*(\S+)[ \t]*$/i;

function findIncidentId(prBody, commitLines) {
  const lines = [...(prBody || '').split(/\r?\n/), ...commitLines];
  for (const line of lines) {
    const m = line.match(INCIDENT_LINE);
    if (m) return m[1].replace(/[.,;]$/, '');
  }
  return undefined;
}

function printFailure(missingItems) {
  const link = gateDocLink();
  console.log('**Action needed: add an eval and candidate invariant**');
  console.log(
    `You are being asked to add a regression eval and a candidate invariant for this incident because every production incident must leave both artefacts in the suite. Approving means the incident becomes a permanent regression check and a documented invariant; declining means the change stays blocked until both artefacts are added. Full explanation: ${link}.`
  );
  console.log('');
  for (const item of missingItems) {
    console.log(`- ${item}`);
  }
}

function main() {
  const prBody = process.env.PR_BODY || '';
  const prLabels = readEnvList('PR_LABELS', ',');
  const commitLines = readCommitLines();

  const hasIncidentLabel = prLabels.some((l) => l.toLowerCase() === 'incident');
  const incidentId = findIncidentId(prBody, commitLines);

  if (!incidentId && !hasIncidentLabel) {
    console.log('no incident reference — skipped');
    process.exit(0);
  }

  const missing = [];

  if (!incidentId) {
    missing.push(
      'The "incident" label is set but no incident id was found. Add a "Fixes-Incident: <id>" line to the PR body or a commit message.'
    );
    printFailure(missing);
    process.exit(1);
  }

  const allFiles = walk(CWD).map((f) => relative(CWD, f).split(sep).join('/'));

  const evalFiles = allFiles.filter((f) => f.startsWith(`${EVAL_DIR}/`));
  const evalMatch = evalFiles.find((f) => {
    if (f.includes(incidentId)) return true;
    try {
      return readFileSync(join(CWD, f), 'utf8').includes(incidentId);
    } catch {
      return false;
    }
  });
  if (!evalMatch) {
    missing.push(
      `No eval under ${EVAL_DIR}/ names or references incident "${incidentId}". Add one so this incident stays a permanent regression test.`
    );
  }

  const invariantFiles = allFiles.filter((f) =>
    INVARIANT_DIRS.some((d) => f.startsWith(`${d}/`))
  );
  const invariantMatch = invariantFiles.find((f) => {
    try {
      return readFileSync(join(CWD, f), 'utf8').includes(incidentId);
    } catch {
      return false;
    }
  });
  if (!invariantMatch) {
    missing.push(
      `No candidate invariant under docs/invariants/ or crosscheck/docs/invariants/ references incident "${incidentId}". Add or amend one to capture what the incident revealed.`
    );
  }

  if (missing.length > 0) {
    printFailure(missing);
    process.exit(1);
  }

  console.log(`incident-eval-check: PASS — incident "${incidentId}" has an eval and a candidate invariant.`);
  process.exit(0);
}

main();
