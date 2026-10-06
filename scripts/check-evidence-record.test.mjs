// Tests for check-evidence-record.mjs. Run:
//
//   node --test scripts/check-evidence-record.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const CHECKER = join(dirname(fileURLToPath(import.meta.url)), 'check-evidence-record.mjs');
const WORK_DIR = mkdtempSync(join(tmpdir(), 'evidence-record-'));
let fileCount = 0;

const VALID = {
  format: 'evidence-record/1',
  commit: '0123456789abcdef0123456789abcdef01234567',
  claims: [
    {
      id: 'proved-example',
      statement: 'The example function never returns a negative number.',
      requirement: null,
      strength: 'proved',
      basis: { theorems: ['Example.nonneg'] },
      trusted_base: [{ component: 'Lean 4 kernel', version: 'leanprover/lean4:v4.28.0' }],
      rerun: { command: 'lake build', exit_code: 0 },
    },
    {
      id: 'tested-example',
      statement: 'The example function agrees with its model on sampled inputs.',
      requirement: 'docs/example.md#agreement',
      strength: 'tested',
      basis: { cases: 1000, seed: '42' },
      trusted_base: [{ component: 'Example input generator', version: '1.2.3' }],
      rerun: { command: 'npm test', exit_code: 0 },
    },
    {
      id: 'observed-example',
      statement: 'The example endpoint answered 200 against the staging system.',
      requirement: null,
      strength: 'observed',
      basis: { source: 'Acceptance run 1234 against staging on 2026-10-06' },
      trusted_base: [{ component: 'Staging deployment', version: 'build-5678' }],
      rerun: { command: 'cat acceptance/run-1234.log', exit_code: 0 },
    },
    {
      id: 'judged-example',
      statement: 'The example design is acceptable for release.',
      requirement: null,
      strength: 'judged',
      basis: { judge: 'A. Reviewer', date: '2026-10-06' },
      trusted_base: [
        { component: "Reviewer's reading of the design doc", version: 'design-doc commit 89abcdef' },
      ],
      rerun: { command: 'cat approvals/design.txt', exit_code: 0 },
    },
  ],
};

function mutated(edit) {
  const record = structuredClone(VALID);
  edit(record);
  return record;
}

const [PROVED, TESTED, OBSERVED, JUDGED] = [0, 1, 2, 3];

// JSON.stringify would normalise a written number, so the edit plants a
// sentinel string and the raw text replaces it afterwards.
const SENTINEL = '@@raw@@';
function withRaw(edit, raw) {
  return JSON.stringify(mutated(edit), null, 2).replace(`"${SENTINEL}"`, raw);
}

function runOnBytes(bytes) {
  const file = join(WORK_DIR, `record-${fileCount++}.json`);
  writeFileSync(file, bytes);
  return runWithArgs([file]);
}

function runWithArgs(args) {
  const result = spawnSync(process.execPath, [CHECKER, ...args], { encoding: 'utf8' });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr };
}

function run(record) {
  return runOnBytes(typeof record === 'string' ? record : JSON.stringify(record, null, 2));
}

const names = (result, rule) => new RegExp(`^${rule}:`, 'm').test(result.stdout);
const namesWith = (result, rule, text) =>
  result.stdout.split('\n').some((line) => line.startsWith(`${rule}:`) && line.includes(text));

function expectOutcome(result, { status, names: named = [], absent = [] }) {
  assert.equal(result.status, status, `exit code; stdout: ${result.stdout}; stderr: ${result.stderr}`);
  for (const rule of named) assert.ok(names(result, rule), `${rule} named; stdout: ${result.stdout}`);
  for (const rule of absent) assert.ok(!names(result, rule), `${rule} not named; stdout: ${result.stdout}`);
}

const ALL_RULES = Array.from({ length: 12 }, (_, i) => `EV-${i + 1}`);
const noRulesBut = (...kept) => ALL_RULES.filter((r) => !kept.includes(r));

