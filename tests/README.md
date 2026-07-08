# Integration tests

Workspace-wide index of the integration test binaries that compare against
checked-in goldens, the fixture directories they read, and the one canonical
way to regenerate those goldens. See [`docs/standards/testing.md`](../docs/standards/testing.md)
for the integration-first policy, golden discipline, and test-naming rules.

The root crate's CLI test suite retired with the native provisioning
surface: the `specify` binary is now a single `omnia::runtime!`
invocation, and the operational surface is covered by the
composed-deployment tests in `crates/runtime/tests/` (driving the
workflow guest through the omnia runtime) plus each crate's own
`tests/`. The root `tests/` tree keeps only the dev-only Rust-quality
gate (`rust_quality.rs`) and the shared `fixtures/` referenced by
crate-level suites.

## Canonical golden regeneration

There is exactly one supported regeneration switch — `REGENERATE_GOLDENS=1` —
and one canonical invocation shape. Always use `cargo nextest run`, never bare
`cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

After regenerating, `git diff` the goldens and review every change: a diff that
flips a kebab-case error `code` is a public-contract change, not a refresh.

## Golden-bearing binaries → fixture dirs

| Crate | Test binary | Fixture / golden dir(s) |
| --- | --- | --- |
| `specify-workflow-lib` | `goldens` | `crates/workflow-lib/tests/fixtures/*.golden.json` |
| `specify-standards` | `lint_index` | `crates/standards/tests/fixtures/lint/` |
| `specify-standards` | `lint_hint` | JSON: `crates/standards/tests/fixtures/lint/`; pretty: `crates/standards/tests/goldens/` |

Binaries not listed here assert structurally (status fields, exit codes, JSON
shape via `assert_cmd`) and carry no regenerable goldens.

## Shared test helpers

Each crate keeps its cross-binary helpers under `tests/<helper>/mod.rs` (the
sole `mod.rs` exception blessed in
[`docs/standards/coding-standards.md`](../docs/standards/coding-standards.md#module-layout)):

- `specify-workflow-lib`: `crates/workflow-lib/tests/common/mod.rs` — `MockCmd`.
- `specify-standards`: `crates/standards/tests/common/mod.rs`; `crates/standards/tests/eval_support/mod.rs` — `make_rule` / `hint` / `NoToolRunner` rule-and-hint scaffolding (the in-memory evaluator testkit lives at `crates/standards/src/lint/eval/testkit.rs`).

The `GIT_ENV` / `run_git` / `copy_dir` trio is single-sourced at
`tests/fs_git.rs` and pulled into each crate's `tests/common` via a
`#[path]` module declaration (each crate's `tests/` is its own
compilation unit, so the file is included rather than imported). Reach for the
shared helper rather than reintroducing a per-binary `copy_dir_recursive`.
