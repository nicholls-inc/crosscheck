# Plan: Witness tests run each counterexample against the target field

Intent: `intent/2026-10-21-witness-tests.md`. Spec: `intent/2026-10-21-witness-tests-spec.md`.

## Files that change

| File | Change |
| --- | --- |
| `cgv/src/db.rs` | `field_origins` table and `insert_field_origin` (WX-3) |
| `cgv/src/model_extractor.rs`, `cgv/src/dataclass_extractor.rs` | write one origin row per field node |
| `cgv/src/witness.rs` (new) | selection, grammar, file names, rendering and writing (WX-1 to WX-8, WX-11), with unit tests |
| `cgv/src/witness_helper.py` (new) | the helper module the generator writes (WX-7 to WX-10) |
| `cgv/src/lib.rs`, `cgv/src/main.rs` | the module and the `--witness-tests` flag |
| `cgv/tests/e2e_witness_tests.rs` (new) | the CLI on the two new fixtures: module count, names, content, removal |
| `cgv/test_fixtures/witness_tests/`, `cgv/test_fixtures/witness_tests_clean/` (new) | Django and pydantic errors of every WT-1 kind, an integral target with a fractional counterexample, a data class target; and the same models with no error |
| `cgv/scripts/witness_tests.py` (new), `cgv/scripts/witness-requirements-django42.txt`, `cgv/scripts/witness-requirements-django52.txt` (new) | WX-12 |
| `.github/workflows/cgv-ci.yml` | two steps that run `witness_tests.py run` and `selftest` |
| `.assurance/protected-surface-amend/witness-tests-ci-2026-10-21.md` (new) | governance note for the workflow |
| `cgv/README.md`, `cgv/CLAUDE.md`, `JOURNAL.md`, `docs/TASKS.md` | the flag, the script, the journal entry, the CG-1.37 row |

## Order of work

1. Commit the intent, spec and plan, and the governance note.
2. The data shape: the `field_origins` table and its writers, with a unit test that each library is recorded.
3. `witness.rs` and the helper, with unit tests of selection, grammar, names, removal and rendering.
4. The flag in `main.rs`, the fixtures and the e2e test.
5. `witness_tests.py` and its requirements; run it and its self-test locally with both environments.
6. The CI steps, docs, journal and task row.

## Risks

- A fixture module that does not import on its own (a relation to an app that is not installed, a package that is not pinned). The run fails on it without the marker, so the script names it and the gap stays visible rather than being skipped.
- The app registration of WX-9 uses `AppConfig` attributes that Django could rename. Both pinned versions are run, and a change shows up as a run without the marker.
- The workflow gains about a minute for two virtual environments. They are not cached, so a change to the requirements files takes effect at once.

## Proof

- `cargo test` (unit and e2e), `scripts/check-fixtures.sh`.
- `python3 scripts/witness_tests.py run` and `python3 scripts/witness_tests.py selftest` exit 0 with both environments, and the transcript of a run on `witness_tests` shows each module failing with `CGV-WITNESS REJECTED` and the codes.
- `contracts check --witness-tests` on `witness_tests_clean` writes no file.
- `git diff --stat origin/main` touches only the files above, and no file under `cgv/prover/`.
