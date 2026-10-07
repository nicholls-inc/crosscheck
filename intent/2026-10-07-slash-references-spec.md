# Spec: Check every slash-reference in Crosscheck's Markdown

Intent: `intent/2026-10-07-slash-references.md`. Governing roadmap item: PB-1. Task: PB-1.18.

This spec adds SR-1 to SR-10. The PC requirements (`intent/2026-10-06-pre-commit-hooks-spec.md`) do not change, except that PC-6 gains one more trigger (SR-8).

## Terms

- **Index.** The files as staged, read with `git ls-files -z --cached` and `git cat-file --batch`. In a CI checkout the index equals `HEAD`.
- **Skill.** A name `<n>` such that `crosscheck/skills/<n>/SKILL.md` is in the index.
- **Agent.** A name `<n>` such that `crosscheck/agents/<n>.md` is in the index.
- **Allowlist.** `crosscheck/slash-allowlist.txt`.

## Requirements

- **SR-1. Inputs.** The checker reads the index, never the working tree, so the hook and CI read the same thing. An untracked file is not read.
- **SR-2. Checked files.** Every path in the index that starts `crosscheck/` and ends `.md`, except a path under one of these prefixes:
  - `crosscheck/docs/add/.retrospective/`, archived records of designs that name skills as they were planned;
  - `crosscheck/.assurance/`, dated run snapshots;
  - `crosscheck/docs/research/` and `crosscheck/docs/reports/`, research notes that propose skills;
  - `crosscheck/docs/examples/workflows/`, gh-aw workflow sources whose `/assurance-recheck` and `/assurance-squad` are GitHub comment commands, not Claude Code skills.

  It also skips three dated records that name a skill as it was planned: `crosscheck/JOURNAL.md`, `crosscheck/skills/JOURNAL.md` and `crosscheck/docs/specs/rationale-2026-05-11.md`. Rewriting a record falsifies it, so the record keeps its text and the check skips the file, by the maintainer's decision on this pull request. A sibling path, such as a later dated snapshot under `crosscheck/docs/specs/`, is checked.

  Both lists are constants in `scripts/ci/skill-references.mjs`, so a change to either is a Class A amendment.
