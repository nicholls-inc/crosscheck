import Lean.Data.Json
import ContractGraph.BehaviorModel

/-!
Evaluates the predicates of `ContractGraph/BehaviorModel.lean` on JSON input,
for `cgv/scripts/behavior_diff.py` (TB-1.14).

Each input line is `{"p": <predicate name>, "a": [<arguments>]}`, and each
output line is `true`, `false` or `error: <reason>`. The `Decidable` instances
below unfold each definition and infer the instance from its body, so a change
to a predicate's text changes what this program answers.
-/

open Lean ContractGraph.BehaviorModel

instance (m d t f i : Nat) : Decidable (decimalFieldAccepts m d t f i) := by
  unfold decimalFieldAccepts; infer_instance
instance (b : Bool) : Decidable (notNullAccepts b) := by
  unfold notNullAccepts; infer_instance
instance (b : Bool) : Decidable (nullableAccepts b) := by
  unfold nullableAccepts; infer_instance
instance (n l : Nat) : Decidable (charFieldAccepts n l) := by
  unfold charFieldAccepts; infer_instance
instance (v : Int) : Decidable (positiveIntFieldAccepts v) := by
  unfold positiveIntFieldAccepts; infer_instance
instance (l v : Int) : Decidable (minValueAccepts l v) := by
  unfold minValueAccepts; infer_instance
instance (l v : Int) : Decidable (maxValueAccepts l v) := by
  unfold maxValueAccepts; infer_instance
instance (cs : List String) (v : String) : Decidable (choicesAccepts cs v) := by
  unfold choicesAccepts; infer_instance
instance (a n : Bool) : Decidable (annotationAcceptsNull a n) := by
  unfold annotationAcceptsNull; infer_instance
instance (m d t f i : Nat) : Decidable (pydanticDecimalAccepts m d t f i) := by
  unfold pydanticDecimalAccepts; infer_instance
instance (n l : Nat) : Decidable (pydanticMaxLengthAccepts n l) := by
  unfold pydanticMaxLengthAccepts; infer_instance
instance (l v : Int) : Decidable (pydanticLeAccepts l v) := by
  unfold pydanticLeAccepts; infer_instance
instance (l v : Int) : Decidable (maxValueAcceptsMicros l v) := by
  unfold maxValueAcceptsMicros; infer_instance
instance (l v : Int) : Decidable (minValueAcceptsMicros l v) := by
  unfold minValueAcceptsMicros; infer_instance
instance (v t : String) : Decidable (annotationAcceptsNumeric v t) := by
  unfold annotationAcceptsNumeric; infer_instance

def strs (j : Json) : Except String (List String) := do
  (← j.getArr?).toList.mapM (·.getStr?)

def evalCase (p : String) (a : Array Json) : Except String Bool := do
  let nat (k : Nat) : Except String Nat := do (← a[k]?.getDM (throw s!"no argument {k}")).getNat?
  let int (k : Nat) : Except String Int := do (← a[k]?.getDM (throw s!"no argument {k}")).getInt?
  let bool (k : Nat) : Except String Bool := do (← a[k]?.getDM (throw s!"no argument {k}")).getBool?
  let str (k : Nat) : Except String String := do (← a[k]?.getDM (throw s!"no argument {k}")).getStr?
  match p with
  | "decimalFieldAccepts" =>
    return decide (decimalFieldAccepts (← nat 0) (← nat 1) (← nat 2) (← nat 3) (← nat 4))
  | "notNullAccepts" => return decide (notNullAccepts (← bool 0))
  | "nullableAccepts" => return decide (nullableAccepts (← bool 0))
  | "charFieldAccepts" => return decide (charFieldAccepts (← nat 0) (← nat 1))
  | "positiveIntFieldAccepts" => return decide (positiveIntFieldAccepts (← int 0))
  | "minValueAccepts" => return decide (minValueAccepts (← int 0) (← int 1))
  | "maxValueAccepts" => return decide (maxValueAccepts (← int 0) (← int 1))
  | "choicesAccepts" =>
    let cs ← strs (← a[0]?.getDM (throw "no argument 0"))
    return decide (choicesAccepts cs (← str 1))
  | "annotationAcceptsNull" => return decide (annotationAcceptsNull (← bool 0) (← bool 1))
  | "pydanticDecimalAccepts" =>
    return decide (pydanticDecimalAccepts (← nat 0) (← nat 1) (← nat 2) (← nat 3) (← nat 4))
  | "pydanticMaxLengthAccepts" => return decide (pydanticMaxLengthAccepts (← nat 0) (← nat 1))
  | "pydanticLeAccepts" => return decide (pydanticLeAccepts (← int 0) (← int 1))
  | "maxValueAcceptsMicros" => return decide (maxValueAcceptsMicros (← int 0) (← int 1))
  | "minValueAcceptsMicros" => return decide (minValueAcceptsMicros (← int 0) (← int 1))
  | "annotationAcceptsNumeric" => return decide (annotationAcceptsNumeric (← str 0) (← str 1))
  | other => throw s!"unknown predicate {other}"

def evalLine (line : String) : String :=
  let r : Except String Bool := do
    let j ← Json.parse line
    evalCase (← j.getObjValAs? String "p") (← (← j.getObjVal? "a").getArr?)
  match r with
  | .ok b => toString b
  | .error e => s!"error: {e}"

partial def loop (h : IO.FS.Stream) (out : IO.FS.Stream) : IO Unit := do
  let line ← h.getLine
  if line.isEmpty then return
  let line := line.trimAsciiEnd.toString
  unless line.isEmpty do out.putStrLn (evalLine line)
  loop h out

def main : IO Unit := do
  loop (← IO.getStdin) (← IO.getStdout)
