# escalation

`escalation` is an error classification and propagation library for layered Rust applications.

It is designed for applications where errors are translated into higher-level logical errors as they cross architectural boundaries, while preserving the original cause and a logical propagation trace.

The crate is primarily intended to be used together with [`hooq`](https://crates.io/crates/hooq).

## Motivation

In layered applications, lower-level technical errors often need to be translated into errors meaningful to the caller.

For example:

```text
sqlx::Error
    ↓
RepositoryError
    ↓
UsecaseError
    ↓
EndpointError
```

`escalation` separates this process into two concepts:

* `Classify` — describes how a lower-level error is interpreted as a higher-level logical error.
* `Report<E>` — carries the logical error together with the original cause and propagation trace.

This allows application code to keep using ordinary `?` propagation while error classification and trace collection are handled separately.

## Example

```rust
use escalation::{Classify, Report, classify};
use thiserror::Error;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("user not found")]
    UserNotFound,

    #[error("internal repository error")]
    Internal,
}

#[derive(Debug, Error, Classify)]
enum UsecaseError {
    #[error("repository error")]
    Repository,

    #[error("internal error")]
    Internal,
}

classify! {
    RepositoryError => UsecaseError::Repository;
}
```

With `hooq`, an error can be escalated while keeping ordinary `?` syntax:

```rust
#[hooq::hooq(escalate)]
fn execute() -> Result<(), Report<UsecaseError>> {
    repository_operation()?;
    Ok(())
}
```

To use the `escalate` profile with `hooq`, place the following `hooq.toml` in the root of your project:

```toml
[escalate]
method = """.map_err(|e| {
    use ::escalation::{Escalate, ErrorInfo, wrapping};

    let error_type_name = ::std::any::type_name_of_val(&e);
    let path = $path;
    let fn_name = $fn_name;
    let line = $line as u32;
    let col = $col as u32;
    let expr = ::hooq::source_excerpt_helpers::one_line_stringify!($source);
    let tag = $tag;

    let info = ErrorInfo {
        error_type_name,
        path,
        fn_name,
        line,
        col,
        tag,
        expr,
    };

    e.escalate(info, wrapping!($error))
})"""
bindings = { tag = "\"no_tag\"", error = "_" }
```

See the `examples` directory for more complete usage patterns, including layered error propagation and call-site classification overrides.

The resulting `Report` keeps:

* the current logical error,
* the original error cause,
* the logical propagation trace.

## Variant mapping

`classify!` also supports enum-to-enum mappings:

```rust
classify! {
    match RepositoryError::* => UsecaseError::* {
        UserNotFound => Repository,
        Internal => Internal,
    }
}
```

The generated implementation uses a Rust `match`, so normal exhaustiveness checking applies.

## Unclassified errors

A logical error type may define a fallback classification for errors that are intentionally left unclassified:

```rust
use escalation::{Classify, Unclassified};

#[derive(Debug, Error, Classify)]
#[classify(Unclassified => Self::Internal)]
enum AppError {
    #[error("internal error")]
    Internal,
}
```

This can then be used explicitly at a call site:

```rust
some_operation()
    .map_err(Unclassified::from)?;
```

`Unclassified` is intended as an explicit escape hatch rather than an implicit catch-all.

## Structured output

`Report<E>` implements `Display` for human-readable output.

When the `serde` feature is enabled, reports can also be serialized for structured logging or observability pipelines.

## Architecture

`escalation` is especially useful at architectural boundaries:

```text
Infrastructure error
    ↓ Classify
Port error
    ↓ Classify
Use case error
    ↓ Classify
Endpoint error
```

For third-party errors, a local infrastructure error type can be used first when required by Rust's orphan rules.

```text
sqlx::Error
    ↓ From
LocalInfraError
    ↓ Classify
RepositoryError
```

`From` is useful for structural wrapping inside a layer, while `Classify` is intended for semantic translation between layers.

## Status

`escalation` is currently in early development.

The API may change while it is being validated in real applications.
