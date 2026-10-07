# Spec: The text report hides missing-guarantee warnings and reports them as coverage for each module

Intent: `intent/2026-10-07-cgv-unverified-coverage.md`. Governing roadmap item: CG-1. Task: CG-1.4.

Every rule applies to `contracts check --format text`. The JSON format is unchanged.

- **UC-1. Classes.** Each checker result is one of four classes. A result with status `incomplete` is *incomplete*. Any other result with severity `error` is an *error*. A result with any other severity is *unverified* when its `suggestion` does not start with `Dependent expression on ` and its `source_guarantee` is `<kind> (unspecified)`, where `<kind>` is a non-empty string with no space. The kind is `<kind>`. Every other result is a *warning*.
- **UC-2. Default body.** By default the body prints every incomplete result, then every error, then every warning, in the order of the checker's results. It prints no unverified result.
- **UC-3. `--warnings`.** With `--warnings`, the body also prints every unverified result, after the warnings. `--warnings` and `--no-warnings` cannot be given together.
- **UC-4. `--no-warnings`.** With `--no-warnings`, the body prints no warning and no unverified result.
- **UC-5. Unverified block.** An unverified block has the shape of a warning block, with three differences. Its label is `UNVERIFIED`. Its second line reads `no <kind> guarantee for the value <hop source> passes to <hop target>`, where the hop source and target are the result's `hop` entries. It has no `guarantees` line.
- **UC-6. Coverage section.** Unless the run is incomplete, the report prints a `COVERAGE BY MODULE` section after the body. It has one line per module that has an edge or an unverified result, sorted by module name: `  <module>: <E> edge(s) checked, <U> requirement(s) unverified`, followed by ` (<kind> <n>, ...)` with the kinds sorted by name when `U > 0`. When some module has `U > 0`, a note follows that says these requirements are not yet reached, names the blocking property (the source has no guarantee of the required kind, so the requirement passes vacuously) and the open question (which guarantee the extractor could infer for such a value, or which annotation it should ask for), and, unless `--warnings` is given, says that `--warnings` lists them.
- **UC-7. Modules.** The module of a path is its first component when the path has more than one, else the file name with a trailing `.py` removed. A path with no normal component (an empty path) belongs to the module `(unknown)`. An unverified result belongs to the module of its `site.file`, or of its `source.file` when the site file is empty. An edge belongs to the module of its `site_file`, or of its source node's `source_file` when the site file is NULL or empty.
- **UC-8. Edges.** `E` counts the rows of the `edges` table of the contract database that the checker read. The sum of `E` over all modules equals the checker's `edges_checked`.
- **UC-9. Result line.** The last line reads `RESULT: <a> error(s), <b> warning(s), <c> unverified. Exit code <x>.`, where `a`, `b` and `c` count the errors, warnings and unverified results, whether or not they were printed.
- **UC-10. Exit code.** The exit code is the checker's `exit_code`, as before.

**Known gaps, not rules.**
- UC-1 reads the class from the text the checker writes. If `missingPostconditionWarning` changes its wording, a missing-guarantee warning becomes a warning, which UC-2 prints, so the change makes the report louder, not quieter. An end-to-end test on a fixture pins the wording. A structured field in the checker's JSON would remove the string match, at the cost of a change to the Lean output, and is left for a later task.
- `E` counts edges, as the checker's `edges_checked` does, including `calls` edges, which no hop checks. A count of checked hop states for each module needs the checker to report the site of each state, and is left for a later task.
