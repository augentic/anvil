# Integration tests

Workspace-wide index of the integration test binaries that compare against checked-in goldens, the fixture directories they read, and the one canonical way to regenerate those goldens. See [`docs/standards/testing.md`](../docs/standards/testing.md) for the integration-first policy, golden discipline, and test-naming rules.

The `specify` binary is a single `omnia::runtime!` invocation; the operational surface is covered by each crate's own `tests/` (the typed command/HTTP routing and argument conversions in `crates/transport/tests/` and the workflow orchestrations in `crates/change/tests/`). The root `tests/` tree holds the lightweight `checks` package (`boundaries` + `links`), shared cross-crate `fixtures/` (today: `spec-*/`), and `fs_git.rs`.

## Repo checks (`-p checks`)

Architecture boundary and docs/plugin link integrity. Separate package so the Wasmtime-heavy root runtime stays out of the ordinary test graph. See [Consistency Checks](../docs/contributing/checks.md).

```bash
cargo test -p checks
```

## Canonical golden regeneration

There is exactly one supported regeneration switch — `REGENERATE_GOLDENS=1` — and one canonical invocation shape. Always use `cargo nextest run`, never bare `cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

After regenerating, `git diff` the goldens and review every change: a diff that flips a kebab-case error `code` is a public-contract change, not a refresh.

## Golden-bearing binaries → fixture dirs

| Crate | Test binary | Fixture / golden dir(s) |
| --- | --- | --- |
| `slice` | `merge_goldens` | `tests/fixtures/spec-*` |
| `schema` | `answers` | `schemas/answers/*.schema.json` |

Binaries not listed here assert structurally and carry no regenerable goldens.

## Shared test helpers

Each crate keeps its cross-binary helpers under `tests/<helper>/mod.rs` (the sole `mod.rs` exception blessed in [`docs/standards/coding-standards.md`](../docs/standards/coding-standards.md#module-layout)), declared per test binary with `mod <helper>;`:

- `workflow`: `crates/change/tests/common/mod.rs` — `MockCmd`, scaffold, and stamped-outcome helpers.
- `diagnostics`: `crates/diagnostics/tests/diagnostics_support/mod.rs` — diagnostic fixtures.

Cross-package model test support comes from Omnia's dev-only `omnia-testkit`: its recorded scripted harness is consumed by the workflow suites (`crates/change/tests/`) and the live-model test (`harness/live-model`).

The `GIT_ENV` / `run_git` / `copy_dir` trio is single-sourced at `tests/fs_git.rs` and pulled into each crate's `tests/common` via a `#[path]` module declaration (each crate's `tests/` is its own compilation unit, so the file is included rather than imported). Reach for the shared helper rather than reintroducing a per-binary `copy_dir_recursive`.
