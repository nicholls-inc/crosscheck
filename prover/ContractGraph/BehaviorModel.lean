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

end ContractGraph.BehaviorModel
