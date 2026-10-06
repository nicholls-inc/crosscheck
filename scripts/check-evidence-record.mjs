#!/usr/bin/env node
// check-evidence-record.mjs checks one evidence record against EV-1 to EV-13 of
// intent/2026-10-06-evidence-record-spec.md.
//
//   node scripts/check-evidence-record.mjs <path>
//
// Exit codes: 0 the record satisfies EV-1 to EV-12, 1 it breaks a rule (one
// line per problem on stdout), 2 no path, unreadable file, or not JSON.

import { readFileSync, realpathSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const FORMAT = 'evidence-record/1';
const RECORD_FIELDS = ['format', 'commit', 'claims'];
const CLAIM_FIELDS = ['id', 'statement', 'requirement', 'strength', 'basis', 'trusted_base', 'rerun'];
const COMPONENT_FIELDS = ['component', 'version'];
const RERUN_FIELDS = ['command', 'exit_code'];
const MAX_EXACT_INTEGER = 2 ** 53;

// The Unicode White_Space property. String.prototype.trim differs: it strips
// U+FEFF, which is not White_Space, and keeps U+0085, which is.
const NON_BLANK = /[^\t-\r \u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]/;

const isObject = (v) => typeof v === 'object' && v !== null && !Array.isArray(v);
const isNonEmpty = (s) => typeof s === 'string' && NON_BLANK.test(s);
const isInteger = (n) =>
  typeof n === 'number' && Number.isFinite(n) && Number.isInteger(n) && Math.abs(n) <= MAX_EXACT_INTEGER;
const field = (obj, key) => (Object.hasOwn(obj, key) ? obj[key] : undefined);

function isRealDate(s) {
  const m = typeof s === 'string' ? /^(\d{4})-(\d{2})-(\d{2})$/.exec(s) : null;
  if (!m) return false;
  const [year, month, day] = [Number(m[1]), Number(m[2]), Number(m[3])];
  if (year < 1 || month < 1 || month > 12 || day < 1) return false;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1];
  return day <= daysInMonth;
}

const BASIS = {
  proved: {
    theorems: {
      expects: 'a non-empty array of non-empty strings',
      ok: (v) => Array.isArray(v) && v.length > 0 && v.every(isNonEmpty),
    },
  },
  tested: {
    cases: { expects: 'an integer of at least 1', ok: (v) => isInteger(v) && v >= 1 },
    seed: { expects: 'a non-empty string or null', ok: (v) => v === null || isNonEmpty(v) },
  },
  observed: {
    source: { expects: 'a non-empty string', ok: isNonEmpty },
  },
  judged: {
    judge: { expects: 'a non-empty string', ok: isNonEmpty },
    date: { expects: 'a real calendar date written YYYY-MM-DD', ok: isRealDate },
  },
};

const isStrength = (v) => typeof v === 'string' && Object.hasOwn(BASIS, v);

function closedObjectProblems(rule, where, obj, expected) {
  const missing = expected.filter((k) => !Object.hasOwn(obj, k));
  const extra = Object.keys(obj).filter((k) => !expected.includes(k));
  return [
    ...missing.map((k) => `${rule}: ${where}: missing field ${k}`),
    ...extra.map((k) => `${rule}: ${where}: unexpected field ${JSON.stringify(k)}`),
  ];
}

function basisProblems(where, strength, basis) {
  if (!isObject(basis)) return [`EV-10: ${where}: basis: must be an object`];
  const shape = BASIS[strength];
  const problems = closedObjectProblems('EV-10', `${where}: basis`, basis, Object.keys(shape));
  for (const [name, { expects, ok }] of Object.entries(shape)) {
    if (Object.hasOwn(basis, name) && !ok(basis[name])) {
      problems.push(`EV-10: ${where}: basis.${name}: must be ${expects}`);
    }
  }
  return problems;
}

function trustedBaseProblems(where, trustedBase) {
  if (!Array.isArray(trustedBase) || trustedBase.length === 0) {
    return [`EV-11: ${where}: trusted_base: must be a non-empty array`];
  }
  const problems = [];
  const seen = new Set();
  trustedBase.forEach((entry, j) => {
    const at = `${where}: trusted_base[${j}]`;
    if (!isObject(entry)) {
      problems.push(`EV-11: ${at}: must be an object`);
      return;
    }
    problems.push(...closedObjectProblems('EV-11', at, entry, COMPONENT_FIELDS));
    for (const name of COMPONENT_FIELDS) {
      if (Object.hasOwn(entry, name) && !isNonEmpty(entry[name])) {
        problems.push(`EV-11: ${at}: ${name}: must be a non-empty string`);
      }
    }
    const component = field(entry, 'component');
    if (typeof component === 'string') {
      if (seen.has(component)) problems.push(`EV-11: ${at}: component: repeats an earlier component`);
      seen.add(component);
    }
  });
  return problems;
}

