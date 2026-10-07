// Tests for scripts/ci/skill-references.mjs (SR-1 to SR-10, see
// intent/2026-10-07-slash-references-spec.md).
// Run: node --test scripts/ci/*.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkSnapshot, findReferences, parseAllowlist, parseDescription, renderCatalogue } from './skill-references.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(HERE, '../..');
const SCRIPT = join(HERE, 'skill-references.mjs');
const FIXTURE = join(HERE, 'fixtures/skill-references');
const GIT_ENV = {
  ...process.env,
  GIT_AUTHOR_NAME: 'test',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'test',
  GIT_COMMITTER_EMAIL: 'test@example.com',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};
const NO_SKILL = 'names no skill in crosscheck/skills/ and no agent in crosscheck/agents/';
const GATE_LINES = [
  '**Action needed: fix the slash-references**',
  "You are being asked to correct a slash-reference in Crosscheck's Markdown, the allowlist, or the generated skill catalogue, because a /name there runs no skill or agent that exists, or the catalogue is out of date.",
  'Approving means every /name a reader or agent follows in crosscheck/ runs something that exists and crosscheck/docs/skills.md lists every skill; declining leaves this check red.',
  'Full explanation: docs/gates/skill-references.md.',
  '',
];
const ALPHA_SKILL = '---\nname: alpha\ndescription: >-\n  Alpha checks\n  one claim.\n---\n# /alpha\n';

const ref = (plugin, name, line = 1) => ({ line, plugin, name, text: plugin ? `/${plugin}:${name}` : `/${name}` });

const GRAMMAR = [
  ['Run /reason now.', [ref(null, 'reason')]],
  ['End of a sentence: /reason.', [ref(null, 'reason')]],
  ['(/reason) and `/reason`', [ref(null, 'reason'), ref(null, 'reason')]],
  ['[see](/reason)', [ref(null, 'reason')]],
  ['/crosscheck:spec-iterate', [ref('crosscheck', 'spec-iterate')]],
  ['/other-plugin:do-thing2, then /a-b-c', [ref('other-plugin', 'do-thing2'), ref(null, 'a-b-c')]],
  ['"/reason", /reason; /reason?', [ref(null, 'reason'), ref(null, 'reason'), ref(null, 'reason')]],
  ['https://example.com/reason', []],
  ['src/reason and packages/*/src', []],
  ['</summary>', []],
  ['${HOME}/reason', []],
  ['~/reason ./reason ../reason', []],
  ['//cdn.example and a//reason', []],
  ['a:/reason a-/reason a_/reason', []],
  ['@scope/reason 50%/reason a+/reason a=/reason', []],
  ['**/reason** x)/reason x]/reason x}/reason \\/reason', []],
  ['/tmp/x', []],
  ['/x.md and /reason.v2', []],
  ['/assurance-*', []],
  ['/lemma\\s+/g', []],
  ['/Reason /reasoN /foo_bar /foo- /foo--bar /9lives', []],
  ['/reason: /reason< /reason> /reason\\n', []],
  ['/crosscheck:Reason', []],
  ['$/reason @/reason /reason* x*/reason', []],
];

for (const [line, expected] of GRAMMAR) {
  test(`SR-3: ${JSON.stringify(line)} yields ${expected.map((r) => r.text).join(', ') || 'no reference'}`, () => {
    assert.deepEqual(findReferences(line), expected);
  });
}

test('SR-3: line numbers are 1-based and a fenced code block is read like prose', () => {
  assert.deepEqual(findReferences('# Title\nRun /alpha.\n\n```\n/beta\n```\n'), [ref(null, 'alpha', 2), ref(null, 'beta', 5)]);
});

test('SR-5: the allowlist keeps <plugin>:<name> lines and reports malformed and crosscheck lines', () => {
  const { entries, problems } = parseAllowlist('# comment\n\nother:thing\n  other-plugin:two  \nnot valid\ncrosscheck:alpha\nOther:thing\nother:thing garbage\n');
  assert.deepEqual([...entries], ['other:thing', 'other-plugin:two']);
  assert.deepEqual(problems, [
    'crosscheck/slash-allowlist.txt:5: "not valid" is not of the form <plugin>:<name>',
    'crosscheck/slash-allowlist.txt:6: crosscheck:alpha is a crosscheck entry, which would let a name with no skill or agent pass; delete the line',
    'crosscheck/slash-allowlist.txt:7: "Other:thing" is not of the form <plugin>:<name>',
    'crosscheck/slash-allowlist.txt:8: "other:thing garbage" is not of the form <plugin>:<name>',
  ]);
});

