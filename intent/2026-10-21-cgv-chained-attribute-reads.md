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

`Scope::linked_field_facts` resolves `root.l1...ln.f`. The root must be a local of a known class, the same rule as `obj.f`. Each link must be in `links`, and the function must not assign it. The result is the facts that `field_read_facts` gives for the chain's flow name `root.l1...ln.f`: the values the function assigns to it, else the field's declared contracts as `linked_declared_facts` gives them, which turns a declared non-null into unknown nullability (see "Decided without the maintainer"). The CG-1.9 caller guards apply to the chain as they do to `obj.f`: `edge_discovery::call_guards` records a narrowed chain `a.link.f` as the parameter's field `link.f`, and `field_read_facts` keys the guard by the chain's root parameter. `flow::attr_name` now names chains of any depth, so `if order.customer.email is None: return`, `if payload.address.postcode:`, a dereference and an assignment narrow or rebind the chain as they do `obj.f`. Rebinding `obj.fk` forgets what was known about `obj.fk.*`, as rebinding `obj` already did for `obj.*`.

The spec is the two fixtures' `expected.json`:

- `cgv/test_fixtures/chained_relation/expected.json`. `bad.py` reads `Customer.email` (`null=True`) into the non-null `Invoice.email` through `order.customer`, and three levels deep through `shipment.order.customer` (a `OneToOneField`, then a `ForeignKey`). It also reads a 40-character `Customer.name` into a 10-character column. All three are errors. `ok.py` guards the read with `is None`, with truthiness, by guarding the nullable link, and by assigning the chain first, and gives no error. It also has one unguarded read through the nullable `referrer` link, which must be a warning, and a read of `Customer.email` that its only caller guards (`bill_caller_guarded`), which must be a warning and not an error.
- `cgv/test_fixtures/chained_dataclass/expected.json`. The same cases through a nested `@dataclass` and a nested pydantic model, with two errors and one required warning.
- `cgv/test_fixtures/chained_write/expected.json`. `clear_name` writes None to the non-null `Customer.name` through `order.customer.name`, which edge discovery does not record. `copy_name` copies `order.customer.name` into the non-null `Invoice.name`. The run must report that read as a warning. It exits 0, because a warning does not change the exit code, and an error needs the write to be checked (CG-1.52).

