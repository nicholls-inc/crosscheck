# Spec: Read a merged pull request's commits in the Incident Eval Check

Intent: `intent/2026-09-30-incident-eval-range.md`. Governing roadmap item: PB-1.

IE-5 and IE-6 are revised, and IE-9 added, by `intent/2026-10-06-incident-line-spec.md`: only a whole `Fixes-Incident:` line is an incident reference, and the PR body is matched one line at a time.

- **IE-1.** `scripts/ci/incident-eval-check.mjs` reads three inputs from the environment: `PR_NUMBER`, `BASE_REF` and `HEAD_SHA`. It fetches `refs/pull/<PR_NUMBER>/head` from `origin` and reads the message of every commit in `origin/<BASE_REF>..<HEAD_SHA>`. It reads them after a squash merge whose head branch was deleted, and reads up to 256 MiB of `git log` output, not Node's 1 MiB default. Output past that limit exits 2 (IE-2), with Node's `ENOBUFS` error in place of git's.
- **IE-2.** If `PR_NUMBER` is not a decimal number, `HEAD_SHA` is not 40 hex characters, `BASE_REF` is empty, either git command fails, or the range holds no commits, the check exits 2. The workflow runs for every base branch, so a merge commit or a rebase merge into any base, not only the default one, exits 2. It prints the git command and git's error. It does not print "no incident reference — skipped".
- **IE-3.** Git runs without a shell, with its arguments passed as an array. This includes `git remote get-url origin`.
- **IE-4.** `.github/workflows/incident-eval-check.yml` has one step after setup. It passes `PR_NUMBER`, `BASE_REF`, `HEAD_SHA`, `PR_BODY` and `PR_LABELS` as environment variables and runs `node scripts/ci/incident-eval-check.mjs`. It writes nothing to `$GITHUB_OUTPUT`, and its `run:` line interpolates no `${{ }}` expression.
- **IE-5.** The incident rules are unchanged. With no incident label and no `Fixes-Incident:` line in the body or the commits, the check exits 0. With either one, it exits 1 unless an eval and a candidate invariant reference the incident. Commit messages are matched one line at a time, as they were when they arrived through `COMMIT_MESSAGES`, so an incident line with no value never takes the next line as its id. `PR_BODY` is matched as one text, as it was on `main`, so there an incident line with no value can still take the next line's first token. The body is read before the commits, and the first match wins. The `COMMIT_MESSAGES` input is removed; the workflow was its only caller.
- **IE-6.** `scripts/ci/incident-eval-check.test.mjs` runs under `node --test scripts/ci/*.test.mjs`. Against a scratch remote, it covers:
  - a squash merge whose branch was deleted, with `Fixes-Incident:` only in the oldest of three commits: exit 1, and the output names the incident (IE-1);
  - a commit message over 1 MiB with `Fixes-Incident:` on its last line: exit 1, and the output names the incident (IE-1);
  - the same, with no incident reference: exit 0 (IE-5);
  - `Fixes-Incident:` in the PR body: exit 1, and the output names the incident (IE-5);
  - a body id and a different commit id: the body's id wins, with a trailing `.`, `,` or `;` dropped, while an interior dot and any other trailing character are kept (IE-5);
  - a commit id ending in `.`: the mark is dropped (IE-5);
  - a body whose `Fixes-Incident:` line has no value, followed by an id on the next line: that id is read (IE-5);
  - a lower-case incident line: it is read (IE-5);
  - the `incident` label, padded and in mixed case, with no id: exit 1 (IE-5);
  - the `incident` label with an id in a commit: exit 1, and the output names that id (IE-5);
  - an eval named for the incident with an invariant citing it, and an eval citing it with an invariant under `crosscheck/docs/invariants/`: exit 0 and the PASS line (IE-5);
  - an eval with no invariant, and an invariant with no eval, each beside a file outside `evals/` and the invariant directories that names the incident: exit 1, and the output names only the missing artefact (IE-5);
  - every exit-1 case asserts the whole output;
  - a commit whose `Fixes-Incident:` line has no value, followed by another line: exit 0 (IE-5);
  - a commit merged in from the base branch that carries `Fixes-Incident:`: it is not read, exit 0 (IE-7);
  - a `HEAD_SHA` that is absent from the remote, a `PR_NUMBER` with no head ref on the remote, each malformed input, and a head already on the base branch: exit 2, empty stdout, and stderr names the cause (IE-2);
  - the script source contains none of `execSync`, `exec(` or `shell: true` (IE-3). This is a text scan: a shell option passed through a variable would get past it, and review covers that case;
  - the workflow's only `run:` line is `node scripts/ci/incident-eval-check.mjs`, it never names `$GITHUB_OUTPUT`, and each of the five inputs appears as a `NAME: ${{` line (IE-4). This is a line scan with no YAML parser, since the scripts take no dependencies. That the file has one step after setup, and that those lines sit in its `env`, rests on review.
- **IE-7.** The range starts at `origin/<BASE_REF>`, not at the base commit the pull request started from. Commits of the base branch that were merged into the pull request's branch are therefore not read.
