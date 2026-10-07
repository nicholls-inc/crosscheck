# Spec: The commit-msg hook reads every staged file name as git stores it

Intent: `intent/2026-10-06-commit-msg-names.md`. Governing roadmap item: PB-1. Task: PB-1.13.

- **CM-1. Which commits are checked.** The check runs when the first line of the message starts with `docs` or `refactor`, an optional `(scope)`, an optional `!`, and `:`. This does not change.
- **CM-2. Which names are behavioural.** A staged name is a behavioural artefact when it ends in `SKILL.md`, or when it holds `agents/` and ends in `.md` after it. The test applies to the whole name as git stores it, byte for byte. A newline or any other byte in the name matches like any other character, and "ends in" means the end of the name, not the end of a line inside it. This keeps the meaning of the old pattern `(SKILL\.md|agents/.*\.md)$`.
- **CM-3. How names are read.** The hook reads `git diff --cached --name-only -z` and passes the NUL-separated names to `xargs -0`, which tests each with a shell `case` pattern (`*SKILL.md|*agents/*.md`). No name is quoted, and none is split at a newline.
- **CM-4. Failure.** When any staged name is behavioural, the hook prints the error it printed before, lists each behavioural name as written, one per line after four spaces of indent, and exits 1. A name that holds a newline prints across two lines.
- **CM-5. Tests.** `scripts/ci/commit-msg.test.mjs` runs `sh .husky/commit-msg` in a scratch repository, with `npx` replaced by a stub that exits 0, so the test isolates the type check. It covers:
  - `docs:` fails, and names the file, for `crosscheck/skills/x/SKILL.md`, `crosscheck/skills/é/SKILL.md`, `crosscheck/agents/a"b.md`, `crosscheck/skills/n<newline>l/SKILL.md` and `crosscheck/agents/x<newline>y.md`;
  - `refactor(crosscheck):` fails for `crosscheck/skills/é/SKILL.md`;
  - `fix:` passes for `crosscheck/skills/é/SKILL.md`;
  - `docs:` passes for `docs/é.md` and for `crosscheck/skills/SKILL.md<newline>x`, which ends in neither suffix.
- **Known gaps, not rules.**
  - The test stubs `npx`, so it does not run commitlint. The CI job that runs `scripts/ci/*.test.mjs` installs no root dependencies.
  - `git commit --no-verify` skips the hook, and a clone without `npm install` never installs it. No CI job checks commit types against behavioural artefacts. That check is not yet reached. The property that blocks it is a CI job that reads every commit of a pull request, and the open question is whether the squash merge, which takes the pull request title as its message, makes a per-commit check worth its cost.
