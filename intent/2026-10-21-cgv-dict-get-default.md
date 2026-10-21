# Intent: `d.get(k, default)` is nullable when the dict may store None

Task: CG-1.29. Governing roadmap item: CG-1. Issue: #6.

## Problem statement
`dict.get(k, default)` returns the value stored under `k` when `k` is a key, whatever the default. If the dict stores None under `k`, the result is None. CGV gives `d.get(k, default)` with a non-None default unknown nullability, so a write of it into a non-Optional field is a warning, not an error, and the run exits 0. One of the three replay bugs in `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` has this shape: `data.get("city", "")` into a non-Optional data class field, reported as warning #3,094 of 8,355.

Reproduced on `origin/main` 2efb07c. The bench case `dict-get-default` is `WARNING_ONLY`, and the new fixture `cgv/test_fixtures/cg_dict_get_default/` exits 0 with ten warnings and no error:

```
crosscheck-contracts contracts check test_fixtures/cg_dict_get_default/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

## Proposed outcome
The data shape is one predicate on the receiver of `.get`, `dict_may_store_none` in `cgv/src/value_analysis.rs`. It holds when the receiver is known to be a dict, by the rule CG-1.9 already uses (`known_dict`), and its values may be None:

- a local that every assignment binds to a dict display, a dict comprehension, or a `dict()`, `defaultdict()` or `OrderedDict()` call;
- a parameter, not rebound in the body, annotated `dict`, `Dict`, `Mapping`, `MutableMapping`, `defaultdict`, `DefaultDict` or `OrderedDict` (also inside `Optional`), with no value type, or with a value type that admits None: `Any`, `object`, `Optional[T]`, `T | None`, or a union with `object`.

Where the predicate holds and `k in d` is not known, `d.get(k, default)` with two positional arguments is nullable. A write of it into a non-null target is an error.

Everything else keeps the facts it has today:

- A parameter annotated `dict[K, V]` with a `V` that excludes None (`dict[str, str]`) stays unknown, so it gives a warning. The annotation is not trusted to make the value non-null.
- A receiver not known to be a dict stays unknown and gives a warning: an unannotated parameter, an attribute (`os.environ`, `request.GET`, `Model.objects`), or a call result (`json.loads(...)`, `response.json()`).
- A module-level dict of non-None literals keeps its rule (fixture `r7_const_dict_get`): one of its values or the default.
- After `k in d`, `d.get(k, default)` has the facts of `d[k]` (unknown nullability, a warning), the same as `d.get(k)` under CG-1.9.

The spec is `cgv/test_fixtures/cg_dict_get_default/expected.json`, with the unit test `value_analysis::tests::test_get_default_from_dict_that_may_store_none`. `bug.py` holds six reads that must be errors: a bare `dict` parameter, `Dict[str, Any]`, `dict[str, Optional[str]]`, `Mapping[str, str | None]`, `Optional[dict]` and a local dict display. `ok.py` holds reads that must not be errors: the fix (`data.get("name") or ""`), a module-level dict, and four reads that must stay warnings (`dict[str, str]`, an untyped receiver, `os.environ`, and a read after `"name" in data`).

The bench case `dict-get-default` becomes `CAUGHT`. Its description, `cgv/bench/baseline.json`, the bench table in `cgv/README.md`, and the value-analysis and false-positive paragraphs of `cgv/README.md` change to match.

## Measurement before the change
The task asked how many warnings become errors. The private codebase of the evaluation is not available, so the measurement is on the public corpus and the fixtures, with the rule built and run against `origin/main`:

- Bench corpus: 2 warnings become 2 errors, both in the pre-fix tree of `dict-get-default`, both on the bug. The fixed tree and every other case are unchanged. The labelled run is unchanged (3 errors, precision 0.67).
- The 103 existing fixtures: no finding changes (253 errors and 227 warnings before and after).

## Affected users and systems
- People who run `crosscheck-contracts contracts check`. A `.get(k, default)` on a dict parameter or local whose values may be None, written into a non-Optional field, return or parameter, is now an error. Code that never stores None under that key gets a false error. The README lists it with the other false positives.
- `cgv/src/value_analysis.rs` (one match arm, one predicate, one unit test) and `cgv/src/function_extractor.rs` (`param_annotation_facts` becomes `pub(crate)`). No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- Tier 2: a behavioural change to the extractor, with no protected path.
- The change turns warnings into errors and never an error into silence or exit 0.
- Every existing fixture keeps its verdict, and `scripts/bench.py run --compare bench/baseline.json` reports no lost true positive.
- Other open pull requests edit `value_analysis.rs` (#132, CG-1.28). The diff there stays one arm and one function, so a rebase stays simple.

## Decided without the maintainer
The maintainer granted full autonomy for this run on 2026-10-21 and was away. Each question the task row left to the intent was decided with the most conservative option that keeps the soundness claims honest. The maintainer can reverse any of them in review.

- **Which receivers count.** Only a receiver known to be a dict, by CG-1.9's `known_dict` rule. The wider rule, every `.get(k, default)` except on receivers known never to return None, was rejected. It would turn many warnings on untyped payloads, `request.GET`, `kwargs` and configuration objects into errors, and CG-1's aim is fewer false errors as well as fewer missed bugs. Those reads stay warnings, so nothing becomes silent. The maintainer could ask for the wider rule. The cost is a false error at each `.get(k, default)` on a receiver whose type CGV cannot see.
- **Which value annotations exclude None.** A value type for which `param_annotation_facts` says "not nullable" (`str`, `int`, a project class). Such a dict stays unknown, a warning, rather than becoming non-null. A type alias from another package may hide an `Optional`, and a warning is the honest answer.
- **Agreement with CG-1.9's `k in d`.** Under `k in d`, `d.get(k, default)` is `d[k]`, as `d.get(k)` is, so it stays a warning. CG-1.9 decided that a membership guard turns this read into a warning. A stored None under a present key is still possible, so the read is never silent.
- **Locals.** A local dict built in the function counts as a dict that may store None, whatever the display holds, since a later `d[k] = v` is not tracked.

## Open questions
None. The decisions above close the questions in the task row.

Not yet reached: a `.get(k, default)` on a receiver whose type CGV cannot see, such as an attribute, a call result or an unannotated parameter. It stays a warning. The blocking property is that the extractor infers no type for those receivers. The open question is whether the result of `json.loads` and `response.json()` should count as a dict that may store None.
