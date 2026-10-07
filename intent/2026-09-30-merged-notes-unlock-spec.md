# Spec: Stop merged governance notes from unlocking the protected-surface hook

Intent: `intent/2026-09-30-merged-notes-unlock.md`. Governing roadmap item: PB-1.

A *note file* is a `.md` file directly under `.assurance/protected-surface-amend/` or directly under a `.assurance/add-session-*/` directory, as before. A *block* is the text from a `## Protected-Surface Amendment` heading up to the next such heading or the end of the file, as before.

- **PG-1.** The *default-branch commit* is the commit that `origin/HEAD` names. If `origin/HEAD` does not resolve, it is the commit that `origin/main` names. The hook reads both refs as last fetched and never fetches.
- **PG-2.** A block is *new* when its text, with CRLF line endings read as LF and trimmed of surrounding whitespace, does not occur in any note file in the default-branch commit. A block copied from any merged note, at any path, is not new. If git fails to list or read the note files in the default-branch commit, no block is new: an edit to a protected file exits 2, and stderr says the notes on the default branch could not be read.
- **PG-3.** An edit to a protected file is allowed only if a new block names the file's repository-relative path, and the note file holding that block did not already name that path in the default-branch commit. A note file that is not in the default-branch commit named nothing there. A block that is not new never allows an edit. So editing an old note unlocks only the paths the edit adds, and re-authorising a path that an old note file named needs a new note file.
- **PG-4.** If neither ref in PG-1 resolves, an edit to a protected file exits 2. Stderr says that the hook cannot find `origin/HEAD` or `origin/main`, and names `git fetch origin` and `git remote set-head origin --auto` as the fix. An edit to a file that is not protected exits 0, as before.
- **PG-5.** Git runs through `execFileSync` with its arguments as an array. No command goes through a shell.
- **PG-6.** Unchanged: the missing-rules-file exit 0, the malformed-rules-file exit 2, the `CROSSCHECK_PROTECTED_RULES` override, the glob matcher (PG-9 in `intent/2026-10-06-hook-newline-spec.md` later lets `**` match a newline), the directories scanned, and the substring match of the path in a block. The gate message for a protected file with no new block keeps its three-sentence shape. Its reason clause now says the file has no governance-note block that is new on this branch.
- **PG-7.** `scripts/ci/protected-surface-guard.test.mjs` runs under `node --test scripts/ci/*.test.mjs`. Each case builds a scratch repository with a bare remote and runs the hook with a `PreToolUse` payload on stdin. It covers:
  - a note on the default branch that names a protected file, with the branch unchanged: exit 2, and stderr is the gate message (PG-3);
  - the same file named by an untracked note, by a staged note, and by a note committed on the branch but not pushed: exit 0 each (PG-2, PG-3);
  - a new block appended to a note file that is on the default branch: the file the new block names exits 0, and a file that only the old block names exits 2 (PG-2);
  - a merged block copied into a new note file under either directory: exit 2 (PG-2);
  - a default branch whose note files git cannot read (a submodule entry at a note path): a new note exits 2, and stderr says the notes on the default branch could not be read (PG-2);
  - a CRLF checkout of a merged note: exit 2 (PG-2);
  - a merged block edited in place to name one more file: the added file exits 0, and the file the block already named exits 2 (PG-3);
  - a new block in an old note file that names a path the file already named: exit 2, and the same block in a new note file: exit 0 (PG-3);
  - a branch whose note has been squash-merged into the default branch and fetched: exit 2 (PG-2);
  - a remote whose default branch is `trunk`, with the note on `trunk` and no `main`: exit 2 with the gate message, not the PG-4 message (PG-1);
  - a remote with `origin/HEAD` unset: the note on `main` exits 2 with the gate message, and a block new on the branch exits 0 (PG-1);
  - a note on the default branch larger than 1 MiB: exit 2 (PG-2);
  - a new note under `.assurance/add-session-<name>/`: exit 0 (PG-6);
  - a note under `.assurance/add-session-<name>/` that is on the default branch: exit 2 (PG-2, PG-6);
  - a repository with no remote: a protected file exits 2 and stderr names `origin/HEAD` and `origin/main`; an unprotected file exits 0 (PG-4);
  - an unprotected file with no note: exit 0 (PG-6);
  - the hook source contains none of `execSync`, `exec(` or `shell: true` (PG-5). This is a text scan, and review covers a shell option passed through a variable.
- **PG-8.** These documents state the PG-3 rule in place of "a note in the working tree": `CLAUDE.md`, `.claude/rules/protected-surfaces.md` (deterministic layer), `docs/assurance/DEVELOPMENT-FRAMEWORK.md` (stage 3), `docs/gates/protected-surface-hook.md`, and the hook contract in `crosscheck/skills/assurance-init/SKILL.md` (step 7.5b). The gate explainer also covers the PG-4 block. `docs/TASKS.md` loses its rule about PB-1.3.
