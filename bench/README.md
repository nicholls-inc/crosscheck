# Benchmark Harness

The benchmark harness measures whether the checker finds real bugs in a Python codebase and catches previously discovered issues across tool versions.

## Running benchmarks

```bash
scripts/bench.py run [--corpus DIR ...] [--cli PATH] [--checker PATH] \
                     [--out RESULT.json] [--compare PREV.json] [--format text|markdown] \
                     [--keep-work DIR]
```

- `--corpus` may be repeated. The default is `bench/corpus`.
- `--cli` and `--checker` default to `target/release/crosscheck-contracts` and `prover/.lake/build/bin/contract-graph-checker`. Build them first.
- `--format` is `text` (default) or `markdown` for pasting into a PR.
- `--out` writes the machine-readable result. It holds no timestamps or absolute paths, so a committed baseline is stable.
- `--compare` reads a previous result and reports regressions, improvements, rank changes, new and removed cases, and changes in precision, false positives and stale labels per labelled run.
- `--keep-work` keeps the extracted git trees and SQLite databases in DIR. Without it they go in a temporary directory that is removed afterwards.

Exit codes: 0 (ok), 1 (a case outcome got worse under `--compare`), 2 (harness or pipeline failure: bad corpus file, missing binary, git failure, or a run where the pipeline exited 2 or printed no JSON). Exit 2 wins over exit 1. A precision drop, a scope change, and new or removed cases are reported but never fail the run.

## Corpus layout and formats

```
<corpus>/corpus.toml            required
<corpus>/cases/<id>/case.toml   one directory per replay case
<corpus>/cases/<id>/pre/, fix/  the two trees of a directory-based case
<corpus>/labels.jsonl           optional labelled findings
```

### corpus.toml

```toml
name = "public"
description = "Synthetic replay cases of real bugs"
check_args = ["--exclude", "**/tests/**"]
repo = "$MY_REPO"            # optional default git repo for git-based cases and labelled runs

[[labelled_run]]             # zero or more
id = "labelled-app"
app = "labelled/app"         # a directory relative to the corpus directory, or:
# repo = "${MY_REPO}"        # a git revision instead of app
# rev = "main"
app_subdir = ""              # optional path inside the tree to check
check_args = []              # optional, appended after the corpus check_args
```

Only `name` is required; `description` is for people. `check_args` are extra arguments for every run in the corpus. `repo` is resolved relative to the corpus directory, and `$VAR` and `${VAR}` are expanded from the environment. An unset variable is an error. A labelled run gives either `app` or `rev` (with `repo`, or the corpus `repo`), not both. Each labelled run is checked once and its errors are joined with `labels.jsonl`.

### case.toml

Each case directory holds `case.toml`, defining a pre-fix and a fixed version of an application. The harness runs the pipeline on both and compares the outcomes. A case is either directory-based (`pre` and `fix`, relative to the case directory) or git-based (`pre_rev` and `fix_rev`, with `repo` here or in `corpus.toml`). Git revisions are extracted with `git archive`, so the repository is never modified.

```toml
id = "precision-two-hop"     # optional, defaults to the directory name; unique across all corpora in a run
description = "Six decimal places written into a three decimal place field."
kind = "precision"
in_scope = true
source = "synthetic"         # optional: "synthetic" (default) or "replay"
pre = "pre"
fix = "fix"
# git-based instead:
# repo = "/path/to/repo"     # relative to the corpus directory; $VAR and ${VAR} expanded
# pre_rev = "abc123^"
# fix_rev = "abc123"
app_subdir = ""              # optional path inside each tree to check
check_args = []              # optional, appended after the corpus check_args

[[bug]]
file = "app/models.py"       # as the checker reports site.file, relative to the checked root
line = 12                    # line in the pre tree; or lines = [10, 14] (inclusive)
target = "EnergyRecord.energy"   # optional substring of target.name
```

`kind`, `in_scope` and at least one `[[bug]]` are required. `kind` is the first token of the checker's target requirement (`non-null`, `length`, `precision`, `range`, `choices`, `type`). `in_scope` must be decided before running the tool. A bug takes exactly one of `line` and `lines`. A result matches a bug when its kind, file (equal, or one path ends with `/` plus the other), line and optional target all match; the case matches if any bug does. A result with no site never matches.

