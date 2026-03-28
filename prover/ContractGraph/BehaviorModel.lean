-- BehaviorModel.lean
-- TRUSTED-NOT-PROVED. These definitions are axiomatized claims about
-- Django's runtime behavior. Version: Django 4.2 / 5.x.

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

end ContractGraph.BehaviorModel