const CASES = [
  { name: 'the valid record exits 0', record: VALID, status: 0, absent: ALL_RULES },
  {
    name: 'a claim with no strength names EV-5 and EV-9',
    record: mutated((r) => delete r.claims[TESTED].strength),
    status: 1,
    names: ['EV-5', 'EV-9'],
  },
  {
    name: 'a claim with strength "verified" and a tested basis names EV-9 and not EV-10',
    record: mutated((r) => (r.claims[TESTED].strength = 'verified')),
    status: 1,
    names: ['EV-9'],
    absent: ['EV-10'],
  },
  {
    name: 'a claim with strength "verified" and an unknown basis names EV-9 and not EV-10',
    record: mutated((r) => {
      r.claims[TESTED].strength = 'verified';
      r.claims[TESTED].basis = { bogus: 1 };
    }),
    status: 1,
    names: ['EV-9'],
    absent: ['EV-10'],
  },
  {
    name: 'a claim with no rerun names EV-5 and EV-12',
    record: mutated((r) => delete r.claims[PROVED].rerun),
    status: 1,
    names: ['EV-5', 'EV-12'],
  },
  {
    name: 'a blank rerun.command names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.command = '  ')),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'a valid strength with no basis names EV-5 and EV-10',
    record: mutated((r) => delete r.claims[TESTED].basis),
    status: 1,
    names: ['EV-5', 'EV-10'],
  },
  {
    name: 'a tested basis with theorems instead of cases and seed names EV-10',
    record: mutated((r) => (r.claims[TESTED].basis = { theorems: ['Example.nonneg'] })),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a judged date of 2026-02-30 names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2026-02-30')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a blank judge names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.judge = '  ')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a commit of 39 hex characters names EV-3',
    record: mutated((r) => (r.commit = VALID.commit.slice(1))),
    status: 1,
    names: ['EV-3'],
  },
  {
    name: 'a commit with an upper-case letter names EV-3',
    record: mutated((r) => (r.commit = VALID.commit.replace('a', 'A'))),
    status: 1,
    names: ['EV-3'],
  },
  {
    name: 'a blank statement names EV-7',
    record: mutated((r) => (r.claims[PROVED].statement = '   ')),
    status: 1,
    names: ['EV-7'],
  },
  {
    name: 'a blank requirement names EV-8',
    record: mutated((r) => (r.claims[PROVED].requirement = '  ')),
    status: 1,
    names: ['EV-8'],
  },
  {
    name: 'an empty trusted_base names EV-11',
    record: mutated((r) => (r.claims[PROVED].trusted_base = [])),
    status: 1,
    names: ['EV-11'],
  },
  {
    name: 'two trusted_base components with the same name name EV-11',
    record: mutated((r) =>
      r.claims[PROVED].trusted_base.push({ component: 'Lean 4 kernel', version: 'other' })
    ),
    status: 1,
    names: ['EV-11'],
  },
  {
    name: 'cases of 1.5 names EV-10',
    record: mutated((r) => (r.claims[TESTED].basis.cases = 1.5)),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'an exit_code of true names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.exit_code = true)),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'an extra top-level field names EV-1',
    record: mutated((r) => (r.claimz = [])),
    status: 1,
    names: ['EV-1'],
  },
  {
    name: 'format evidence-record/2 names EV-2',
    record: mutated((r) => (r.format = 'evidence-record/2')),
    status: 1,
    names: ['EV-2'],
  },
  {
    name: 'an empty claims array names EV-4',
    record: mutated((r) => (r.claims = [])),
    status: 1,
    names: ['EV-4'],
  },
  {
    name: 'a record that is a JSON array names EV-1 and EV-2 to EV-12 are not named',
    record: '[]',
    status: 1,
    names: ['EV-1'],
    absent: noRulesBut('EV-1'),
  },
  {
    name: 'claims holding the string "x" names EV-5 and not EV-6 to EV-12',
    record: mutated((r) => (r.claims = ['x'])),
    status: 1,
    names: ['EV-5'],
    absent: ['EV-6', 'EV-7', 'EV-8', 'EV-9', 'EV-10', 'EV-11', 'EV-12'],
  },
  {
    name: 'a statement of U+0085 alone is blank and names EV-7',
    record: mutated((r) => (r.claims[PROVED].statement = '\u0085')),
    status: 1,
    names: ['EV-7'],
  },
  {
    name: 'a statement of U+FEFF alone is not blank and exits 0',
    record: mutated((r) => (r.claims[PROVED].statement = '\uFEFF')),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'cases 1e999 names EV-10 and exits 1, not 2',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '1e999'),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'cases 1e30 names EV-10',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '1e30'),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'exit_code 1e-999 reads as 0 and exits 0',
    record: withRaw((r) => (r.claims[PROVED].rerun.exit_code = SENTINEL), '1e-999'),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'cases 1e-999 reads as 0 and names EV-10',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '1e-999'),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'cases 9007199254740993 reads as 2^53 and exits 0',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '9007199254740993'),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'cases 9007199254740994 names EV-10',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '9007199254740994'),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'cases written 1.0 is accepted',
    record: withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), '1.0'),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'exit_code written 1e0 is accepted',
    record: withRaw((r) => (r.claims[PROVED].rerun.exit_code = SENTINEL), '1e0'),
    status: 0,
    absent: ALL_RULES,
  },

  {
    name: 'a missing top-level format names EV-1 and EV-2',
    record: mutated((r) => delete r.format),
    status: 1,
    names: ['EV-1', 'EV-2'],
  },
  {
    name: 'a missing commit names EV-1 and EV-3',
    record: mutated((r) => delete r.commit),
    status: 1,
    names: ['EV-1', 'EV-3'],
  },
  {
    name: 'a missing claims names EV-1 and EV-4 and no per-claim rule',
    record: mutated((r) => delete r.claims),
    status: 1,
    names: ['EV-1', 'EV-4'],
    absent: ['EV-5', 'EV-6', 'EV-7', 'EV-8', 'EV-9', 'EV-10', 'EV-11', 'EV-12'],
  },
  {
    name: 'claims that is not an array names EV-4',
    record: mutated((r) => (r.claims = { a: 1 })),
    status: 1,
    names: ['EV-4'],
    absent: ['EV-5'],
  },
  {
    name: 'a top-level field named __proto__ names EV-1',
    record: JSON.stringify(VALID).replace('{', '{"__proto__":1,'),
    status: 1,
    names: ['EV-1'],
  },
  {
    name: 'a misspelled claim field names EV-5 and EV-9',
    record: mutated((r) => {
      r.claims[TESTED].strenght = r.claims[TESTED].strength;
      delete r.claims[TESTED].strength;
    }),
    status: 1,
    names: ['EV-5', 'EV-9'],
  },
  {
    name: 'an id with an upper-case letter names EV-6',
    record: mutated((r) => (r.claims[PROVED].id = 'Proved')),
    status: 1,
    names: ['EV-6'],
  },
  {
    name: 'an id starting with a dash names EV-6',
    record: mutated((r) => (r.claims[PROVED].id = '-proved')),
    status: 1,
    names: ['EV-6'],
  },
  {
    name: 'a missing id names EV-5 and EV-6',
    record: mutated((r) => delete r.claims[PROVED].id),
    status: 1,
    names: ['EV-5', 'EV-6'],
  },
  {
    name: 'a requirement that is a number names EV-8',
    record: mutated((r) => (r.claims[PROVED].requirement = 7)),
    status: 1,
    names: ['EV-8'],
  },
  {
    name: 'a missing requirement names EV-5 and EV-8',
    record: mutated((r) => delete r.claims[PROVED].requirement),
    status: 1,
    names: ['EV-5', 'EV-8'],
  },
  {
    name: 'a strength that is not a string names EV-9',
    record: mutated((r) => (r.claims[PROVED].strength = 3)),
    status: 1,
    names: ['EV-9'],
    absent: ['EV-10'],
  },
  {
    name: 'a strength of __proto__ names EV-9',
    record: mutated((r) => (r.claims[PROVED].strength = '__proto__')),
    status: 1,
    names: ['EV-9'],
    absent: ['EV-10'],
  },
  {
    name: 'a basis that is an array names EV-10',
    record: mutated((r) => (r.claims[PROVED].basis = [])),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'an empty theorems array names EV-10',
    record: mutated((r) => (r.claims[PROVED].basis.theorems = [])),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a blank theorem name names EV-10',
    record: mutated((r) => (r.claims[PROVED].basis.theorems = ['A.b', ' '])),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'an extra basis field names EV-10',
    record: mutated((r) => (r.claims[PROVED].basis.extra = 1)),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'cases of 0 names EV-10',
    record: mutated((r) => (r.claims[TESTED].basis.cases = 0)),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'an empty seed names EV-10',
    record: mutated((r) => (r.claims[TESTED].basis.seed = '')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a null seed is accepted',
    record: mutated((r) => (r.claims[TESTED].basis.seed = null)),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'a missing seed names EV-10',
    record: mutated((r) => delete r.claims[TESTED].basis.seed),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a blank observed source names EV-10',
    record: mutated((r) => (r.claims[OBSERVED].basis.source = '\t')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date of 2025-02-29 names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2025-02-29')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date of 2024-02-29 is accepted',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2024-02-29')),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'a date of 1900-02-29 names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '1900-02-29')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date of 2000-02-29 is accepted',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2000-02-29')),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'a date of 0000-01-01 names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '0000-01-01')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date of 2026-13-01 names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2026-13-01')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date without zero padding names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2026-1-01')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date written in full-width digits names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '\uFF12\uFF10\uFF12\uFF16-10-06')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a date with a trailing newline names EV-10',
    record: mutated((r) => (r.claims[JUDGED].basis.date = '2026-10-06\n')),
    status: 1,
    names: ['EV-10'],
  },
  {
    name: 'a commit with a trailing newline names EV-3',
    record: mutated((r) => (r.commit = `${VALID.commit}\n`)),
    status: 1,
    names: ['EV-3'],
  },
  {
    name: 'a trusted_base element with an extra field names EV-11',
    record: mutated((r) => (r.claims[PROVED].trusted_base[0].extra = 'x')),
    status: 1,
    names: ['EV-11'],
  },
  {
    name: 'a blank trusted_base version names EV-11',
    record: mutated((r) => (r.claims[PROVED].trusted_base[0].version = ' ')),
    status: 1,
    names: ['EV-11'],
  },
  {
    name: 'a trusted_base element that is a string names EV-11',
    record: mutated((r) => (r.claims[PROVED].trusted_base = ['Lean'])),
    status: 1,
    names: ['EV-11'],
  },
  {
    name: 'a missing trusted_base names EV-5 and EV-11',
    record: mutated((r) => delete r.claims[PROVED].trusted_base),
    status: 1,
    names: ['EV-5', 'EV-11'],
  },
  {
    name: 'an exit_code of 256 names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.exit_code = 256)),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'an exit_code of 255 is accepted',
    record: mutated((r) => (r.claims[PROVED].rerun.exit_code = 255)),
    status: 0,
    absent: ALL_RULES,
  },
  {
    name: 'an exit_code of -1 names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.exit_code = -1)),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'an extra rerun field names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.shell = 'sh')),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'a rerun that is a string names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun = 'lake build')),
    status: 1,
    names: ['EV-12'],
  },
  {
    name: 'a command of U+3000 alone is blank and names EV-12',
    record: mutated((r) => (r.claims[PROVED].rerun.command = '\u3000')),
    status: 1,
    names: ['EV-12'],
  },
];

