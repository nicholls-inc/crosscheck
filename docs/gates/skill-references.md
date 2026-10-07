# Gate: Skill References Check (CI Failure)

## What this gate protects

Crosscheck's skills and agents tell the reader, often another agent, to run a skill by writing `/name`, such as `/reason`, or `/<plugin>:<name>`, such as `/crosscheck:reason`. A **skill** is a directory under `crosscheck/skills/` with a `SKILL.md`. An **agent** is a file under `crosscheck/agents/`. When a skill is renamed or deleted, every mention of it points at nothing, and an agent that follows the text fails or improvises.

The **skill references check** (`scripts/ci/skill-references.mjs`, run by `.github/workflows/skill-references.yml`) runs on every pull request and every push to `main`. It reads every Markdown file under `crosscheck/` as committed, and fails when:

- a `/name`, or a `/crosscheck:name`, names no skill and no agent;
- a `/<plugin>:<name>` for another plugin is not a line of the **allowlist**, `crosscheck/slash-allowlist.txt`, which lists the other plugins' skills that Crosscheck's text may name, one `<plugin>:<name>` per line;
- an allowlist line is malformed, or names the `crosscheck` plugin, which would let a missing skill pass;
- `crosscheck/docs/skills.md`, the skill catalogue, differs from the catalogue that the script generates from `crosscheck/skills/`.

Each finding names the file and line.

## What each decision means

- **Approving (fixing the reference or the catalogue)**: every reference in a checked file names a skill or agent that exists, and the catalogue lists every skill.
- **Declining (leaving it as it is)**: the check stays red. The ruleset on the default branch requires no status checks, so GitHub does not stop the merge. The maintainer does not merge while this check is red.

To fix a finding, do one of these:

- correct the name, if it is a typo or names a renamed skill;
- drop the slash, if the text mentions a skill that does not exist yet, such as a planned one, so that it does not read as an instruction to run it;
- add `<plugin>:<name>` to the allowlist, if it names another plugin's skill;
- run `node scripts/ci/skill-references.mjs --write`, then `git add crosscheck/docs/skills.md`, if the catalogue is stale. Do not edit the catalogue by hand. A skill's row comes from the `description` in its `SKILL.md`.

## What the check does not catch

- Files under `crosscheck/docs/add/.retrospective/`, `crosscheck/.assurance/`, `crosscheck/docs/research/`, `crosscheck/docs/reports/` and `crosscheck/docs/examples/workflows/` are not read. They are archives, dated snapshots, research notes and GitHub workflow examples, and they name skills that were planned and never built, or GitHub comment commands. Checking them is not yet reached: a historical mention and an instruction look the same in the text, and how a document should mark the difference is open.
- A reference with an upper-case letter, or one that follows a letter, `.`, `/` or `-` (for example `x/name`), is not read as a reference. The spec lists the full grammar (SR-3 in `intent/2026-10-07-slash-references-spec.md`).
- A bare agent name, such as `byfuglien`, without a slash. Task PB-1.19 adds that to conformance AUTO 5.

## Before you commit

`npm install` at the repository root installs a pre-commit hook, `.husky/pre-commit`. When a commit stages a Markdown file under `crosscheck/` or the allowlist, the hook runs this check on the files as staged. Run `node scripts/ci/pre-commit.mjs` to recheck after a fix.

## How long this takes

Usually a minute. Run `node scripts/ci/skill-references.mjs` at the repository root to see the same result. It reads the files as staged, so `git add` a fix before you rerun it.

## Who to ask if unsure

The Crosscheck maintainers, via a GitHub issue on this repository.
