# Diagnostics lint unification — superseded

> **Superseded (2026-07).** The lint engine this record tracked is deleted: `specify lint framework`, the Road A / Road B dispatcher, the `WorkspaceModel` indexer, the in-process framework checkers, and the `codex/rules/core/` (`CORE-*`) pack are gone. Framework invariants now run as plain cargo tests at [`tests/framework/`](./tests/framework/); repo-local Rust-quality predicates stay at [`tests/rust_quality.rs`](./tests/rust_quality.rs). The authoritative decision is [DECISIONS.md §"Lint engine deleted: framework checks are cargo tests"](./DECISIONS.md#lint-engine-deleted-framework-checks-are-cargo-tests); the contributor model is [docs/contributing/checks.md](./docs/contributing/checks.md).

This file was the status record for the lint unification work tracked as **A19** (unify lint output path + framework/consumer dispatch) and **A16** (imperative→declarative lint burn-down). Both completed and were then superseded wholesale by the engine deletion. The durable residue:

- The neutral `Diagnostic` / `DiagnosticReport` substrate in `diagnostics` survives — `slice validate`, plan validation, and build reports speak it. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](./DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).
- `Exit::from(&Error)` remains the only exit mapping ([docs/standards/handler-shape.md](./docs/standards/handler-shape.md)).
- The rules parser/resolver behind `specify rules export` remains in `standards`; rule-shape validation lives beside the rules in `augentic/specify-adapters` as a cargo test.

Git history carries the full A19 / A16 record.