for (const { name, record, ...expected } of CASES) {
  test(name, () => expectOutcome(run(record), expected));
}

test('two claims with the same id name EV-6 and the second claim by its index', () => {
  const result = run(mutated((r) => (r.claims[TESTED].id = 'proved-example')));
  expectOutcome(result, { status: 1, names: ['EV-6'] });
  assert.ok(namesWith(result, 'EV-6', 'claims[1]'), result.stdout);
  assert.ok(!namesWith(result, 'EV-6', 'claims[0]'), result.stdout);
});

test('a third claim repeating an id is reported once for each later claim', () => {
  const result = run(
    mutated((r) => {
      r.claims[OBSERVED].id = 'proved-example';
      r.claims[TESTED].id = 'proved-example';
    })
  );
  const lines = result.stdout.split('\n').filter((l) => l.startsWith('EV-6:'));
  assert.equal(result.status, 1);
  assert.equal(lines.length, 2);
  assert.ok(lines[0].includes('claims[1]') && lines[1].includes('claims[2]'), result.stdout);
});

test('three broken claims report all three', () => {
  const result = run(
    mutated((r) => {
      r.claims[PROVED].strength = 'verified';
      r.claims[TESTED].statement = ' ';
      r.claims[OBSERVED].rerun.exit_code = 256;
    })
  );
  expectOutcome(result, { status: 1, names: ['EV-9', 'EV-7', 'EV-12'] });
  assert.ok(namesWith(result, 'EV-9', 'claim proved-example'), result.stdout);
  assert.ok(namesWith(result, 'EV-7', 'claim tested-example'), result.stdout);
  assert.ok(namesWith(result, 'EV-12', 'claim observed-example'), result.stdout);
});

