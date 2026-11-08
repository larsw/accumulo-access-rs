import AccessOracle.Spec

/-!
# Answers for the differential test

How the oracle describes an accepted expression to the Rust harness in `oracle-difftest/`. None of
this is proven; it only formats what the proven parser and evaluator return. The Rust harness
computes the same canonical form from the crate's parse tree, so the two must stay in step.
-/

namespace AccessOracle

/-- Lexicographic order on code points, which is also the byte order of their UTF-8 encodings
and so the order of Rust's `String`. -/
def lexLt : List Char → List Char → Bool
  | [], [] => false
  | [], _ :: _ => true
  | _ :: _, [] => false
  | a :: xs, b :: ys =>
    if a.toNat < b.toNat then true
    else if a.toNat = b.toNat then lexLt xs ys
    else false

def sortLex (l : List (List Char)) : List (List Char) :=
  l.mergeSort fun a b => !(lexLt b a)

/-- Removes adjacent duplicates, so all duplicates of a sorted list. -/
def dedupSorted {α : Type} [BEq α] : List α → List α
  | a :: tl@(b :: _) => if a == b then dedupSorted tl else a :: dedupSorted tl
  | l => l

def intercalate (sep : List Char) : List (List Char) → List Char
  | [] => []
  | [x] => x
  | x :: xs => x ++ sep ++ intercalate sep xs

def natChars (n : Nat) : List Char := (toString n).toList

/-- A token as its code points in decimal, e.g. `[65,66]` for `AB`: unambiguous, and free of
spaces and newlines. -/
def tokenCanon (v : List Char) : List Char :=
  ['['] ++ intercalate [','] (v.map fun c => natChars c.toNat) ++ [']']

def conjOperands : Expr → List Expr
  | .conj l r => conjOperands l ++ conjOperands r
  | e => [e]

def disjOperands : Expr → List Expr
  | .disj l r => disjOperands l ++ disjOperands r
  | e => [e]

def group (op : Char) (operands : List (List Char)) : List Char :=
  match dedupSorted (sortLex operands) with
  | [c] => c
  | cs => op :: '(' :: (intercalate [','] cs ++ [')'])

/-- The expression up to the order and repetition of operands and the grouping of repeated `&`
or `|`: `&(` or `|(`, then the sorted, distinct operands separated by `,`, then `)`. -/
partial def canon : Expr → List Char
  | .tok v => tokenCanon v
  | e@(.conj _ _) => group '&' ((conjOperands e).map canon)
  | e@(.disj _ _) => group '|' ((disjOperands e).map canon)

def canonAccess : Option Expr → List Char
  | none => "EMPTY".toList
  | some e => canon e

def Expr.tokens : Expr → List (List Char)
  | .tok v => [v]
  | .conj l r => l.tokens ++ r.tokens
  | .disj l r => l.tokens ++ r.tokens

/-- The distinct tokens of the expression, sorted. -/
def accessTokens : Option Expr → List (List Char)
  | none => []
  | some e => dedupSorted (sortLex e.tokens)

def indexOf (v : List Char) : List (List Char) → Option Nat
  | [] => none
  | w :: ws => if v = w then some 0 else (indexOf v ws).map (· + 1)

/-- The truth table of the expression over its tokens: for each `mask` below `2 ^ toks.length`,
`1` when holding the tokens whose bit is set in `mask` grants access. `-` when there are more
than `cap` tokens. -/
def table (cap : Nat) (toks : List (List Char)) (x : Option Expr) : List Char :=
  if toks.length > cap then ['-']
  else
    (List.range (2 ^ toks.length)).map fun mask =>
      let auths : List Char → Bool := fun v =>
        match indexOf v toks with
        | some i => mask.testBit i
        | none => false
      if accessEval auths x then '1' else '0'

def tokensField (toks : List (List Char)) : List Char :=
  if toks.isEmpty then ['-']
  else intercalate [';'] (toks.map fun v => intercalate [','] (v.map fun c => natChars c.toNat))

/-- The deepest nesting of parentheses outside quoted tokens. Only meaningful for a string the
parser accepts. -/
def parenDepth (s : List Char) : Nat := go s 0 0 0
where
  /-- `state`: 0 outside quotes, 1 inside quotes, 2 right after a backslash inside quotes. -/
  go : List Char → Nat → Nat → Nat → Nat
    | [], _, _, deepest => deepest
    | c :: rest, state, depth, deepest =>
      if state = 2 then go rest 1 depth deepest
      else if state = 1 then
        if c = '\\' then go rest 2 depth deepest
        else if c = '"' then go rest 0 depth deepest
        else go rest 1 depth deepest
      else if c = '"' then go rest 1 depth deepest
      else if c = '(' then go rest 0 (depth + 1) (max deepest (depth + 1))
      else if c = ')' then go rest 0 (depth - 1) deepest
      else go rest 0 depth deepest

end AccessOracle
