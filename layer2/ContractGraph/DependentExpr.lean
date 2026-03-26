-- DependentExpr.lean

import ContractGraph.Types

namespace ContractGraph

/-- Simple tokenizer for dependent expressions. -/
private def tokenize (s : String) : List String :=
  let s := s.trimAscii.toString
  go s.toList "" []
where
  go : List Char → String → List String → List String
    | [], current, acc =>
      if current.length > 0 then acc ++ [current] else acc
    | c :: cs, current, acc =>
      if c == '(' || c == ')' || c == ',' then
        let acc := if current.length > 0 then acc ++ [current] else acc
        go cs "" (acc ++ [c.toString])
      else if c == ' ' then
        let acc := if current.length > 0 then acc ++ [current] else acc
        go cs "" acc
      else
        go cs (current.push c) acc

/-- Parse an integer from a string. -/
private def parseInt? (s : String) : Option Int :=
  if s.startsWith "-" then
    (s.drop 1).toNat?.map (fun n => -(n : Int))
  else
    s.toNat?.map (fun n => (n : Int))

/-- Recursive descent parser for DepExpr.
    Grammar:
      expr := literal | "input_" name | func "(" expr "," expr ")"
      func := "max" | "min" | "add" | "sub"
      literal := integer -/
private partial def parseTokens (tokens : List String) : Option (DepExpr × List String) :=
  match tokens with
  | [] => none
  | tok :: rest =>
    if tok == "max" || tok == "min" || tok == "add" || tok == "sub" then
      -- Function call: expect "(" expr "," expr ")"
      match rest with
      | "(" :: rest1 =>
        match parseTokens rest1 with
        | some (arg1, "," :: rest2) =>
          match parseTokens rest2 with
          | some (arg2, ")" :: rest3) =>
            let expr := match tok with
              | "max" => DepExpr.max arg1 arg2
              | "min" => DepExpr.min arg1 arg2
              | "add" => DepExpr.add arg1 arg2
              | "sub" => DepExpr.sub arg1 arg2
              | _ => DepExpr.lit 0  -- unreachable
            some (expr, rest3)
          | _ => none
        | _ => none
      | _ => none
    else if tok.startsWith "input_" then
      some (DepExpr.input tok, rest)
    else
      match parseInt? tok with
      | some n => some (DepExpr.lit n, rest)
      | none => none

/-- Parse a dependent expression string into a DepExpr.
    Returns none if the string is malformed. -/
def parseDepExpr (s : String) : Option DepExpr :=
  let tokens := tokenize s
  match parseTokens tokens with
  | some (expr, []) => some expr
  | _ => none

/-- Evaluate a dependent expression given a mapping of input names to bounds.
    Returns the resulting bound. -/
def evalDepExpr (expr : DepExpr) (inputs : List (String × Int)) : Option Int :=
  match expr with
  | .lit n       => some n
  | .input name  => inputs.lookup name
  | .max a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return if va ≥ vb then va else vb
  | .min a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return if va ≤ vb then va else vb
  | .add a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return va + vb
  | .sub a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return va - vb

end ContractGraph
