# Accumulo Access for Rust

## Introduction

This crate provides a Rust API for parsing and evaluating Accumulo Access Expressions, based on the [AccessExpression specification](https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md).

## Quickstart

Add the following to your `Cargo.toml`:

```toml
[dependencies]
accumulo-access = "0.1"
```

## Example

```rust
use accumulo_access::check_authorization;

fn main() {
    let expr = "A&B&(C|D)";
    let auths = vec!["A", "B", "C"];
    let result = check_authorization(expr, auths);
    assert!(result.is_ok());
}
```

## Functionality

* Mostly follows the specification.
* Using the equivalent method in `caching::check_authorization` will memoize/cache the result based on the input (expression+authorization tuple).
* Possibility to return parsed expression as an expression tree; either as a serde JSON Value-based tree, or a JSON string representation.

## Limitations

* It doesn't limit the unicode ranges in quoted access tokens (ref. the specification).
* It doesn't have functionality for normalizing expressions (ref. the Java-based accumulo-access project).

## Known usages

* [Accumulo Access extension for PostgreSQL](https://github.com/larsw/accumulo-access-pg)

## Maintainers

* Lars Wilhelmsen (https://github.com/larsw/)

## License

Licensed under both the the Apache License, Version 2.0 ([LICENSE_APACHE](accumulo-access/LICENSE_APACHE) or http://www.apache.org/licenses/LICENSE-2.0) and the MIT License [LICENSE_MIT](accumulo-access/LICENSE_MIT).

