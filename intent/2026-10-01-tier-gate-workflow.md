# Intent: The Tier Gate workflow reads the base ref from the environment and every changed file name

Task: PB-1.7. Governing roadmap item: PB-1.

## Problem statement
`.github/workflows/tier-gate.yml` computes the changed files in one step and passes them to the gate in the next. It has two faults.

1. The step writes the list to `$GITHUB_OUTPUT` between `changed_files<<EOF` and `EOF`. A changed file named `EOF` ends that block early, and the runner reads the lines after it as more output commands. A second file named `<name><<EOF` opens a new block that the closing `EOF` ends, so the step succeeds and the gate never sees the files after the first `EOF`. Scratch pull request #60 shows this on GitHub. It changes `EOF`, `docs/assurance/a<<EOF` and the protected file `docs/assurance/zz.md`, declares `Tier: 1`, and the Tier Gate passed with `CHANGED_FILES` empty. With only the file named `EOF`, the step fails with "Invalid format 'EOF'".
2. The step puts `${{ github.event.pull_request.base.ref }}` straight into the `run:` script. GitHub substitutes the text before bash runs, so the branch name becomes shell source. Only people with write access name a base branch, so this is hardening, not an open hole. It is still the pattern GitHub's hardening guide says to avoid.

## Proposed outcome
- One step fetches the base, computes the changed files with `git diff --name-only --no-renames`, and runs the gate. The list goes to the gate through a shell variable, so nothing is written to `$GITHUB_OUTPUT` and no file name can end or open a block.
- The base ref reaches the script as the environment variable `BASE_REF`. The gate already reads `BASE_REF` and prints it in its pass line, which today says `base: (unspecified)`.
- A test runs the gate step's `run:` script from `tier-gate.yml` itself, in a scratch repository, with files named `EOF` and `a<<EOF` next to a protected file, and with a renamed protected file. The tests no longer leave `--no-renames` (TG-1a) unchecked.

## Affected users and systems
- Everyone who opens a pull request here: the Tier Gate reads their changed files.
- `.github/workflows/tier-gate.yml` and a new `scripts/ci/tier-gate-workflow.test.mjs`.
- `intent/2026-09-29-deterministic-evidence-spec.md` (TG-1a), `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- `scripts/ci/tier-gate.mjs` does not change. Its inputs stay `PR_BODY`, `PR_LABELS`, `CHANGED_FILES`, `BASE_REF` and `CROSSCHECK_PROTECTED_RULES`.
- The gate's own tests still run first, in their own step.
- No dependency is added, so the test reads the YAML without a YAML parser.
- The other workflows already pass the base ref through `env:` and do not use `$GITHUB_OUTPUT`, so they do not change.

## Open questions
None. The task row states the outcome.

A related fault is out of scope and gets its own row, PB-1.10. `git diff --name-only` quotes a path with a byte outside printable ASCII, so `docs/assurance/é.md` reaches the gate as `"docs/assurance/\303\251.md"`, which matches no protected glob.
