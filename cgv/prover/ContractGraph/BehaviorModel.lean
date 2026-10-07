-- BehaviorModel.lean
-- TRUSTED-NOT-PROVED. These definitions are axiomatized claims about
-- Django's runtime behavior (Django 4.2 / 5.x) and about plain-Python data
-- classes (dataclasses, attrs, NamedTuple, TypedDict, pydantic v2).

namespace ContractGraph.BehaviorModel

/-- A DecimalField(max_digits=m, decimal_places=d) accepts a value v iff:
    - v has at most d fractional digits
    - v has at most m total digits
    - v has at most (m - d) integer digits -/
def decimalFieldAccepts (maxDigits decimalPlaces : Nat)
    (totalDigits fractionalDigits integerDigits : Nat) : Prop :=
  fractionalDigits ≤ decimalPlaces ∧
  totalDigits ≤ maxDigits ∧
  integerDigits ≤ maxDigits - decimalPlaces

/-- null=False rejects None values. -/
def notNullAccepts (isNull : Bool) : Prop :=
  isNull = false

/-- null=True accepts any value. -/
def nullableAccepts (_isNull : Bool) : Prop :=
  True

/-- CharField(max_length=n) accepts strings of length ≤ n. -/
def charFieldAccepts (maxLength : Nat) (actualLength : Nat) : Prop :=
  actualLength ≤ maxLength

/-- PositiveIntegerField accepts values ≥ 0. -/
def positiveIntFieldAccepts (value : Int) : Prop :=
  value ≥ 0

/-- MinValueValidator(limit) accepts values ≥ limit. -/
def minValueAccepts (limit : Int) (value : Int) : Prop :=
  value ≥ limit

/-- MaxValueValidator(limit) accepts values ≤ limit. -/
def maxValueAccepts (limit : Int) (value : Int) : Prop :=
  value ≤ limit

/-- choices accepts only listed values. -/
def choicesAccepts (validChoices : List String) (value : String) : Prop :=
  value ∈ validChoices

/-! ## Plain-Python data classes

Fields of `@dataclass`, attrs, `NamedTuple` and `TypedDict` classes carry
their contract in the type annotation only. Python does not enforce it at
runtime; the claim is that a well-typed program (as a type checker such as
mypy or pyright accepts it) never stores a value outside the annotation.
pydantic v2 `BaseModel` fields are enforced by validation at construction. -/

/-- `x: T` (no `None` in the annotation) accepts only non-None values;
    `Optional[T]`, `T | None` and `Union[T, None]` accept None. -/
def annotationAcceptsNull (annotationAllowsNone : Bool) (isNull : Bool) : Prop :=
  isNull = true → annotationAllowsNone = true

/-- pydantic `Field(max_digits=m, decimal_places=d)` / `condecimal(...)`:
    validation rejects values with more fractional or total digits,
    the same predicate as Django's DecimalField. -/
def pydanticDecimalAccepts (maxDigits decimalPlaces : Nat)
    (totalDigits fractionalDigits integerDigits : Nat) : Prop :=
  decimalFieldAccepts maxDigits decimalPlaces totalDigits fractionalDigits integerDigits

/-- pydantic `Field(max_length=n)` / `constr(max_length=n)` on `str`. -/
def pydanticMaxLengthAccepts (maxLength : Nat) (actualLength : Nat) : Prop :=
  actualLength ≤ maxLength

/-- pydantic `Field(le=n)`; `Field(lt=n)` on `int` is extracted as `le=n-1`. -/
def pydanticLeAccepts (limit : Int) (value : Int) : Prop :=
  value ≤ limit

/-! ## Numeric bounds, scaled to integers

Range bounds (`le`/`ge`, `MinValueValidator`/`MaxValueValidator`,
`PositiveIntegerField`, `condecimal(ge=...)`, on int, float and Decimal
fields) are compared as integers value × 10^D, with one D per database: the
most decimal places of any exact decimal bound (`param_min_decimal`/
`param_max_decimal`), and at least 6 when a bound is only given in micros
(`param_*_micros`, value × 10^6) or as a REAL. The claim is that for bounds
with at most D decimal places, `v ≤ b ↔ v·10^D ≤ b·10^D` (and likewise `≥`),
so decimal and micros bounds compare exactly at any size; that the
extractor's micros for a bound with more than 6 places are rounded toward
the requirement's stricter side and the guarantee's weaker side; and that
legacy REAL bounds, multiplied by 10^D and rounded the same way (a product
within 10^-4 of an integer is taken as that integer, floating-point noise
such as 0.3 × 10^6), denote bounds with at most D decimal places. The
functions below state the comparison with the scaled integers (named
"micros" for the common case D = 6). -/

/-- `v ≤ limit`, both in micros. -/
def maxValueAcceptsMicros (limitMicros : Int) (valueMicros : Int) : Prop :=
  valueMicros ≤ limitMicros

/-- `v ≥ limit`, both in micros. -/
def minValueAcceptsMicros (limitMicros : Int) (valueMicros : Int) : Prop :=
  valueMicros ≥ limitMicros

/-! ## Numeric tower

PEP 484: where an argument is annotated `float`, an argument of type `int`
is acceptable. This is the PEP text for every annotation, so it covers
parameters and dataclass, attrs, `NamedTuple` and `TypedDict` fields; no
checker (mypy, pyright) or attrs validator was run for it. pydantic 2.11.10
accepts an `int` for a `float` field in strict mode too (`ConfigDict(strict=True)`,
`StrictFloat`, `Field(strict=True)`), storing `float(v)`. Lax pydantic numeric
fields and Django `FloatField` writes carry no type requirement.

Not stated here:
- `bool` into `float`. PEP 484 checkers accept it (`bool` subclasses `int`),
  but strict pydantic rejects it.
- `int` or `float` into `complex`. PEP 484 accepts both, but strict pydantic
  rejects both, and no `complex` requirement is extracted.
- The magnitude of the value. An `int` is exact and unbounded, but `float(v)`
  loses precision above 2^53 and raises `OverflowError` above about 1.8e308.
  PEP 484 checkers accept the pair regardless, and no extractor path is known
  that carries such a bound into a `float` requirement.

Premise: the names `int` and `float` denote the builtins. The extractor takes
an annotation's type name from its last dotted segment without resolving it
(`walk_annotation` and `last_segment` in `dataclass_extractor.rs`), so a project
that defines or imports its own `float` or `int` is read as the builtin. Name
resolution against the project index is not yet reached; the blocking property
is that the extractor reads annotations syntactically, and the open question is
whether it should emit a type name only when the name resolves to the builtin,
as `is_external` does for calls. -/

/-- An annotation naming `targetType` accepts a value of `valueType`: the same
    type, or an `int` where `float` is annotated. -/
def annotationAcceptsNumeric (valueType targetType : String) : Prop :=
  valueType = targetType ∨ (valueType = "int" ∧ targetType = "float")

end ContractGraph.BehaviorModel