test('SR-6: parseDescription folds a block scalar and reads an inline one', () => {
  assert.equal(parseDescription(ALPHA_SKILL), 'Alpha checks one claim.');
  assert.equal(parseDescription('---\nname: a\ndescription: >-\n  One\n  two.\nargument-hint: "x"\n---\n'), 'One two.');
  assert.equal(parseDescription('---\nname: a\ndescription: Plain text.\n---\n'), 'Plain text.');
  assert.equal(parseDescription("---\ndescription: 'Quoted: text.'\n---\n"), 'Quoted: text.');
  assert.equal(parseDescription('---\r\ndescription: >-\r\n  Windows\r\n  lines.\r\n---\r\n'), 'Windows lines.');
  assert.equal(parseDescription('---\nname: a\n---\ndescription: not frontmatter\n'), null);
  assert.equal(parseDescription('# no frontmatter\ndescription: x\n'), null);
});

test('SR-6: parseDescription reads every block form, double quotes, and returns null for an empty or unterminated description', () => {
  for (const indicator of ['>', '>-', '>+', '|', '|-', '|+']) {
    assert.equal(parseDescription(`---\ndescription: ${indicator}\n  One\n  two.\n---\n`), 'One two.', indicator);
  }
  assert.equal(parseDescription('---\ndescription: "Double: quoted."\n---\n'), 'Double: quoted.');
  assert.equal(parseDescription('---\ndescription:\n---\n'), null, 'an empty scalar is no description');
  assert.equal(parseDescription('---\ndescription: ""\n---\n'), null, 'an empty quoted value is no description');
  assert.equal(parseDescription('---\ndescription: >-\n---\n'), null, 'an empty block scalar is no description');
  assert.equal(parseDescription('---\ndescription: Never closed.\n'), null, 'frontmatter with no closing --- is not frontmatter');
  assert.equal(parseDescription('\n---\ndescription: Not first.\n---\n'), null, 'frontmatter must start on the first line');
  assert.equal(parseDescription('intro\ndescription: Not first.\n---\n'), null, 'a closing --- with no opening --- is not frontmatter');
});

test('SR-6: renderCatalogue sorts by name, links each skill and escapes a pipe', () => {
  const text = renderCatalogue(
    new Map([
      ['zeta', '---\ndescription: Last | first.\n---\n'],
      ['alpha', ALPHA_SKILL],
    ]),
  );
  assert.equal(
    text,
    [
      '# Crosscheck Skill Catalogue',
      '',
      "Index of all 2 skills in the crosscheck plugin: one row per directory under `crosscheck/skills/`, with the `description` from that skill's frontmatter.",
      '',
      'This file is generated by `node scripts/ci/skill-references.mjs --write`. Do not edit it by hand: CI fails when it differs from the generated text.',
      '',
      'See [`../README.md`](../README.md) for the plugin overview, and [`./agents.md`](./agents.md) for the orchestrator agent pages.',
      '',
      '| Skill | Description |',
      '| --- | --- |',
      '| [`/alpha`](../skills/alpha/SKILL.md) | Alpha checks one claim. |',
      '| [`/zeta`](../skills/zeta/SKILL.md) | Last \\| first. |',
      '',
    ].join('\n'),
  );
});

// An in-memory index with skill `alpha`, agent `beta`, a matching catalogue,
// and `extra` files.
function snapshot(extra = {}) {
  const skills = { 'crosscheck/skills/alpha/SKILL.md': ALPHA_SKILL };
  const files = new Map(Object.entries({ ...skills, 'crosscheck/agents/beta.md': '# beta\n', ...extra }));
  if (!files.has('crosscheck/docs/skills.md')) files.set('crosscheck/docs/skills.md', renderCatalogue(new Map([['alpha', ALPHA_SKILL]])));
  return { files, skills: new Set(['alpha']), agents: new Set(['beta']) };
}

test('SR-4: a skill, an agent and crosscheck:<skill> resolve, and an unknown name does not', () => {
  const problems = checkSnapshot(
    snapshot({ 'crosscheck/docs/a.md': 'Use /alpha, /beta, /crosscheck:alpha and /crosscheck:beta.\nNot /gamma or /crosscheck:gamma.\n' }),
    '',
  );
  assert.deepEqual(problems, [`crosscheck/docs/a.md:2: /gamma ${NO_SKILL}`, `crosscheck/docs/a.md:2: /crosscheck:gamma ${NO_SKILL}`]);
});

