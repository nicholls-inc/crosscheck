# Intent: A field read through a relation or a nested data class carries the field's contracts

Task: CG-1.28. Governing roadmap item: CG-1. Issue: #6.

## Problem statement
CGV resolves a field read only one level deep. `obj.f` gets the declared contracts of field `f` when `obj` is a local of a known class. A read two or more levels deep, `order.customer.email` or `payload.address.postcode`, gets no facts at all. `attribute_facts` and `receiver_class` in `cgv/src/value_analysis.rs` resolve a receiver only through a name, and `cgv/src/model_extractor.rs` does not record what a `ForeignKey` or `OneToOneField` points at. So a `null=True` Django field or an `Optional` data class field read through a relation or a nested data class has unspecified nullability. The checker then reports a missing-guarantee warning, which CG-1.4 hides by default, instead of an error. In the real-codebase evaluation (`cgv/docs/evaluation/real-codebase-evaluation-2026-09.md`) this caused 2 of the 3 replay bugs that were reported only as warnings. The bench case `two-attribute-deep` shows it: WARNING_ONLY on `origin/main`.

Reproduced on `origin/main` 2efb07c with the two new fixtures:

```
crosscheck-contracts contracts check test_fixtures/chained_relation/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker --format text
RESULT: 0 errors, 0 warnings, 20 unverified. Exit code 0.

crosscheck-contracts contracts check test_fixtures/chained_dataclass/ ...
RESULT: 0 errors, 0 warnings, 9 unverified. Exit code 0.
```

Each fixture's `bad.py` holds real bugs, and each one exits 0.

## Proposed outcome
The data shape is `resolve::Link`, kept per class in `ClassInfo::links`. It records a field whose value is an instance of a project class: the qualified name of that class, and whether the field may hold None.

- Django: `model_extractor::relations` reads each `ForeignKey` and `OneToOneField` in a model body. It reads the target (first argument or `to=`) and `null=` (default `False`, `None` when it is not a literal). `extractor::relation_class` resolves the target. It accepts a class expression, `"self"`, a model name the module binds, or `"app_label.Model"` when exactly one project model `Model` lives in an `app_label.models` module. Anything else gives no link. Links are inherited from project bases, and a child's own link wins.
- Data classes: a field whose annotation (or `Optional` of it, or a quoted name) names a project class. The annotation comes from the class that declares the field, found through the bases in the order `dataclass_extractor` uses for inherited fields. The link's nullability is the field's.

`Scope::linked_field_facts` resolves `root.l1...ln.f`. The root must be a local of a known class, the same rule as `obj.f`. Each link must be in `links`, and the function must not assign it. The result is the facts that `field_read_facts` gives for the chain's flow name `root.l1...ln.f`: the values the function assigns to it, else the field's declared contracts. `flow::attr_name` now names chains of any depth, so `if order.customer.email is None: return`, `if payload.address.postcode:`, a dereference and an assignment narrow or rebind the chain as they do `obj.f`. Rebinding `obj.fk` forgets what was known about `obj.fk.*`, as rebinding `obj` already did for `obj.*`.

The spec is the two fixtures' `expected.json`:

- `cgv/test_fixtures/chained_relation/expected.json`. `bad.py` reads `Customer.email` (`null=True`) into the non-null `Invoice.email` through `order.customer`, and three levels deep through `shipment.order.customer` (a `OneToOneField`, then a `ForeignKey`). It also reads a 40-character `Customer.name` into a 10-character column. All three are errors. `ok.py` guards the read with `is None`, with truthiness, by guarding the nullable link, and by assigning the chain first, and stays clean. It also has one unguarded read through the nullable `referrer` link, which must be a warning.
- `cgv/test_fixtures/chained_dataclass/expected.json`. The same cases through a nested `@dataclass` and a nested pydantic model, with two errors and one required warning.

`extractor::tests::test_links` pins the resolution rules: inheritance, `to=`, `"self"`, a non-literal `null=`, `"app.Model"`, an unresolvable `"auth.User"`, a non-project target, a quoted `Optional["Address"]`, a `list[Address]` that is no link, and an inherited data class field. `flow::tests::test_attribute_chains_narrow_and_rebinding_a_link_forgets_them` pins chain narrowing and forgetting, and it fails if the old early return in `forget_attributes` comes back.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on a project that reads model or data class fields through relations or nested data classes. Real nullability, length, precision, range, choices and type bugs on those reads become errors. A read that was hidden as unverified may now be an error or clean. On the bench, `two-attribute-deep` moves from WARNING_ONLY to CAUGHT, and `bench/baseline.json` and the README table record it. No other case changes. All 108 fixtures pass. The labelled run keeps precision 0.67 on 3 errors.
- `cgv/src/resolve.rs` (`Link`, `ClassInfo::links`, and `ProjectIndex::annotation_class`, moved from `value_analysis.rs` so the extractor can use it), `cgv/src/model_extractor.rs` (`relations`), `cgv/src/extractor.rs` (links for Django models and data classes), `cgv/src/flow.rs` (`attr_name`, `forget_attributes`), `cgv/src/value_analysis.rs` (`linked_field_facts`), and `cgv/README.md`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The tier is 2: a behavioural change to the extractor, with no protected path.
- The read relies on the premise that `obj.f` already relies on. Every write to a field is checked against the field's contracts, so a stored value satisfies them. An alias (`c = order.customer; c.email = None`) is checked at its own write, as before.
- Effect on the guarantee: exit 0 promises the same for every input, except reads through a link. There, a read that used to be unverified now has the field's declared facts.
- Not yet reached: a reverse relation (`customer.order_set`, the reverse side of a `OneToOneField`), a target named by `settings.AUTH_USER_MODEL`, a model name defined in another module that the declaring module does not import, an `"app_label.Model"` string that does not name exactly one project model in an `app_label.models` module, a chain whose root is not a local of a known class (`get_order().customer.email`), and a chain whose link the function assigns. Each still reads as unknown, so each is unverified as before and is never silent. The blocking property is that a link is resolved only from the field's own declaration. The open question is whether a project-wide model registry is worth building for the rest.
- Edge discovery still records a write only through `obj.f`, so `order.customer.email = None` is not a write to `Customer.email`. That is the existing write rule, and this task does not change it.

## Decided without the maintainer
The maintainer granted full autonomy for this run on 2026-10-21 and was away. The task row left one question to the intent: whether a read through a relation that may be None is a finding.

A link that may be None (`ForeignKey(null=True)`, an `Optional` nested class, or a `null=` that is not a literal), and that is not narrowed, raises `AttributeError` when it is None. It does not yield None, so no None reaches the target through it. Reporting a nullability error there would be a false error, because the value the write receives is never None from that cause. Treating the read as non-null would turn a possible crash into a silent exit 0. CGV has no constraint for "dereferences None" (see the README's `none-dereference` case, which is not yet reached).

Decision: the read keeps every fact of the field. When the field is non-null, its nullability becomes unknown, so the target's non-null requirement gives a warning ("no nullability postcondition"), not an error and not silence. A nullable field stays nullable, so a real nullability bug is still an error. Once the chain or the link is narrowed (`if order.referrer is not None:`, or `if order.referrer.name is None: return`, which only completes if the link held an object), the read gets the field's full facts. Both fixtures require the warning.

The maintainer could choose instead to report an unguarded nullable link as an error of its own kind (a None dereference), which needs a new constraint kind in the checker and is Tier 3. Or the maintainer could treat it as non-null, which matches what the write receives but drops the warning.

## Open questions
None beyond the not-yet-reached cases under Constraints.
