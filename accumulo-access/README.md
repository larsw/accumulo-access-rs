# Accumulo Access for Rust

## Introduction

This crate provides a Rust API for parsing and evaluating Accumulo Access Expressions, based on the [AccessExpression specification](https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md).

## Quickstart

Add the following to your `Cargo.toml`:

```toml
[dependencies]
accumulo-access = "0.3"
```

## Example

```rust
use accumulo_access::check_authorization;

fn main() {
    let expr = "A&B&(C|D)";
    let auths = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let result = check_authorization(expr, &auths);
    assert_eq!(result, Ok(true));
}
```

## Functionality

* Using `caching::check_authorization_csv` (enabled by the default `caching` feature) will memoize/cache the result based on the input (expression, comma-separated authorizations) pair.
* Possibility to return parsed expression as an expression tree; either as a serde JSON Value-based tree, or a JSON string representation.

## How the parser is checked

The parser is checked against a parser proven correct in Lean 4, in [`lean-oracle/`](https://github.com/larsw/accumulo-access-rs/tree/main/lean-oracle).
That parser is proven to accept exactly the strings the specification's grammar derives, and to
return the expression the grammar assigns. The proofs also show that the grammar is unambiguous,
that holding more authorizations never loses access, and that without authorizations only the
empty expression grants access.

This crate's Rust parser is not itself proven. A differential test in
[`oracle-difftest/`](https://github.com/larsw/accumulo-access-rs/tree/main/oracle-difftest) compares it with the proven parser on acceptance, the parsed
expression, evaluation under every subset of the expression's tokens, and the `Display` output.
For 0.3.0 they agree on every string of up to 8 characters over a ten-character alphabet (111
million inputs), on Apache's 242 test vectors, and on 4 million random expressions. CI checks the
proofs and repeats a smaller run on every change.

Parentheses may nest at most 256 deep, so that untrusted input cannot overflow the stack. The
specification has no such limit, so deeper expressions are valid but deliberately rejected.

## Known usages

* [Accumulo Access extension for PostgreSQL](https://github.com/larsw/accumulo-access-pg)

## Maintainers

* Lars Wilhelmsen (https://github.com/larsw/)

## License

Licensed under both the the Apache License, Version 2.0 ([LICENSE_APACHE](accumulo-access/LICENSE_APACHE) or http://www.apache.org/licenses/LICENSE-2.0) and the MIT License [LICENSE_MIT](accumulo-access/LICENSE_MIT).

