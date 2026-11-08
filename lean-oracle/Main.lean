import AccessOracle

/-!
# The oracle

Reads one request per line on standard input and writes one answer per line on standard output.

A request is the input's code points in decimal, separated by spaces, so that any string fits on a
line. An empty line is the empty string.

The answer is `ERR` when the input is not an access expression, and otherwise

```
OK <depth> <tokens> <table> <canon>
```

* `depth`: the deepest nesting of parentheses.
* `tokens`: the distinct tokens, sorted, separated by `;`, each as its code points in decimal
  separated by `,`. `-` when there are none.
* `table`: the truth table over those tokens (see `AccessOracle.table`), or `-` when there are more
  tokens than the cap given as the first argument (default 6).
* `canon`: the expression in canonical form (see `AccessOracle.canon`), `EMPTY` for the empty
  expression.
-/

open AccessOracle

/-- Decodes a request: code points in decimal, separated by spaces, ending in a newline. -/
def decode (line : List Char) : List Char := go line 0 false
where
  go : List Char → Nat → Bool → List Char
    | [], acc, inNumber => if inNumber then [Char.ofNat acc] else []
    | c :: rest, acc, inNumber =>
      if c.isDigit then go rest (acc * 10 + (c.toNat - '0'.toNat)) true
      else if inNumber then Char.ofNat acc :: go rest 0 false
      else go rest 0 false

def answer (cap : Nat) (s : List Char) : List Char :=
  match parse s with
  | none => "ERR".toList
  | some x =>
    let toks := accessTokens x
    "OK ".toList ++ natChars (parenDepth s) ++ [' '] ++ tokensField toks ++ [' '] ++
      table cap toks x ++ [' '] ++ canonAccess x

def toStr (cs : List Char) : String := cs.foldl String.push ""

partial def serve (cap : Nat) (stdin stdout : IO.FS.Stream) : IO Unit := do
  let line ← stdin.getLine
  if line.isEmpty then return
  stdout.putStrLn (toStr (answer cap (decode line.toList)))
  serve cap stdin stdout

def main (args : List String) : IO Unit := do
  let cap := (args.head?.bind String.toNat?).getD 6
  serve cap (← IO.getStdin) (← IO.getStdout)