function rerunProblems(where, rerun) {
  if (!isObject(rerun)) return [`EV-12: ${where}: rerun: must be an object`];
  const problems = closedObjectProblems('EV-12', `${where}: rerun`, rerun, RERUN_FIELDS);
  if (Object.hasOwn(rerun, 'command') && !isNonEmpty(rerun.command)) {
    problems.push(`EV-12: ${where}: rerun.command: must be a non-empty string`);
  }
  if (Object.hasOwn(rerun, 'exit_code') && !(isInteger(rerun.exit_code) && rerun.exit_code >= 0 && rerun.exit_code <= 255)) {
    problems.push(`EV-12: ${where}: rerun.exit_code: must be an integer from 0 to 255`);
  }
  return problems;
}

function claimProblems(claim, index, seenIds) {
  const id = field(claim, 'id');
  const idIsValid = typeof id === 'string' && /^[a-z0-9][a-z0-9-]*$/.test(id);
  const where = `claim ${idIsValid ? id : `claims[${index}]`}`;
  const problems = closedObjectProblems('EV-5', where, claim, CLAIM_FIELDS);

  if (!idIsValid) {
    problems.push(`EV-6: ${where}: id: must be a string of lowercase letters, digits and "-" starting with a letter or digit`);
  } else if (seenIds.has(id)) {
    problems.push(`EV-6: claim claims[${index}]: id: repeats the id of an earlier claim`);
  }
  if (idIsValid) seenIds.add(id);

  if (!isNonEmpty(field(claim, 'statement'))) {
    problems.push(`EV-7: ${where}: statement: must be a non-empty string`);
  }
  const requirement = field(claim, 'requirement');
  if (requirement !== null && !isNonEmpty(requirement)) {
    problems.push(`EV-8: ${where}: requirement: must be null or a non-empty string`);
  }
  const strength = field(claim, 'strength');
  if (!isStrength(strength)) {
    problems.push(`EV-9: ${where}: strength: must be one of ${Object.keys(BASIS).map((s) => JSON.stringify(s)).join(', ')}`);
  } else {
    problems.push(...basisProblems(where, strength, field(claim, 'basis')));
  }
  problems.push(...trustedBaseProblems(where, field(claim, 'trusted_base')));
  problems.push(...rerunProblems(where, field(claim, 'rerun')));
  return problems;
}

export function checkRecord(value) {
  if (!isObject(value)) return ['EV-1: record: must be a JSON object'];
  const problems = closedObjectProblems('EV-1', 'record', value, RECORD_FIELDS);
  if (field(value, 'format') !== FORMAT) {
    problems.push(`EV-2: record: format: must be the string ${JSON.stringify(FORMAT)}`);
  }
  const commit = field(value, 'commit');
  if (typeof commit !== 'string' || !/^[0-9a-f]{40}$/.test(commit)) {
    problems.push('EV-3: record: commit: must be 40 lowercase hex characters');
  }
  const claims = field(value, 'claims');
  if (!Array.isArray(claims) || claims.length === 0) {
    problems.push('EV-4: record: claims: must be a non-empty array');
  }
  if (Array.isArray(claims)) {
    const seenIds = new Set();
    claims.forEach((claim, index) => {
      if (isObject(claim)) problems.push(...claimProblems(claim, index, seenIds));
      else problems.push(`EV-5: claim claims[${index}]: must be an object`);
    });
  }
  return problems;
}

// ignoreBOM keeps a leading byte order mark in the string, so JSON.parse rejects it.
function readRecord(path) {
  const bytes = readFileSync(path);
  const text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes);
  return JSON.parse(text);
}

function main() {
  const path = process.argv[2];
  if (path === undefined) {
    console.error('usage: check-evidence-record.mjs <path>');
    process.exit(2);
  }
  let record;
  try {
    record = readRecord(path);
  } catch (err) {
    console.error(`${path}: ${err.message}`);
    process.exit(2);
  }
  const problems = checkRecord(record);
  for (const problem of problems) console.log(problem);
  process.exitCode = problems.length > 0 ? 1 : 0;
}

function isMain() {
  if (!process.argv[1]) return false;
  try {
    return realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url));
  } catch {
    return false;
  }
}

if (isMain()) main();
