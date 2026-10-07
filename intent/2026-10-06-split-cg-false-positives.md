# Intent: Split CG-1.2 into one row per false-positive cause

Task: CG-1.2. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
Row CG-1.2 asks for five fixes in one pull request. Issue #5 lists them: the numeric tower, parameters typed `object`, `NoReturn` calls and exhaustive `match`, four narrowing patterns, and pydantic `model_validate`. They share no code, and they do not share a tier.

- The numeric tower changes the `type` check. That check is `st = tt` in `constraintImplies` (`cgv/prover/ContractGraph/Checker.lean`). `constraintImplies` is a protected definition whose value hash is in `cgv/prover/protected-statements.txt`. Widening it is a Tier 3 change with a governance note and a **Protected-surface change** section, and the rule it relies on belongs in `BehaviorModel.lean`, which is protected too.
- Parameters typed `object` get their nullability from `annotation_facts` in `cgv/src/dataclass_extractor.rs`, which returns `Some(false)` for every plain name except `Any`. That is a Rust extractor change.
- `NoReturn` calls and exhaustive `match` live in `terminates` and `always_exits` in `cgv/src/flow.rs`. `case _ as x: assert_never(x)` already makes the `match` exhaustive, but the case body is an expression statement, so `terminates` reports that it falls through. An exhaustive `match` over every member of an `Enum`, with no wildcard case, is not recognised at all.
- The four narrowing patterns are separate additions to the `Narrowed` walk in `cgv/src/flow.rs`.
- `model_validate` is modelled as a typed write in `cgv/src/edge_discovery.rs` and as a constructor in `cgv/src/value_analysis.rs`. `BehaviorModel.lean` says that pydantic enforces fields "by validation at construction" and says nothing of lax coercion, so this fix may also need a protected-surface amendment.

One pull request with all five would mix a Tier 3 proof-surface change with four Tier 2 extractor changes. A reviewer could not tell which fixture covers which rule, and a fault in one fix would hold back the other four.

## Proposed outcome
`docs/TASKS.md` replaces row CG-1.2 with five rows, CG-1.7 to CG-1.11, one for each cause in issue #5, in the order of how many false positives each cause produced on the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured: 35, 4, 4, 4 and 2. Each row names the code that holds the fault and asks for a fixture with an `ok.py` that fails if the false positive returns. No row depends on another. No other row names CG-1.2 in `Depends on`, so no dependency changes.

This pull request does nothing else, as the queue's rule for splitting a task requires. It sets no row to `done`. Issue #5 stays open until the last of the five rows is done.

## Affected users and systems
- The maintainer and the agents that pick up CG-1 work through `node scripts/ci/task-queue.mjs next`.
- `docs/TASKS.md` and this intent. Neither is protected, so the change is Tier 1.

## Constraints
- Task IDs are never reused. CG-1.7 to CG-1.11 appear in no row on `origin/main` and in no open pull request's diff of `docs/TASKS.md`.
- The task queue check must pass: each new ID names roadmap item CG-1, and every status is `todo`.
- The evaluation found a sixth cause that issue #5 does not list: ORM invariants, such as `.first()` after `MultipleObjectsReturned` and `filter(f__gte=...)` excluding NULL (2 false positives). No row is added for it here, because the split rule allows only rows that replace CG-1.2. It needs its own row or issue.

## Open questions
None for the split. Each new row carries its own question for its own intent. For CG-1.7, whether an extractor-side change can avoid widening `constraintImplies`. For CG-1.8 and CG-1.10, whether `BehaviorModel.lean` needs a new rule.
