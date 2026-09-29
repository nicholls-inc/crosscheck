#!/usr/bin/env node
// tier-gate.mjs — deterministic CI gate enforcing the assurance tier map.
//
// Requirement IDs (TG-*) refer to intent/2026-09-29-deterministic-evidence-spec.md.
//
// Inputs (env):
//   PR_BODY           - full pull request description text
//   PR_LABELS         - comma-separated label list (e.g. "tier:2,needs-review")
//   CHANGED_FILES     - newline-separated list of changed file paths (repo-relative)
//   BASE_REF          - the PR's base branch/commit (accepted for interface
//                        completeness and included in the pass summary; the
//                        actual diff is supplied via CHANGED_FILES)
//   CROSSCHECK_PROTECTED_RULES - optional override path to the protected-surfaces
//                        rules file (default: .claude/rules/protected-surfaces.md)
//
// An artefact found implicitly (root spec.md, root plan.md, a governance note)
// counts only if this pull request changes it; an artefact the PR body cites by
// path ("Spec: <path>", "Plan: <path>") counts if it exists. No LLM verdict is
// read: intent-check attestations are ignored (TG-6).
//
// No dependencies. Node ESM. Exits 0 on pass, 1 on failure.

import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
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

const NOT_YET_REACHED = 'not yet reached: human review is the only evidence';

// Evidence classes for the pass report (TG-8). First match wins.
export const EVIDENCE_CLASSES = [
  { re: /^cgv\//, evidence: 'CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check)' },
  {
    re: /^crosscheck\/(mcp-server|docs\/invariants)\//,
    evidence: 'CI workflow (npm test, including the property tests)',
  },
  { re: /^scripts\/ci\//, evidence: 'Tier Gate workflow (node --test scripts/ci/*.test.mjs)' },
  { re: /^evals\//, evidence: 'Incident Eval Check workflow' },
  {
    re: /^(crosscheck\/(skills|agents)\/|\.claude\/|docs\/assurance\/|\.github\/workflows\/)/,
    evidence: NOT_YET_REACHED,
  },
];

function splitList(raw, separator) {
  return (raw || '')
    .split(separator)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function globToRegExp(glob) {
  const parts = glob.split('/').map((part) => {
    if (part === '**') return '.*';
    const escaped = part.replace(/[.+^${}()|[\]\\]/g, '\\$&');
    return escaped.replace(/\*/g, '[^/]*');
  });
  return new RegExp(`^${parts.join('/')}$`);
}

function loadProtectedGlobs(rulesPath) {
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

function parseDeclaredTier(prBody, prLabels) {
  const bodyMatch = (prBody || '').match(/Tier:\s*([123])\b/i);
  if (bodyMatch) return Number(bodyMatch[1]);
  const labelMatch = prLabels.find((l) => /^tier:([123])$/i.test(l));
  if (labelMatch) return Number(labelMatch.match(/^tier:([123])$/i)[1]);
  return undefined;
}

function isGovernanceNotePath(path) {
  return (
    path.endsWith('.md') &&
    (/^\.assurance\/protected-surface-amend\/[^/]+\.md$/.test(path) ||
      /^\.assurance\/add-session-[^/]+\/.+\.md$/.test(path))
  );
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

function evidenceReport(changedFiles) {
  const byEvidence = new Map();
  for (const file of changedFiles) {
    const cls = EVIDENCE_CLASSES.find((c) => c.re.test(file));
    const evidence = cls ? cls.evidence : 'none required at this tier';
    if (!byEvidence.has(evidence)) byEvidence.set(evidence, []);
    byEvidence.get(evidence).push(file);
  }
  const lines = ['Deterministic evidence for this diff:'];
  for (const [evidence, files] of byEvidence) {
    lines.push(`- ${evidence}: ${files.length} file(s), e.g. ${files[0]}`);
  }
  return lines;
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
  // A citation is a line of its own ("Plan: <path>"), not the word anywhere in
  // the body, so text inside a governance-note block cannot satisfy it.
  const citedExisting = (keyword) => {
    const match = prBody.match(new RegExp(`^[ \\t]*${keyword}:[ \\t]*(\\S+)`, 'im'));
    if (!match) return false;
    const cited = match[1].replace(/^`|`$/g, '');
    return present(cited);
  };

  const { globs: protectedGlobs, missing: rulesMissing } = loadProtectedGlobs(resolve(cwd, rulesPath));
  const protectedRegexes = protectedGlobs.map(globToRegExp);
  const protectedMatches = changedFiles.filter((f) => protectedRegexes.some((re) => re.test(f)));
  const floorTier = protectedMatches.length > 0 ? 3 : undefined;
  const declaredTier = parseDeclaredTier(prBody, prLabels);

  const missing = [];
  const fail = () => ({ pass: false, lines: failureLines(missing, cwd) });

  if (rulesMissing) {
    missing.push(
      `Could not read the protected-surface glob list from "${rulesPath}" (expected a "## Machine-readable path list" fenced block).`
    );
  }

  // TG-1: declaration and floor.
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
      const unnamed = protectedMatches.filter((f) => !noteText.includes(f));
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
      'Human sign-off: the maintainer\'s merge. This repository has no branch protection, so CI informs the merge but cannot block it.',
    ],
  };
}

function main() {
  const result = evaluate({
    prBody: process.env.PR_BODY || '',
    prLabels: splitList(process.env.PR_LABELS, ','),
    changedFiles: splitList(process.env.CHANGED_FILES, '\n'),
    baseRef: process.env.BASE_REF || '(unspecified)',
    rulesPath: process.env.CROSSCHECK_PROTECTED_RULES || DEFAULT_RULES_PATH,
  });
  for (const line of result.lines) console.log(line);
  process.exit(result.pass ? 0 : 1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