### labels.jsonl

Optional file of labels for the errors of a labelled run, one JSON object per line:

```json
{"run": "labelled-app", "source": "f", "target": "M.field", "hop": ["f", "M.field"], "kind": "length", "site_file": "app/x.py", "label": "true_bug", "cause": "", "note": "one line"}
```

`run`, `source`, `target`, `hop`, `kind` and `label` are required. `label` is one of `true_bug`, `benign`, `false_positive`. `cause` is a short kebab-case tag for false positives (for example `numeric-tower`, `narrowing`). The identity of a finding is `(source, target, hop, kind, site_file)`, which does not depend on line numbers. Two labels with the same run and identity are an error. Labels for a run that is not a `[[labelled_run]]` id (including `<case>:pre` and `<case>:fix`, which are not supported) are ignored with a note on stderr.

## Outcome definitions

| Outcome | Meaning |
|---------|---------|
| CAUGHT | In-scope: an error matches a bug in pre, and no matching error in fix. Bug was found and fixed. |
| DETECTED_NOT_CLEARED | In-scope: a matching error in both pre and fix. Bug was found but not fully fixed. |
| WARNING_ONLY | In-scope: no matching error in pre, but a matching warning. Potential issue detected at warning level. |
| MISSED | In-scope: no error or warning matched any bug in pre. Issue was not detected. |
| NOT_REPORTED | Out-of-scope: no matching error in pre. Out-of-scope cases serve as negative controls: the checker should not flag them. |
| SPURIOUS_ERROR | Out-of-scope: a matching error in pre. The checker reported a false positive in out-of-scope code. |
| FAILED | A pipeline run of the case failed: checker exit 2, output that is not JSON, or a git extraction error. A bad case file or missing binary stops the whole run with exit 2 instead. |

In-scope cases are ordered best to worst: CAUGHT > DETECTED_NOT_CLEARED > WARNING_ONLY > MISSED. Out-of-scope: NOT_REPORTED > SPURIOUS_ERROR. FAILED is always worst.

## Rank, precision and stale labels

The rank is the 1-based position of the first matching result among the pre-fix errors (or, for WARNING_ONLY, warnings), sorted by file, line, target and source, and is reported as "k of N". It shows how deep in the output a reader must look to find the real bug.

For labelled runs the harness reports counts per label, `precision` (`true_bug` divided by labelled errors, rounded to 4 decimal places, `null` when nothing is labelled), the number of unlabelled errors, and stale labels (labels whose identity matches no current error, which is how a fixed false positive shows up). Stale labels are information, not failure. A labelled run whose pipeline fails is recorded as `{"id", "corpus", "failed": true}` and the run exits 2.

Incomplete results (`status` of `incomplete`) count as neither errors nor warnings.

## Private corpora

Create a corpus outside this repository for a private codebase:

1. Build a `corpus.toml` with git-based cases (use `repo = "/abs/path"` or `repo = "$MY_REPO"` with the variable in your environment).
2. Use `pre_rev` and `fix_rev` to specify pre-fix and post-fix commits. The site lines in bugs are line numbers in the `pre_rev` tree.
3. Set `in_scope` before running and record the decision.
4. Run the harness: `scripts/bench.py run --corpus /path/to/private-corpus --out /path/to/result.json`.
5. Never commit private corpora or their results to this repository.

## Adding a replay case from a bug fix

When a real bug is fixed, replay it as a synthetic case:

1. Find the fix commit.
2. Review the change and decide whether to mark it `in_scope = true` (a real bug we should catch) or `false` (test, cleanup, or a false positive corrected).
3. In the parent commit (pre), find the exact line number of the contract violation.
4. Create a case directory with `case.toml`, a `pre/` copy of the affected files from the parent, and a `fix/` copy from the fix commit.
5. Run the harness to verify the case produces the expected outcome.
6. Record the `rank` and `outcome` in the case or commit message for tracking across tool versions.
