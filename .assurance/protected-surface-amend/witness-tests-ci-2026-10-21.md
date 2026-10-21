## Protected-Surface Amendment

**Target file(s):** `.github/workflows/cgv-ci.yml`
**Class:** A (CI enforcement)
**Matched rule:** `.github/workflows/**`
**Date:** 2026-10-21

### Change Description

1. `.github/workflows/cgv-ci.yml`: after the `Fixtures (extractor + checker against expected.json)` step, `actions/setup-python@v5` installs Python 3.12, and two steps run `python3 scripts/witness_tests.py run` and `python3 scripts/witness_tests.py selftest` from `cgv/`. They install Django 4.2.30 or 5.2.18 with pydantic 2.13.5 and pytest 8.4.2 from hash-pinned requirements files into two virtual environments, generate the witness modules of every fixture with `contracts check --witness-tests`, run them, and fail unless every generated test fails for the stated reason (`CGV-WITNESS REJECTED`) or is withheld for a named gap, and unless a counterexample replaced by an accepted value makes the run fail (WX-12).

No existing step changes. No step calls an LLM or reads a secret.

The edit is not in this branch's commits. GitHub refused the agent's push of a commit that touches `.github/workflows/`, because its token lacks the `workflow` scope. The exact diff is in the pull request body. The maintainer pushes it to this branch before merging, or the pull request merges without CI running the witness tests and the CG-1.37 row is not done.

### Rationale

Task CG-1.37. Each CGV error now carries a counterexample that Lean proves relative to the translated constraints, and `BehaviorModel.lean`, which says how Django and pydantic treat it, is trusted, not proved. The row asks CGV CI to run the generated modules against pinned library versions so that a change to the extractor, the checker or a library that stops the target field rejecting a fixture's counterexample fails CI. Intent: `intent/2026-10-21-witness-tests.md`. Spec: `intent/2026-10-21-witness-tests-spec.md`. Plan: `intent/2026-10-21-witness-tests-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item CG-1)
- **Title:** Make CGV findings worth reading on a real codebase
- **Scope coverage:** CG-1's scope names "a checkable witness for each error" and issue #8. Task CG-1.37 in `docs/TASKS.md` names this CI change and says editing the workflow is Class A.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.github/workflows/cgv-ci.yml` | after the fixture step | three steps added |

### Test / Coverage Impact

- CGV CI gains a check that the library rejects each fixture counterexample, on two Django versions, and a mutation self-test of that check.
- Pull requests that touch `cgv/` take about a minute longer for the two virtual environments.
- No invariant, eval or existing CI step changes.

### Review Checklist

- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that CGV CI now installs Django, pydantic and pytest from PyPI on each run, pinned by hash.
- [ ] REQUIRES HUMAN VERIFICATION: CG-1 covers a CI step that runs generated tests.
- [x] Authoriser is a named human.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer pushed the workflow diff of the pull request body to this branch. Until then `git diff --name-only origin/main...HEAD` lists no path under the protected globs, and the diff plan names one.
- [x] No check is weakened: the existing steps are unchanged.