- **SR-3. Grammar.** A name is `[a-z][a-z0-9]*(-[a-z0-9]+)*`: lower case, no trailing or doubled hyphen. A reference is `/`, then an optional `<plugin>:` where the plugin is a name, then a name, with both conditions:
  - the character before `/` is not a letter, digit, `_`, `/`, `.`, `-`, `~`, `:`, `<`, `*`, `\`, `$`, `}`, `)`, `]`, `@`, `%`, `+` or `=`. This leaves out URLs, paths such as `src/x` and `packages/*/src`, `</summary>`, and `${HOME}/x`;
  - the character after the name is not a letter, digit, `_`, `/`, `-`, `*`, `<`, `>`, `:` or `\`, and not a `.` followed by a letter, digit or `_`. This leaves out `/tmp/x`, `/x.md`, `/assurance-*` and the regex `/lemma\s+/g`, and keeps a reference that ends a sentence.

  Fenced code blocks are checked like prose.
- **SR-4. Resolution.** A reference with no prefix, or with the prefix `crosscheck`, resolves when its name is a skill or an agent. A reference with another prefix resolves when the allowlist has the line `<plugin>:<name>`.
- **SR-5. Allowlist.** One `<plugin>:<name>` per line. Blank lines and lines that start `#` are ignored. A line that is not of that form, or whose plugin is `crosscheck`, is a finding: a `crosscheck` entry would let a name that has no skill or agent pass. The file ships with comments and no entries. A missing allowlist exits 2.
- **SR-6. Catalogue.** `crosscheck/docs/skills.md` must equal the text that `renderCatalogue` produces from the skills in the index, sorted by name. Each row links `/<n>` to `../skills/<n>/SKILL.md` and holds the `description` of the skill's frontmatter, folded to one line with each `|` escaped. The header states the count. A difference is a finding whose fix is `node scripts/ci/skill-references.mjs --write`. A `SKILL.md` with no frontmatter `description` is a finding.
- **SR-7. Output and exit codes.** Each finding is one line: `<path>:<line>: <reference> names no skill in crosscheck/skills/ and no agent in crosscheck/agents/` (or `... is not in crosscheck/slash-allowlist.txt` for another plugin). The checker exits 0 with no findings, 1 with findings, and 2 when git cannot read the index. On a failure it prints the four gate lines (`**Action needed: ...**`, the decision, its costs, `Full explanation:` linking `docs/gates/skill-references.md`), then the findings. `--write` regenerates `crosscheck/docs/skills.md` in the working tree and then checks.
- **SR-8. Pre-commit.** `scripts/ci/pre-commit.mjs` runs the check when the staged set has a path that starts `crosscheck/` and ends `.md`, or the allowlist. It prints the findings with a `Fix:` line, the rerun command and the explainer, as PC-5 says. It reads nothing more when neither is staged (PC-6).
- **SR-9. CI.** `.github/workflows/skill-references.yml` runs `node scripts/ci/skill-references.mjs` on every pull request and on every push to `main`.
- **SR-10. Tests.** `scripts/ci/skill-references.test.mjs`, run by the Tier Gate workflow's `node --test scripts/ci/*.test.mjs`:
  - a table of literal positive and negative lines for SR-3, each with the exact references it must yield;
  - the committed fixture `scripts/ci/fixtures/skill-references/`, copied into a scratch repository and staged, exits 1 and prints `crosscheck/docs/broken.md:` with the line of `/no-such-skill`, and the same tree with that reference removed exits 0;
  - a reference to an agent, to `crosscheck:<skill>`, and to another plugin's skill with and without an allowlist line;
  - an allowlist `crosscheck:` entry and a malformed line are findings;
  - a stale catalogue is a finding, and `--write` fixes it;
  - a file under an SR-2 prefix, or one of the three SR-2 records, is not read;
  - the real repository passes.

  `scripts/ci/pre-commit.test.mjs` adds a staged Markdown file under `crosscheck/` with a broken reference, which fails the commit and names the file and line.

## Concerns flagged, not resolved here

- **Files not checked.** The decision for #36 named all of `crosscheck/**/*.md`. SR-2 leaves out archives, snapshots, research notes, gh-aw examples and three dated records, where about 150 mentions name skills that were planned or never built, or GitHub comment commands. Rewriting a historical record to satisfy a checker would falsify it. Checking these files is not yet reached. The property that blocks it is that a historical mention and an instruction look the same in the text. The open question is how a document marks a mention as historical so a checker can tell the two apart. The maintainer decided that the three records stay as written and are skipped. Whether the five directories stay skipped, or some of their files are rewritten instead, is still the maintainer's call (the open marker in the governance note).
- **Catalogue shape.** The hand-written catalogue grouped skills by category with trigger phrases and an owner. None of those is in `crosscheck/skills/`, so the generated catalogue is one table of names and frontmatter descriptions. Categories would need a new frontmatter key in all 30 `SKILL.md` files, or a second source file that could drift on its own.
- **Live text that named unbuilt skills.** Four live documents named a skill that does not exist (`/journal-lint`, `/crosscheck-gc`, the four greenfield skills of ADR-004), and `crosscheck/conformance/README.md` used `/skill` and `/x` as placeholders. This change drops the slash from the unbuilt names and writes the placeholders as `/<skill>`, so the text no longer reads as an instruction to run them. Three dated records also named unbuilt skills (`/journal-lint`, `/rationale-adversary`). They keep their text, and SR-2 skips them.
- **Overlap with conformance.** AUTO 2 and AUTO 5 still run. PB-1.19 aligns AUTO 5 with this grammar and allowlist.
- The hook runs only where `npm install` ran, and `--no-verify` skips it. CI is the check every pull request passes through.
