# Accumulo Access for Rust

## Introduction

This crate provides a Rust API for parsing and evaluating Accumulo Access Expressions, based on the [AccessExpression specification](https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md).

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
accumulo-access-rs = "0.1.0"
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

## Limitations

* It doesn't limit the unicode ranges in quoted access tokens (ref. the specification).
* It doesn't have functionality for normalizing expressions (ref. the Java-based accumulo-access project).
* It doesn't have functionality for serializing expression trees to a string representation. 

## Known usages

* [Accumulo Access extension for PostgreSQL](https://github.com/larsw/accumulo-access-pg)

## Maintainers

* Lars Wilhelmsen (https://github.com/larsw/)

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](accumulo-access/LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
