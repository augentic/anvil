# Integration tests

Workspace-wide index of the integration test binaries that compare against checked-in goldens, the fixture directories they read, and the one canonical way to regenerate those goldens. See [`docs/standards/testing.md`](../docs/standards/testing.md) for the integration-first policy, golden discipline, and test-naming rules.

The `specify` binary is a single `omnia::runtime!` invocation; the operational surface is covered by each crate's own `tests/` (the typed command/HTTP routing and argument conversions in `crates/transport/tests/` and the workflow orchestrations in `crates/change/tests/`). The root `tests/` tree holds only the lightweight `checks` package (`boundaries` + `links` + `authoring`); fixtures are crate-local under `crates/<name>/tests/fixtures/`.

## Repo checks (`-p checks`)

Architecture boundary and docs/plugin link integrity. Separate package so the Wasmtime-heavy root runtime stays out of the ordinary test graph. See [Consistency Checks](../docs/contributing/checks.md).

```bash
cargo test -p checks
```

## Canonical regeneration

There is exactly one supported regeneration switch, with one canonical invocation shape. Always use `cargo nextest run`, never bare `cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

`REGENERATE_GOLDENS=1` refreshes checked-in golden outputs, including the request goldens under `crates/change/tests/goldens/` that pin the assembled judgment prompts (regenerate them whenever a judgment prompt or answer schema changes).

After regenerating, `git diff` the outputs and review every change: a diff that flips a kebab-case error `code` is a public-contract change, not a refresh.

## Golden-bearing binaries → fixture dirs

| Crate | Test binary | Fixture / golden dir(s) |
| --- | --- | --- |
| `slice` | `merge_goldens` | `crates/slice/tests/fixtures/spec-*` |
| `project` / `slice` | `answers` | `crates/project/answers/`, `crates/slice/answers/` |
| `change` | `reconciliation` / `synthesis` | `crates/change/tests/goldens/` (request goldens) |

Binaries not listed here assert structurally and carry no regenerable goldens.

## Shared test helpers

Cross-crate test support is single-sourced in the `crates/testkit` crate — the fixture adapter core, the unified capability provider, scripted answers, request goldens, command mocking, filesystem/git helpers (`GIT_ENV` / `run_git` / `copy_dir`), env guards, and plan builders. Suites depend on it as an ordinary dev-dependency (`use testkit::…`); do not reintroduce `#[path]` splices or per-suite provider copies.

Crate-private helpers stay under that crate's `tests/<helper>/mod.rs` (the sole `mod.rs` exception blessed in [`docs/standards/coding-standards.md`](../docs/standards/coding-standards.md#module-layout)), e.g. `crates/diagnostics/tests/diagnostics_support/mod.rs`.

Generic model test mechanics (`Harness`, `Scripted`) come from Omnia's dev-only `omnia-testkit`, re-exported through `testkit::model`.
