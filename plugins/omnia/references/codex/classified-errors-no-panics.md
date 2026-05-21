---
id: RUST-001
title: Classified SDK Errors, No Panic Paths
severity: critical
trigger: Production Rust handler code can panic, discard a fallible result, or return an error that bypasses Omnia SDK classification.
---

## Rule

Production Omnia handler paths must handle fallible operations by returning `omnia_sdk::Result` with an appropriate `omnia_sdk::Error` classification, stable code, description, and useful context. Do not use panic-based control flow outside tests.

Use `bad_request!` for invalid caller input, `bad_gateway!` for upstream provider or dependency failures, `server_error!` for internal invariant failures, and domain error conversions when stable error codes matter. Add context at parse, serialization, provider, and conversion boundaries before mapping errors into the SDK model.

## Look For

- `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, unchecked indexing, or `unreachable!` in non-test handler paths.
- `let _ = fallible_call(...)`, ignored `Result`s, swallowed provider errors, or fallback values that hide failed work.
- Public handler or domain functions returning `anyhow::Error`, string errors, or custom errors that do not convert to `omnia_sdk::Error`.
- Method-style constructors such as `Error::bad_request(...)` or `Error::not_found(...)`, which are not the Omnia SDK error API.
- Provider failures mapped to generic internal errors when they should be dependency failures.
- Error codes built from variable data instead of stable snake_case identifiers.
- Generic descriptions such as "failed" or "invalid input" that omit the operation, field, or dependency involved.

## Spec Guidance

If artifacts define error codes or classifications, preserve them rather than inventing a finer-grained error model. If the spec is silent, propose the smallest useful error contract: invalid input, not found, upstream dependency failure, and internal invariant failure.
