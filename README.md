# Accumulo Access for Rust

## Introduction

This crate provides a Rust API for parsing and evaluating Accumulo Access Expressions, based on the [AccessExpression specification](https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md).

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
accumulo-access = "0.2"
```

## Example

```rust
use accumulo_access::check_authorization;

fn main() {
    let expr = "A&B&(C|D)";
    let auths = ["A".to_string(), "B".to_string(), "C".to_string()];
    let result = check_authorization(expr, &auths);
    assert_eq!(Ok(true), result);
}
```

Malformed expressions are rejected rather than reinterpreted, so `check_authorization`
returns `Err` for input the [specification][spec] calls improper (`A&`, `&A`, `A&&B`,
`(A`, `A)`, `()`, `A&B|C`). The empty expression is valid and authorizes everything.

[spec]: https://github.com/apache/accumulo-access/blob/main/SPECIFICATION.md

## Limitations

* It doesn't have functionality for normalizing expressions (ref. the Java-based accumulo-access project).

## Known usages

* [Accumulo Access extension for PostgreSQL](https://github.com/larsw/accumulo-access-pg)

## Releasing

Releases are published to crates.io by the [`Release` workflow](.github/workflows/release.yml)
via [crates.io trusted publishing](https://crates.io/docs/trusted-publishing), so no API token
is kept in this repository.

1. On `main`, bump `version` in `accumulo-access/Cargo.toml` and the `accumulo-access`
   dependency version in `wasm-accumulo-access/Cargo.toml` to match.
2. Tag the commit and push the tag:

   ```bash
   git tag v0.2.0 && git push origin v0.2.0
   ```

The workflow refuses to run if the tag and `Cargo.toml` disagree. It then tests the workspace
with and without default features, packages the crate, publishes it, and creates a GitHub
release with generated notes. It can also be started from the Actions tab, which tags the
current commit for you and offers a dry-run mode that publishes nothing.

Re-running it for an already published version is safe: it skips the publish and finishes the
remaining steps, which is how to recover if a release fails after the upload succeeded.

> The workflow's **file name** is part of the trusted publisher configuration on crates.io.
> Renaming or moving it stops publishing until that configuration is updated.

## Maintainers

* Lars Wilhelmsen (https://github.com/larsw/)

## License

Licensed under both the Apache License, Version 2.0 ([LICENSE_APACHE](accumulo-access/LICENSE_APACHE) or http://www.apache.org/licenses/LICENSE-2.0) and the MIT License [LICENSE_MIT](accumulo-access/LICENSE_MIT).

