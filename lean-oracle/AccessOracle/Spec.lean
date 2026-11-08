/-!
# The access expression specification

A transcription of the grammar and the evaluation rule in Apache Accumulo Access's
`SPECIFICATION.md` (https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md):

```
access-expression = [expression]   ; empty string is a valid access expression
expression        = (access-token / paren-expression) [and-expression / or-expression]
paren-expression  = "(" expression ")"
and-expression    = "&" (access-token / paren-expression) [and-expression]
or-expression     = "|" (access-token / paren-expression) [or-expression]
access-token      = 1*( ALPHA / DIGIT / "_" / "-" / "." / ":" / slash )
access-token      =/ DQUOTE 1*(unicode-subset / escaped) DQUOTE
unicode-subset    = %x00-21 / %x23-5B / %x5D-7F / unicode-beyond-ascii
escaped           = "\" DQUOTE / "\\"
slash             = "/"
```

Each nonterminal becomes an inductive relation between the characters it derives and the value
they denote. Nothing here is executable: `Parser.lean` holds the parser, and `Correctness.lean`
proves that it accepts exactly the strings these relations derive.
-/

namespace AccessOracle

/-- A non-empty access expression. The operators are binary: `A&B&C` is `conj A (conj B C)`. -/
inductive Expr where
  | tok (value : List Char)
  | conj (left right : Expr)
  | disj (left right : Expr)
  deriving Repr, DecidableEq, Inhabited

/-- `ALPHA / DIGIT / "_" / "-" / "." / ":" / slash`. ALPHA and DIGIT are the ASCII letters and
digits (RFC 5234). -/
def isTokChar (c : Char) : Bool :=
  let n := c.toNat
  (65 ≤ n && n ≤ 90) || (97 ≤ n && n ≤ 122) || (48 ≤ n && n ≤ 57) ||
    n == 95 || n == 45 || n == 46 || n == 58 || n == 47

/-- `unicode-subset`: every character except `"` and `\`. The specification leaves
`unicode-beyond-ascii` undefined; it is read here as every Unicode scalar value above `%x7F`,
which is what a Lean `Char` is. -/
def isQuotedChar (c : Char) : Bool := c != '"' && c != '\\'

/-- One `unicode-subset / escaped` item: the characters written and the character they denote. -/
inductive QuotedItem : List Char → Char → Prop
  | plain {c : Char} : isQuotedChar c = true → QuotedItem [c] c
  | escapedQuote : QuotedItem ['\\', '"'] '"'
  | escapedBackslash : QuotedItem ['\\', '\\'] '\\'

/-- `*(unicode-subset / escaped)`: the characters written and the characters they denote. -/
inductive QuotedItems : List Char → List Char → Prop
  | nil : QuotedItems [] []
  | cons {raw rest : List Char} {c : Char} {value : List Char} :
      QuotedItem raw c → QuotedItems rest value → QuotedItems (raw ++ rest) (c :: value)

/-- `access-token`: the characters written and the token they denote. Each quoted item denotes
exactly one character, so the quoted form's `1*` is `*` with a non-empty value. -/
inductive Token : List Char → List Char → Prop
  | unquoted {cs : List Char} : cs ≠ [] → (∀ c ∈ cs, isTokChar c = true) → Token cs cs
  | quoted {raw value : List Char} :
      QuotedItems raw value → value ≠ [] → Token ('"' :: (raw ++ ['"'])) value

mutual
  /-- `expression`. -/
  inductive Expression : List Char → Expr → Prop
    | operand {s : List Char} {e : Expr} : Operand s e → Expression s e
    | conj {s t : List Char} {e r : Expr} :
        Operand s e → AndExpression t r → Expression (s ++ t) (.conj e r)
    | disj {s t : List Char} {e r : Expr} :
        Operand s e → OrExpression t r → Expression (s ++ t) (.disj e r)

  /-- `access-token / paren-expression`, where `paren-expression = "(" expression ")"`. -/
  inductive Operand : List Char → Expr → Prop
    | token {s v : List Char} : Token s v → Operand s (.tok v)
    | paren {s : List Char} {e : Expr} : Expression s e → Operand ('(' :: (s ++ [')'])) e

  /-- `and-expression`, denoting the conjunction of its operands. -/
  inductive AndExpression : List Char → Expr → Prop
    | last {s : List Char} {e : Expr} : Operand s e → AndExpression ('&' :: s) e
    | more {s t : List Char} {e r : Expr} :
        Operand s e → AndExpression t r → AndExpression ('&' :: (s ++ t)) (.conj e r)

  /-- `or-expression`, denoting the disjunction of its operands. -/
  inductive OrExpression : List Char → Expr → Prop
    | last {s : List Char} {e : Expr} : Operand s e → OrExpression ('|' :: s) e
    | more {s t : List Char} {e r : Expr} :
        Operand s e → OrExpression t r → OrExpression ('|' :: (s ++ t)) (.disj e r)
end

/-- `access-expression`. `none` is the empty expression. -/
inductive AccessExpression : List Char → Option Expr → Prop
  | empty : AccessExpression [] none
  | expression {s : List Char} {e : Expr} : Expression s e → AccessExpression s (some e)

/-- Evaluation (`SPECIFICATION.md`, "Evaluation"): a token is true when the entity holds it as an
authorization, and the operators are boolean `and` and `or`. -/
def Expr.eval (auths : List Char → Bool) : Expr → Bool
  | .tok v => auths v
  | .conj l r => l.eval auths && r.eval auths
  | .disj l r => l.eval auths || r.eval auths

/-- An empty access expression is always true. -/
def accessEval (auths : List Char → Bool) : Option Expr → Bool
  | none => true
  | some e => e.eval auths

end AccessOracle
