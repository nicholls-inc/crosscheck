#!/usr/bin/env node
// tier-gate.mjs — deterministic CI gate enforcing the assurance tier map.
//
// Requirement IDs (TG-*) refer to intent/2026-09-29-deterministic-evidence-spec.md
// and intent/2026-09-30-tier-anchor-spec.md (TG-1 and TG-8 revised there).
//
// Inputs (env):
//   PR_BODY           - full pull request description text
//   PR_LABELS         - comma-separated label list (e.g. "tier:2,needs-review")
//   CHANGED_FILES_PATH - path of a file holding the changed file paths
//                        (repo-relative), NUL-separated as `git diff -z
//                        --name-only` writes them, so no name is quoted (TG-16).
//                        Unset, empty or unreadable fails the gate (TG-17)
//   BASE_REF          - the PR's base branch/commit (accepted for interface
//                        completeness and included in the pass summary; the
//                        actual diff is supplied via CHANGED_FILES_PATH)
//   CROSSCHECK_PROTECTED_RULES - optional override path to the protected-surfaces
//                        rules file (default: .claude/rules/protected-surfaces.md)
//
// An artefact found implicitly (root spec.md, root plan.md, a governance note)
// counts only if this pull request changes it; an artefact the PR body cites by
// path ("Spec: <path>", "Plan: <path>") counts if it is a regular file inside
// the repository. No LLM verdict is read: intent-check attestations are ignored
// (TG-6).
//
// No dependencies. Node ESM. Exits 0 on pass, 1 on failure.

import { existsSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { execSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const DEFAULT_RULES_PATH = '.claude/rules/protected-surfaces.md';
const GATE_DOC = 'tier-layer-gate.md';

// Changing either of these changes what CGV's exit code 0 promises (TG-7).
export const CGV_PROOF_SURFACES = [
  'cgv/prover/ContractGraph/BehaviorModel.lean',
  'cgv/prover/protected-statements.txt',
  'cgv/prover/scripts/ProtectedStatements.lean',
];

// Evidence classes for the pass report (TG-8). First match wins; the last row
// matches every path.
export const EVIDENCE_CLASSES = [
  {
    re: /^(crosscheck\/(skills|agents)\/|\.claude\/rules\/|docs\/assurance\/)|(^|\/)(CLAUDE|AGENTS|REVIEW)\.md$/,
    kind: 'notYetReached',
    property: 'their behaviour is prompt text that an agent interprets',
    question: 'what a replayable behavioural eval of a prompt artefact looks like',
  },
  {
    re: /^cgv\//,
    kind: 'checked',
    workflow: 'CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check)',
  },
  {
    re: /^crosscheck\/(mcp-server|docs\/invariants)\//,
    kind: 'checked',
    workflow: 'CI workflow (npm test, including the property tests)',
  },
  {
    re: /^crosscheck\/conformance\//,
    kind: 'checked',
    workflow: 'CI workflow, conformance job (go vet, go test, go run . ..)',
  },
  {
    re: /^(scripts\/ci\/|\.claude\/hooks\/protected-surface-guard\.mjs$|\.husky\/pre-commit$)/,
    kind: 'checked',
    workflow: 'Tier Gate workflow (node --test scripts/ci/*.test.mjs)',
  },
  {
    re: /^scripts\/check-evidence-record(\.test)?\.mjs$/,
    kind: 'checked',
    workflow: 'Evidence Record workflow (node --test scripts/check-evidence-record.test.mjs)',
  },
  {
    re: /^evals\//,
    kind: 'notYetReached',
    property: 'no CI job runs an eval, and the Incident Eval Check only looks, after the merge, for an eval under evals/ and a candidate invariant that name the incident',
    question: 'what runs an eval as a regression test, and which workflow runs it before the merge',
  },
  {
    re: /^docs\/invariants\//,
    kind: 'notYetReached',
    property: 'no CI job maps these invariants to the tests that cover them',
    question: 'which test covers each invariant, and which workflow checks that mapping',
  },
  {
    re: /^\.github\/workflows\//,
    kind: 'notYetReached',
    property: "a workflow runs only on GitHub's runners, on GitHub's events",
    question: 'how to replay a workflow against recorded events before it merges',
  },
  {
    re: /\.(md|pdf)$/i,
    kind: 'notYetReached',
    property: 'prose has no executable meaning, so no check reads what it claims',
    question: 'which claims in a prose document, such as cited paths and commands, a deterministic check can verify',
  },
  {
    re: /./,
    kind: 'notYetReached',
    property: 'no CI workflow runs a check on this path',
    question: 'which deterministic check this code needs, and which workflow runs it',
  },
];

function splitList(raw, separator) {
  return (raw || '')
    .split(separator)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export function globToRegExp(glob) {
  const parts = glob.split('/').map((part) => {
    if (part === '**') return '.*';
    const escaped = part.replace(/[.+^${}()|[\]\\]/g, '\\$&');
    return escaped.replace(/\*/g, '[^/]*');
  });
  return new RegExp(`^${parts.join('/')}$`, 's');
}

export function loadProtectedGlobs(rulesPath) {
  if (!existsSync(rulesPath)) {
    return { globs: [], missing: true };
  }
  const text = readFileSync(rulesPath, 'utf8');
  const headingIdx = text.indexOf('## Machine-readable path list');
  if (headingIdx === -1) return { globs: [], missing: true };
  const afterHeading = text.slice(headingIdx);
  const fenceMatch = afterHeading.match(/```\n([\s\S]*?)```/);
  if (!fenceMatch) return { globs: [], missing: true };
  const globs = fenceMatch[1]
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  return { globs, missing: false };
}

// TG-1. The first line that starts with "Tier:" (after an optional indent, but
// no list or quote marker) is the declaration, valid or not. A tier:N label
// must agree with it and with every other tier:N label.
function parseDeclaredTier(prBody, prLabels) {
  const labelTiers = [
    ...new Set(prLabels.map((l) => l.match(/^tier:([123])$/i)).filter(Boolean).map((m) => Number(m[1]))),
  ];
  if (labelTiers.length > 1) {
    return { problem: `The labels ${labelTiers.map((t) => `tier:${t}`).join(' and ')} disagree. Keep one tier:N label.` };
  }
  const line = (prBody || '').match(/^[ \t]*Tier:(.*)$/im);
  if (!line) return { tier: labelTiers[0] };
  const value = line[1].trim();
  if (!/^[123]$/.test(value)) {
    return {
      problem: `The first "Tier:" line in the PR body, "${line[0].trim()}", does not declare Tier 1, 2 or 3. Write the line as "Tier: 1", "Tier: 2" or "Tier: 3" with nothing after the digit.`,
    };
  }
  const tier = Number(value);
  if (labelTiers.length === 1 && labelTiers[0] !== tier) {
    return {
      problem: `The PR body declares Tier ${tier}, but the label says Tier ${labelTiers[0]}. They must agree: change the "Tier:" line or the tier:N label.`,
    };
  }
  return { tier };
}

export function isGovernanceNotePath(path) {
  return (
    path.endsWith('.md') &&
    (/^\.assurance\/protected-surface-amend\/[^/]+\.md$/.test(path) ||
      /^\.assurance\/add-session-[^/]+\/.+\.md$/.test(path))
  );
}

// TG-5. A path is named when the note text contains it as a substring.
export function unnamedProtectedFiles(protectedMatches, noteText) {
  return protectedMatches.filter((f) => !noteText.includes(f));
}

function hasProtectedSurfaceChangeSection(prBody) {
  return /^#{2,6}[ \t]+protected-surface change[ \t]*#*[ \t]*$/im.test(prBody || '');
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

function failureLines(missingItems, cwd) {
  const link = gateDocLink(cwd);
  return [
    '**Action needed: declare the correct tier and add missing artefacts**',
    `You are being asked to declare this pull request's assurance tier and supply the artefacts that tier requires because the change does not yet satisfy the tier-layer gate. Approving means the pull request carries an auditable record matching its actual risk level; declining means the change stays blocked until the tier and artefacts are corrected. Full explanation: ${link}.`,
    '',
    ...missingItems.map((item) => `- ${item}`),
  ];
}

function reportLine(row, files) {
  const count = `${files.length} file(s), e.g. ${files[0]}`;
  if (row.kind === 'checked') return `- ${row.workflow}: ${count}`;
  return `- not yet reached: ${count}. Blocking property: ${row.property}. Open question: ${row.question}.`;
}

function evidenceReport(changedFiles) {
  const byRow = new Map();
  for (const file of changedFiles) {
    const row = EVIDENCE_CLASSES.find((c) => c.re.test(file));
    if (!byRow.has(row)) byRow.set(row, []);
    byRow.get(row).push(file);
  }
  return ['Deterministic evidence for this diff:', ...[...byRow].map(([row, files]) => reportLine(row, files))];
}

/**
 * Evaluate the tier gate. Pure apart from reading files under `cwd`.
 * Returns { pass, lines } where `lines` is the output to print.
 */
export function evaluate({
  prBody = '',
  prLabels = [],
  changedFiles = [],
  baseRef = '(unspecified)',
  rulesPath = DEFAULT_RULES_PATH,
  cwd = process.cwd(),
}) {
  const present = (p) => existsSync(join(cwd, p));
  const changedAndPresent = new Set(changedFiles.filter(present));
  // A citation starts its own line ("Plan: <path>"), optionally as a list item
  // or quote ("- Plan: <path>", "> Plan: <path>"), so prose that happens to
  // contain "plan:" does not count. Unlike a "Tier:" line, a quoted citation
  // still counts: it can only name a file in the repository, not pick the tier.
  // Any citation line may satisfy the requirement (TG-12).
  const cwdReal = realpathSync(cwd);
  const isRepoFile = (cited) => {
    let real;
    try {
      real = realpathSync(resolve(cwd, cited));
    } catch {
      return false;
    }
    const rel = relative(cwdReal, real);
    return rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel) && statSync(real).isFile();
  };
  const citedExisting = (keyword) =>
    [...prBody.matchAll(new RegExp(`^[ \\t]*(?:[-*+>][ \\t]+)?${keyword}:[ \\t]*(\\S+)`, 'gim'))].some((m) =>
      isRepoFile(m[1].replace(/^`|`$/g, ''))
    );

  const { globs: protectedGlobs, missing: rulesMissing } = loadProtectedGlobs(resolve(cwd, rulesPath));
  const protectedRegexes = protectedGlobs.map(globToRegExp);
  const protectedMatches = changedFiles.filter((f) => protectedRegexes.some((re) => re.test(f)));
  const floorTier = protectedMatches.length > 0 ? 3 : undefined;
  const { tier: declaredTier, problem: declarationProblem } = parseDeclaredTier(prBody, prLabels);

  const missing = [];
  const fail = () => ({ pass: false, lines: failureLines(missing, cwd) });

  if (rulesMissing) {
    missing.push(
      `Could not read the protected-surface glob list from "${rulesPath}" (expected a "## Machine-readable path list" fenced block).`
    );
  }

  // TG-1: declaration and floor.
  if (declarationProblem) {
    missing.push(declarationProblem);
    return fail();
  }
  if (floorTier === undefined && declaredTier === undefined) {
    missing.push('Declare this pull request\'s tier via a "Tier: N" line in the PR body or a "tier:N" label.');
    return fail();
  }
  if (floorTier !== undefined && declaredTier !== undefined && declaredTier < floorTier) {
    missing.push(
      `Protected file(s) in this diff force a floor of Tier ${floorTier}, but the PR declares Tier ${declaredTier}: ${protectedMatches.join(', ')}`
    );
    return fail();
  }

  const effectiveTier = floorTier !== undefined ? floorTier : declaredTier;

  if (effectiveTier === 1) {
    // TG-2
    const changedIntent = [...changedAndPresent].some((f) => {
      if (!f.startsWith('intent/')) return false;
      const base = f.split('/').pop();
      return base !== 'README.md' && base !== 'TEMPLATE.md';
    });
    if (!changedIntent && !citedExisting('Intent')) {
      missing.push(
        'Tier 1 requires an intent artefact: a changed file under intent/ (other than README.md or TEMPLATE.md), or an "Intent: <path>" reference in the PR body pointing to an existing file.'
      );
    }
  } else if (effectiveTier === 2) {
    // TG-3
    if (!changedAndPresent.has('spec.md') && !citedExisting('Spec')) {
      missing.push(
        'Tier 2 requires a spec artefact: spec.md at the repository root changed in this pull request, or a "Spec: <path>" reference in the PR body pointing to an existing file.'
      );
    }
  } else if (effectiveTier === 3) {
    // TG-4
    if (!changedAndPresent.has('plan.md') && !citedExisting('Plan')) {
      missing.push(
        'Tier 3 requires a build plan: plan.md at the repository root changed in this pull request, or a "Plan: <path>" reference in the PR body pointing to an existing file. A plan from an earlier change does not count unless the PR body cites it.'
      );
    }

    // TG-5
    if (protectedMatches.length > 0) {
      const notes = [...changedAndPresent].filter(isGovernanceNotePath);
      const noteText = notes.map((f) => readFileSync(join(cwd, f), 'utf8')).join('\n');
      const unnamed = unnamedProtectedFiles(protectedMatches, noteText);
      if (notes.length === 0 || unnamed.length > 0) {
        missing.push(
          `Tier 3 requires a governance note changed in this pull request, under .assurance/protected-surface-amend/ or .assurance/add-session-*/, naming each changed protected file. Notes from earlier changes do not count. Not named: ${(unnamed.length > 0 ? unnamed : protectedMatches).join(', ')}`
        );
      }
    }
    // TG-6: no attestation requirement. intent-check verdicts are LLM judgement
    // and are never evidence here.
  }

  // TG-7: CGV proof surfaces need the protected-surface change section at any tier.
  const cgvSurfaces = changedFiles.filter((f) => CGV_PROOF_SURFACES.includes(f));
  if (cgvSurfaces.length > 0 && !hasProtectedSurfaceChangeSection(prBody)) {
    missing.push(
      `This pull request changes a CGV proof surface (${cgvSurfaces.join(', ')}). Add a "## Protected-surface change" section to the PR body: what changed, why, the effect on the guarantee, and the evidence (see .claude/rules/protected-surfaces.md).`
    );
  }

  if (missing.length > 0) return fail();

  return {
    pass: true,
    lines: [
      `tier-gate: PASS — Tier ${effectiveTier} artefacts present (declared: ${declaredTier ?? 'undeclared'}, floor: ${floorTier ?? 'none'}, base: ${baseRef}).`,
      '',
      ...evidenceReport(changedFiles), // TG-8
      '',
      // TG-9
      'Human sign-off: the maintainer\'s merge, which bypasses the default-branch ruleset. The ruleset requires no status checks, so CI informs the merge but cannot block it.',
    ],
  };
}

// TG-17: without a readable list the gate cannot see a protected path, so it
// fails rather than evaluating an empty diff.
function readChangedFiles(path) {
  if (!path) {
    return {
      problem:
        'CHANGED_FILES_PATH is not set. Set it to a file holding the changed files, as `git diff -z --name-only --no-renames "origin/$BASE_REF...HEAD"` writes them.',
    };
  }
  try {
    return { files: readFileSync(path, 'utf8').split('\0').filter((s) => s.length > 0) };
  } catch (err) {
    return { problem: `Could not read the changed files from CHANGED_FILES_PATH ("${path}"): ${err.code ?? err.message}.` };
  }
}

function main() {
  const { files, problem } = readChangedFiles(process.env.CHANGED_FILES_PATH);
  const result = problem
    ? { pass: false, lines: failureLines([problem], process.cwd()) }
    : evaluate({
        prBody: process.env.PR_BODY || '',
        prLabels: splitList(process.env.PR_LABELS, ','),
        changedFiles: files,
        baseRef: process.env.BASE_REF || '(unspecified)',
        rulesPath: process.env.CROSSCHECK_PROTECTED_RULES || DEFAULT_RULES_PATH,
      });
  for (const line of result.lines) console.log(line);
  process.exit(result.pass ? 0 : 1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