test('a claim whose id is broken is named by its index', () => {
  const result = run(mutated((r) => (r.claims[JUDGED].id = 'Bad Id')));
  assert.ok(namesWith(result, 'EV-6', 'claims[3]'), result.stdout);
});

test('a rerun.command that would create a file exits 0 and creates nothing', () => {
  const target = join(WORK_DIR, 'created-by-record');
  const result = run(mutated((r) => (r.claims[PROVED].rerun.command = `touch ${target}`)));
  expectOutcome(result, { status: 0 });
  assert.equal(existsSync(target), false);
});

test('a file that is not JSON exits 2', () => {
  assert.equal(run('this is not json').status, 2);
});

test('an empty file exits 2', () => {
  assert.equal(runOnBytes('').status, 2);
});

test('a run with no path argument exits 2', () => {
  const result = runWithArgs([]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /usage/);
});

test('a path that cannot be read exits 2', () => {
  assert.equal(runWithArgs([join(WORK_DIR, 'no-such-file.json')]).status, 2);
});

test('a leading byte order mark exits 2', () => {
  const bytes = Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from(JSON.stringify(VALID))]);
  assert.equal(runOnBytes(bytes).status, 2);
});

test('a NaN token exits 2', () => {
  assert.equal(run(withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), 'NaN')).status, 2);
});

