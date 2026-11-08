import AccessOracle.Spec

/-!
# The parser

A recursive-descent parser with one character of lookahead, one function per nonterminal.
Each function returns the value of what it parsed and the rest of the input, or `none`.
`Correctness.lean` proves that `parse` accepts exactly the strings the grammar in `Spec.lean`
derives.

The mutually recursive functions take fuel, which every call decreases. `parse` passes one more
than the input's length, which `Correctness.lean` shows is always enough.
-/

namespace AccessOracle

/-- The longest prefix of unquoted-token characters, and the rest. -/
def unquoted : List Char → List Char × List Char
  | [] => ([], [])
  | c :: rest =>
    if isTokChar c then
      match unquoted rest with
      | (tok, r) => (c :: tok, r)
    else ([], c :: rest)

/-- Puts `c` in front of the value of a successful parse. -/
def prepend (c : Char) : Option (List Char × List Char) → Option (List Char × List Char)
  | some (v, r) => some (c :: v, r)
  | none => none

mutual
  /-- The inside of a quoted token, after the opening quote: the characters it denotes and the
  rest after the closing quote. A character other than `"` and `\` stands for itself. -/
  def quotedBody : List Char → Option (List Char × List Char)
    | [] => none
    | c :: rest =>
      if c = '"' then some ([], rest)
      else if c = '\\' then afterBackslash rest
      else prepend c (quotedBody rest)

  /-- The inside of a quoted token, right after a `\`. -/
  def afterBackslash : List Char → Option (List Char × List Char)
    | [] => none
    | d :: rest => if d = '"' ∨ d = '\\' then prepend d (quotedBody rest) else none
end

/-- `access-token`. -/
def token : List Char → Option (List Char × List Char)
  | [] => none
  | c :: rest =>
    if c = '"' then
      match quotedBody rest with
      | some (v, r) => if v = [] then none else some (v, r)
      | none => none
    else
      match unquoted (c :: rest) with
      | (tok, r) => if tok = [] then none else some (tok, r)

mutual
  /-- `expression`. -/
  def expression : Nat → List Char → Option (Expr × List Char)
    | 0, _ => none
    | n + 1, s =>
      match operand n s with
      | none => none
      | some (e, []) => some (e, [])
      | some (e, c :: r) =>
        if c = '&' then
          match andTail n r with
          | some (e', r') => some (.conj e e', r')
          | none => none
        else if c = '|' then
          match orTail n r with
          | some (e', r') => some (.disj e e', r')
          | none => none
        else some (e, c :: r)

  /-- `access-token / paren-expression`. -/
  def operand : Nat → List Char → Option (Expr × List Char)
    | 0, _ => none
    | _ + 1, [] => none
    | n + 1, c :: rest =>
      if c = '(' then
        match expression n rest with
        | some (e, d :: r) => if d = ')' then some (e, r) else none
        | _ => none
      else
        match token (c :: rest) with
        | some (v, r) => some (.tok v, r)
        | none => none

  /-- `and-expression`, after its `&`. -/
  def andTail : Nat → List Char → Option (Expr × List Char)
    | 0, _ => none
    | n + 1, s =>
      match operand n s with
      | none => none
      | some (e, []) => some (e, [])
      | some (e, c :: r) =>
        if c = '&' then
          match andTail n r with
          | some (e', r') => some (.conj e e', r')
          | none => none
        else some (e, c :: r)

  /-- `or-expression`, after its `|`. -/
  def orTail : Nat → List Char → Option (Expr × List Char)
    | 0, _ => none
    | n + 1, s =>
      match operand n s with
      | none => none
      | some (e, []) => some (e, [])
      | some (e, c :: r) =>
        if c = '|' then
          match orTail n r with
          | some (e', r') => some (.disj e e', r')
          | none => none
        else some (e, c :: r)
end

/-- `access-expression`: `some none` for the empty expression, `some (some e)` for `e`, and
`none` when the input is not an access expression. -/
def parse : List Char → Option (Option Expr)
  | [] => some none
  | c :: rest =>
    match expression (rest.length + 2) (c :: rest) with
    | some (e, []) => some (some e)
    | _ => none

end AccessOracle