`extractor::tests::test_links` pins the resolution rules: inheritance, `to=`, `"self"`, a non-literal `null=`, `"app.Model"`, an unresolvable `"auth.User"`, a non-project target, a quoted `Optional["Address"]`, a `list[Address]` that is no link, and an inherited data class field. `flow::tests::test_attribute_chains_narrow_and_rebinding_a_link_forgets_them` pins chain narrowing and forgetting, and it fails if the old early return in `forget_attributes` comes back. `value_analysis::tests::test_linked_field_facts` pins each branch of `linked_field_facts`, the non-null rule and the caller-guard lowering of a chain.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on a project that reads model or data class fields through relations or nested data classes. Real nullability, length, precision, range, choices and type bugs on those reads become errors. A read that was hidden as unverified may now be an error or clean, except that a field declared non-null stays unverified when read through a link. On the bench, `two-attribute-deep` moves from WARNING_ONLY to CAUGHT, and `bench/baseline.json` and the README table record it. No other case changes. All 109 fixtures pass. The labelled run keeps precision 0.67 on 3 errors.
- `cgv/src/resolve.rs` (`Link`, `ClassInfo::links`, and `ProjectIndex::annotation_class`, moved from `value_analysis.rs` so the extractor can use it), `cgv/src/model_extractor.rs` (`relations`), `cgv/src/extractor.rs` (links for Django models and data classes), `cgv/src/flow.rs` (`attr_name`, `forget_attributes`), `cgv/src/value_analysis.rs` (`linked_field_facts`, `linked_declared_facts`, `field_read_facts`), `cgv/src/edge_discovery.rs` (`call_guards`), and `cgv/README.md`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The tier is 2: a behavioural change to the extractor, with no protected path.
- A one-level read `obj.f` relies on every write to the field being checked against its contracts. A chained read cannot rely on that for nullability: a write through a chain (`order.customer.name = None`) or through an alias (`c = order.customer; c.name = None`) is not an edge, so a None can be stored in a field declared non-null. The review of #132 ran both and got exit 0 with no warning. So a chained read of a non-null field has unknown nullability, and CG-1.52 records the write rule that would let it carry non-null.
- Effect on the guarantee: exit 0 promises the same for every input, except reads through a link. There, a read that used to be unverified now has the field's declared facts, other than a declared non-null, which stays unverified. The other declared facts (length, precision, range, choices, type) are trusted as for `obj.f`, although a write through a chain or an alias is not checked against them either. CG-1.52 covers those writes too.
- Not yet reached: a reverse relation (`customer.order_set`, the reverse side of a `OneToOneField`), a target named by `settings.AUTH_USER_MODEL`, a model name defined in another module that the declaring module does not import, an `"app_label.Model"` string that does not name exactly one project model in an `app_label.models` module, a chain whose root is not a local of a known class (`get_order().customer.email`), and a chain whose link the function assigns. Each still reads as unknown, so each is unverified as before and is never silent. The blocking property is that a link is resolved only from the field's own declaration. The open question is whether a project-wide model registry is worth building for the rest.
- Edge discovery still records a write only through `obj.f`, so `order.customer.email = None` is not a write to `Customer.email`. That is the existing write rule, and this task does not change it (CG-1.52).
- The review of #132 left LOW findings that this task does not fix, listed in CG-1.53.

## Decided without the maintainer
The maintainer granted full autonomy for this run on 2026-10-21 and was away. The task row left one question to the intent: whether a read through a relation that may be None is a finding. The review of #132 raised two more. The coordinating agent decided those two under the maintainer's explicit full-autonomy and merge grant, given to it directly in chat on 2026-10-21.

A link that may be None (`ForeignKey(null=True)`, an `Optional` nested class, or a `null=` that is not a literal) raises `AttributeError` when it is None. It does not yield None, so no None reaches the target through it. Reporting a nullability error there would be a false error, because the value the write receives is never None from that cause. CGV has no constraint for "dereferences None" (see the README's `none-dereference` case, which is not yet reached).

Decision 1 (review of #132): a chained read (two or more levels) keeps every declared fact of the last field except a non-null one. A field declared nullable stays nullable, so a real nullability bug is still an error. A field declared non-null has unknown nullability, as on `origin/main` before this task, so the target's non-null requirement gives a warning ("no nullability postcondition"), not an error and not silence. The reason is the unchecked write: `a.b.f = None` elsewhere stores None in a field declared non-null, and trusting the declaration made such a read exit 0 with no warning (`chained_write`). Once the function narrows the chain (`if order.customer.name is None: return`) or stores a value into it, the read has the facts of what the guard or the store shows. This decision also covers the earlier question: a non-null field read through a nullable link is a warning, now for the same reason as through any link. The rule that lowered a non-null read through an unnarrowed nullable link is gone, since no input can reach it any more, and `Link::nullable` is read by nothing until CG-1.52 lets a chained read carry non-null again.

Decision 2 (review of #132): the CG-1.9 caller-guard lowering applies to chained reads. A chained read of a nullable field that every caller narrows has unknown nullability, so it is a warning, as the one-level rule gives, and not an error (`bill_caller_guarded`).

The maintainer could choose instead to check writes through chains and aliases first (CG-1.52) and then let a chained read of a non-null field carry non-null. Or the maintainer could report an unguarded nullable link as an error of its own kind (a None dereference), which needs a new constraint kind in the checker and is Tier 3.

## Open questions
None beyond the not-yet-reached cases under Constraints.