test('an Infinity token exits 2', () => {
  assert.equal(run(withRaw((r) => (r.claims[TESTED].basis.cases = SENTINEL), 'Infinity')).status, 2);
});

test('bytes that are not UTF-8 exit 2', () => {
  assert.equal(runOnBytes(Buffer.from([0x22, 0xff, 0x22])).status, 2);
});

// Every code point of the Unicode White_Space property, so a narrowed NON_BLANK
// class in the checker fails a test.
const WHITE_SPACE = [
  ...Array.from({ length: 5 }, (_, i) => 0x09 + i),
  0x20,
  0x85,
  0xa0,
  0x1680,
  ...Array.from({ length: 11 }, (_, i) => 0x2000 + i),
  0x2028,
  0x2029,
  0x202f,
  0x205f,
  0x3000,
];
// Neighbours of the set that are not White_Space, so a widened class fails too.
const NOT_WHITE_SPACE = [0x08, 0x0e, 0x1c, 0x21, 0x180e, 0x200b, 0x2010, 0x2060, 0xfeff];

for (const code of WHITE_SPACE) {
  const label = `U+${code.toString(16).toUpperCase().padStart(4, '0')}`;
  test(`a statement of ${label} alone is blank and names EV-7`, () => {
    const result = run(mutated((r) => (r.claims[PROVED].statement = String.fromCodePoint(code))));
    expectOutcome(result, { status: 1, names: ['EV-7'] });
  });
}

for (const code of NOT_WHITE_SPACE) {
  const label = `U+${code.toString(16).toUpperCase().padStart(4, '0')}`;
  test(`a statement of ${label} alone is not blank and passes`, () => {
    const result = run(mutated((r) => (r.claims[PROVED].statement = String.fromCodePoint(code))));
    expectOutcome(result, { status: 0, absent: ALL_RULES });
  });
}

test('a claim with neither strength nor basis names EV-5 and EV-9 and not EV-10', () => {
  const result = run(
    mutated((r) => {
      delete r.claims[TESTED].strength;
      delete r.claims[TESTED].basis;
    })
  );
  expectOutcome(result, { status: 1, names: ['EV-5', 'EV-9'], absent: ['EV-10'] });
});

test('a record that writes strength twice is judged on the last value (duplicate keys are accepted)', () => {
  const text = JSON.stringify(VALID, null, 2).replace(
    '"strength": "tested"',
    '"strength": "verified", "strength": "tested"'
  );
  assert.ok(text.includes('"verified"'), 'the duplicate key was planted');
  expectOutcome(run(text), { status: 0, absent: ALL_RULES });
});

test('stdout is empty on exit 0 and on exit 2, and holds only EV-N lines on exit 1', () => {
  assert.equal(run(VALID).stdout, '');
  assert.equal(run('this is not json').stdout, '');
  assert.equal(runWithArgs([]).stdout, '');
  const broken = run(
    mutated((r) => {
      r.claims[PROVED].strength = 'verified';
      r.claims[TESTED].statement = ' ';
    })
  );
  assert.equal(broken.status, 1);
  const lines = broken.stdout.split('\n').filter((l) => l !== '');
  assert.ok(lines.length >= 2, broken.stdout);
  for (const line of lines) assert.match(line, /^EV-\d+: /);
  assert.equal(broken.stderr, '');
});
