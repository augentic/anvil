# Integration tests

Workspace-wide index of the integration test binaries that compare against checked-in goldens, the fixture directories they read, and the one canonical way to regenerate those goldens. See [`docs/standards/testing.md`](../docs/standards/testing.md) for the integration-first policy, golden discipline, and test-naming rules.

The `specify` binary is a single `omnia::runtime!` invocation; the operational surface is covered by each crate's own `tests/` (the typed command/HTTP routing and argument conversions in `crates/transport/tests/` and the workflow orchestrations in `crates/workflow/tests/`). The root `tests/` tree holds the framework checks (`framework/`) and the shared `fixtures/` referenced by crate-level suites.

## Canonical golden regeneration

There is exactly one supported regeneration switch — `REGENERATE_GOLDENS=1` — and one canonical invocation shape. Always use `cargo nextest run`, never bare `cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

After regenerating, `git diff` the goldens and review every change: a diff that flips a kebab-case error `code` is a public-contract change, not a refresh.

## Golden-bearing binaries → fixture dirs

| Crate | Test binary | Fixture / golden dir(s) |
| --- | --- | --- |
| `workflow` | `merge_goldens` | `tests/fixtures/merge/case-*` |
| `schema` | `answers` | `schemas/answers/*.schema.json` |

Binaries not listed here assert structurally and carry no regenerable goldens.

## Shared test helpers

Each crate keeps its cross-binary helpers under `tests/<helper>/mod.rs` (the sole `mod.rs` exception blessed in [`docs/standards/coding-standards.md`](../docs/standards/coding-standards.md#module-layout)), declared per test binary with `mod <helper>;`:

- `workflow`: `crates/workflow/tests/common/mod.rs` — `MockCmd`, scaffold, and stamped-outcome helpers.
- `schema`: `crates/schema/tests/diagnostics_support/mod.rs` — diagnostic fixtures.

Cross-package test support lives in the dev-only `specify-testkit` workspace crate (`crates/specify-testkit`): its scripted `Model` mock (`specify_testkit::MockModel`) is consumed by the native harness and the sibling adapter suites.

The `GIT_ENV` / `run_git` / `copy_dir` trio is single-sourced at `tests/fs_git.rs` and pulled into each crate's `tests/common` via a `#[path]` module declaration (each crate's `tests/` is its own compilation unit, so the file is included rather than imported). Reach for the shared helper rather than reintroducing a per-binary `copy_dir_recursive`.