test("SR-4: another plugin's skill resolves only with its allowlist line", () => {
  const index = snapshot({ 'crosscheck/docs/a.md': 'Then run /other:thing.\n' });
  assert.deepEqual(checkSnapshot(index, '# none\n'), ['crosscheck/docs/a.md:1: /other:thing is not in crosscheck/slash-allowlist.txt']);
  assert.deepEqual(checkSnapshot(index, 'other:thing\n'), [], 'the allowlist line other:thing resolves it');
  assert.deepEqual(
    checkSnapshot(snapshot({ 'crosscheck/docs/a.md': 'Then run /other:alpha.\n' }), ''),
    ['crosscheck/docs/a.md:1: /other:alpha is not in crosscheck/slash-allowlist.txt'],
    'a crosscheck skill name under another plugin still needs an allowlist line',
  );
});

test('SR-5: allowlist problems are findings of the check', () => {
  assert.deepEqual(checkSnapshot(snapshot(), 'crosscheck:gamma\nbad line\n'), [
    'crosscheck/slash-allowlist.txt:1: crosscheck:gamma is a crosscheck entry, which would let a name with no skill or agent pass; delete the line',
    'crosscheck/slash-allowlist.txt:2: "bad line" is not of the form <plugin>:<name>',
  ]);
});

test('SR-2: a file under each prefix that is not checked is not read, and a sibling path is', () => {
  const broken = 'Run /gamma.\n';
  const problems = checkSnapshot(
    snapshot({
      'crosscheck/docs/add/.retrospective/a.md': broken,
      'crosscheck/.assurance/a.md': broken,
      'crosscheck/docs/research/a.md': broken,
      'crosscheck/docs/reports/a.md': broken,
      'crosscheck/docs/examples/workflows/a.md': broken,
      'crosscheck/docs/examples/a.md': broken,
      'crosscheck/docs/researcher.md': broken,
      'crosscheck/notes.txt': broken,
    }),
    '',
  );
  assert.deepEqual(problems, [`crosscheck/docs/examples/a.md:1: /gamma ${NO_SKILL}`, `crosscheck/docs/researcher.md:1: /gamma ${NO_SKILL}`]);
});

test('SR-6: a stale or missing catalogue and a skill with no description are findings', () => {
  const stale = 'crosscheck/docs/skills.md: differs from the catalogue generated from crosscheck/skills/; run node scripts/ci/skill-references.mjs --write';
  assert.deepEqual(checkSnapshot(snapshot({ 'crosscheck/docs/skills.md': '# hand-written\n' }), ''), [stale]);
  const missing = snapshot();
  missing.files.delete('crosscheck/docs/skills.md');
  assert.deepEqual(checkSnapshot(missing, ''), [stale]);
  const bare = '---\nname: alpha\n---\n';
  assert.deepEqual(
    checkSnapshot(
      snapshot({ 'crosscheck/skills/alpha/SKILL.md': bare, 'crosscheck/docs/skills.md': renderCatalogue(new Map([['alpha', bare]])) }),
      '',
    ),
    ['crosscheck/skills/alpha/SKILL.md: has no frontmatter description'],
  );
});

// A scratch repository holding the committed fixture, staged.
function fixtureRepo() {
  const work = mkdtempSync(join(tmpdir(), 'skill-references-test-'));
  execFileSync('git', ['init', '-q', '-b', 'main', work], { env: GIT_ENV });
  cpSync(FIXTURE, work, { recursive: true });
  git(work, ['add', '.']);
  return work;
}

function git(cwd, args) {
  return execFileSync('git', args, { cwd, env: GIT_ENV, encoding: 'utf8' });
}

function cli(cwd, args = []) {
  const r = spawnSync(process.execPath, [SCRIPT, ...args], { cwd, env: GIT_ENV, encoding: 'utf8' });
  return { status: r.status, stdout: r.stdout, stderr: r.stderr };
}

function stage(work, files) {
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(work, path)), { recursive: true });
    writeFileSync(join(work, path), content);
  }
  git(work, ['add', '--', ...Object.keys(files)]);
}

const BROKEN_FIXED = '# Broken\n\nRun /alpha, then hand off to /beta.\nThe plugin form /crosscheck:alpha works too.\n';

