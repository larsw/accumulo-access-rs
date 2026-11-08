# Lean oracle for access expressions

A Lean 4 model of the [access expression specification][spec], a parser proven to implement it,
and an executable oracle built from that parser. The Rust harness in `../oracle-difftest`
streams inputs to the oracle and checks that the `accumulo-access` crate gives the same answers.

## What is proven

`AccessOracle/Correctness.lean` proves, for every string `s`:

| Theorem | Statement |
|---|---|
| `parse_iff` | The parser returns `x` for `s` exactly when the grammar derives `s` with value `x`. |
| `parse_none_iff` | The parser rejects `s` exactly when the grammar does not derive it. |
| `AccessExpression.unique` | The grammar derives `s` with at most one value. |
| `accessEval_mono` | Holding more authorizations never loses access. |
| `nothing_only_empty` | Without authorizations, only the empty expression grants access. |

`lake build` checks these proofs.

## What you have to trust

- **The grammar in `AccessOracle/Spec.lean`.** It is a transcription of the ABNF, one inductive
  relation per nonterminal. Read it against the specification.
- **The Lean kernel and compiler.** The kernel checks the proofs, and the compiler builds the
  executable.
- **The formatting in `AccessOracle/Canon.lean` and `Main.lean`.** It is not proven. It turns
  what the proven parser and evaluator return into the answers the harness reads.

Two readings of the specification are built in:

- **Characters above ASCII.** The specification uses `unicode-beyond-ascii` but does not define
  it. It is read as every Unicode scalar value above `%x7F`. The 2024 version defined it that way.
- **Control characters.** The specification allows the control characters U+0000 to U+001F
  and U+007F inside quoted tokens since apache/accumulo-access#96 (January 2026). The oracle
  implements that version.

## Running the differential test

Install Lean with [elan](https://github.com/leanprover/elan). The version is pinned in
`lean-toolchain`. Then build the oracle:

```sh
cd lean-oracle
lake build
```

Run the harness from the repository root:

```sh
# Every string of up to 7 characters over A B & | ( ) " \ space é.
cargo run --release -p accumulo-access-oracle-difftest -- exhaustive 7

# Random expressions from the grammar, half of them mutated (count, seed).
cargo run --release -p accumulo-access-oracle-difftest -- random 1000000 1

# Apache's test vectors, also checked against their expected results.
cargo run --release -p accumulo-access-oracle-difftest -- upstream testdata.json
```

Apache's test vectors are in [testdata.json][testdata].

The harness exits with 1 when the crate and the oracle disagree, and prints examples of each kind
of disagreement.

[spec]: https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md
[testdata]: https://github.com/apache/accumulo-access/blob/main/modules/test-data/src/main/resources/org/apache/accumulo/access/testdata/testdata.json