test('SR-7, SR-10: the fixture exits 1 naming broken.md:5, and exits 0 once that reference is gone', () => {
  const work = fixtureRepo();
  try {
    const failing = cli(work);
    assert.equal(failing.status, 1);
    assert.equal(failing.stdout, [...GATE_LINES, `crosscheck/docs/broken.md:5: /no-such-skill ${NO_SKILL}`, ''].join('\n'));
    stage(work, { 'crosscheck/docs/broken.md': BROKEN_FIXED });
    const passing = cli(work);
    assert.equal(passing.status, 0, passing.stdout);
    assert.equal(passing.stdout, 'skill-references: PASS - 11 reference(s) in 4 file(s) checked, 1 skill(s) catalogued.\n');
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});

test('SR-1: the check reads the index, so an untracked file and an unstaged edit are not read', () => {
  const work = fixtureRepo();
  try {
    stage(work, { 'crosscheck/docs/broken.md': BROKEN_FIXED });
    writeFileSync(join(work, 'crosscheck/docs/broken.md'), 'Run /gamma.\n');
    writeFileSync(join(work, 'crosscheck/docs/untracked.md'), 'Run /gamma.\n');
    const r = cli(work);
    assert.equal(r.status, 0, r.stdout);
    git(work, ['add', 'crosscheck/docs/broken.md']);
    const staged = cli(work);
    assert.equal(staged.status, 1, 'the same edit fails once it is staged');
    assert.match(staged.stdout, /^crosscheck\/docs\/broken\.md:1: \/gamma names no skill/m);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});

test('SR-6, SR-7: a stale catalogue fails, and --write regenerates it in the working tree', () => {
  const work = fixtureRepo();
  try {
    stage(work, {
      'crosscheck/docs/broken.md': BROKEN_FIXED,
      'crosscheck/skills/gamma/SKILL.md': '---\nname: gamma\ndescription: Gamma.\n---\n',
    });
    const stale = cli(work);
    assert.equal(stale.status, 1);
    assert.equal(
      stale.stdout,
      [...GATE_LINES, 'crosscheck/docs/skills.md: differs from the catalogue generated from crosscheck/skills/; run node scripts/ci/skill-references.mjs --write', ''].join('\n'),
    );
    const written = cli(work, ['--write']);
    assert.equal(written.status, 0, written.stdout);
    assert.match(written.stdout, /^skill-references: wrote crosscheck\/docs\/skills\.md; git add it so the index holds it$/m);
    const catalogue = readFileSync(join(work, 'crosscheck/docs/skills.md'), 'utf8');
    assert.match(catalogue, /^Index of all 2 skills in the crosscheck plugin/m);
    assert.match(catalogue, /^\| \[`\/gamma`\]\(\.\.\/skills\/gamma\/SKILL\.md\) \| Gamma\. \|$/m);
    assert.equal(cli(work).status, 1, 'the index still holds the stale catalogue until it is added');
    git(work, ['add', 'crosscheck/docs/skills.md']);
    assert.equal(cli(work).status, 0, 'the added catalogue passes');
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});

test('SR-5, SR-7: an allowlist missing from the index exits 2', () => {
  const work = fixtureRepo();
  try {
    git(work, ['rm', '-q', '--cached', 'crosscheck/slash-allowlist.txt']);
    const r = cli(work);
    assert.equal(r.status, 2);
    assert.equal(r.stdout, '');
    assert.equal(
      r.stderr,
      'crosscheck/slash-allowlist.txt is not in the index\nFix: git restore --source=HEAD --staged --worktree crosscheck/slash-allowlist.txt, or create it with comments only and git add crosscheck/slash-allowlist.txt\n',
    );
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});

test('SR-7: outside a git repository the check exits 2', () => {
  const dir = mkdtempSync(join(tmpdir(), 'skill-references-no-git-'));
  try {
    const r = spawnSync(process.execPath, [SCRIPT], { cwd: dir, env: { ...GIT_ENV, GIT_CEILING_DIRECTORIES: dirname(dir) }, encoding: 'utf8' });
    assert.equal(r.status, 2);
    assert.match(r.stderr, /^git ls-files -z --cached --stage -- crosscheck: fatal: not a git repository/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('SR-10: the real repository passes', () => {
  const r = cli(REPO_ROOT);
  assert.equal(r.status, 0, `${r.stdout}${r.stderr}`);
  assert.match(r.stdout, /^skill-references: PASS - \d+ reference\(s\) in \d+ file\(s\) checked, \d+ skill\(s\) catalogued\.$/m);
});
