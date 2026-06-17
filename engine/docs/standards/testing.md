# Testing

Integration-first test posture: `cargo nextest` over the binary, one file per integration target, golden JSON checked in. Read this before adding a new test or harness.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --all --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — but a missing `[[test]]` declaration that should exist will silently produce no output. Cross-check `cargo test` output if you suspect a target is being skipped.

## Integration-first policy

Integration tests under `tests/` use `assert_cmd::Command::cargo_bin("specify")`, drive the binary through clap, and assert against stdout JSON or filesystem state. Test-binary names are `tests/<area>.rs` (`bootstrap`, `catalog_infer`, `cli`, `cli_contract`, `e2e`, `init`, `journal`, `lint`, `plan`, `registry`, `rules`, `rust_quality`, `slice`, `source`, `target`, `tool`, `workspace`); areas with several themed suites collapse their submodules under a sibling `tests/<area>/` directory via `#[path]` (e.g. `tests/slice/`, `tests/source/`, `tests/plan/`; a hub may also pull submodules from more than one such directory, as `plan` does from both `tests/plan/` and `tests/workflow/`).

One binary per *area* is the intentional layout — see [DECISIONS.md "Integration tests"](../../DECISIONS.md#integration-tests-one-binary-per-area-themed-submodules-via-path). Full `tests/it.rs` consolidation (every integration test in a single binary) was measured and dropped: the cold-build win was 7.3 % cargo-reported (well below the 20 % bar we apply to "Idiomatic Rust Cleanup" chunks) and a mega-binary makes `cargo test --test <area>` useless for local iteration. The middle ground we keep: conceptually-related suites that share a helper collapse their themed files under a sibling `tests/<area>/` directory wired with `#[path]` (the hub declares the helper — `common` / `eval_support` — once and links the crate-under-test once instead of N times), while unrelated areas stay in their own binary. Never group across crates — each crate's `tests/` is its own compilation unit.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatcher contract"](./handler-shape.md#dispatcher-contract).

## The three-layer pyramid

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety.

| Layer | Location | Required when | Forbidden when |
| ----- | -------- | ------------- | -------------- |
| **Kernel unit** | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The behavior is a pure projection/parse/validation kernel with meaningful edge cases (malformed input, boundary values, error variants the CLI cannot trigger) | The behavior is only observable through I/O orchestration the unit layer would have to fake |
| **Crate integration** | `crates/<name>/tests/` | The behavior spans modules within one crate and is unreachable (or impractical to reach) through the binary — internal invariants, filesystem-shape corner cases, registry-pinned schema compilation | The same observable behavior is already asserted through the binary; if a CLI test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process |
| **Binary integration** | `tests/<area>.rs` | The behavior is part of the CLI wire contract: flag parsing, exit codes, stdout JSON shape, journal events, filesystem effects of a verb | The assertion re-tests kernel logic already covered unit-side — binary tests buy wiring confidence, not rule-by-rule behavior matrices |

Rules of thumb:

- **One layer owns a behavior.** When a unit test and a binary test assert the same envelope shape, keep the unit test for the edge matrix and exactly one binary test for the wiring.
- **Per-rule coverage is unit-side.** Doctor/validation rules get one binary test per rendering path at most, never one binary test per rule outcome.
- **Don't promote pure-library tests into the binary harness.** A test that never spawns the binary belongs in the crate that owns the code (this is a policy violation the harness comment cannot excuse).
- **Err toward deletion at review time.** The registry/workspace duplication documented in the 2026-06 review grew because this boundary was undocumented; when in doubt about which layer covers a behavior, check the other layers before adding a test.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: an integration binary `tests/<area>.rs` supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`.

- Drop tokens the binary name or enclosing module already supplies: in `engine/layout.rs`, write `different_skeletons_error`, not `layout_different_skeletons_is_an_error`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix: six `mark_complete_*` tests become `mod mark_complete { fn idempotent() … }`.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the test body or a `//` comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns, so the dev-only predicate in `tests/rust_quality/checks.rs` enforces a 40-char cap instead. It scans an upward attribute window, so `#[tokio::test]` / `async fn` and tests behind intervening attributes (`#[ignore]`, `#[case(..)]`) are covered. `tests/rust_quality/main.rs::no_gated_rust_quality_findings` fails CI on any `rust.test-fn-name-too-long` finding.

## Patterns to follow

- Spin up a real `specify init` in a `tempfile::TempDir`. Reach for the shared helpers in `tests/common/mod.rs` (`init_workspace`, `copy_dir`, `run_git`/`GIT_ENV`) and follow the fake-forge bare-repo patterns in `tests/workspace.rs` for multi-repo / fake-forge work; do not invent a parallel harness.
- Compare stdout JSON against checked-in goldens under `tests/fixtures/e2e/goldens/`. Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run --test e2e` and `git diff` before committing. The harness substitutes tempdir paths to `<TEMPDIR>` so goldens stay machine-independent.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set the four `GIT_*` env vars from `tests/common::GIT_ENV` so authorship is deterministic.

`tests/plan/end_to_end.rs` is the RM-05 (multi-repo evals) deterministic CLI proof — the end-to-end fan-in-twice / fan-out-once path (`source survey` → `plan propose --dry-run | --from` → per-slice `source extract` → `slice synthesize` → `slice build` → `slice merge`, plus `depends-on` ordering and byte-identical kernel re-projection). Read it first when extending multi-repo coverage; the exhaustive reconcile-code coverage over the same fan-out shape lives in `tests/workflow/`.

## Golden file discipline

`REGENERATE_GOLDENS=1` is the single supported regeneration switch. After regenerating, run `git diff` on the goldens and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors) and [DECISIONS.md §"Wire compatibility"](../../DECISIONS.md#wire-compatibility)).

## Test-side gotchas

- Never hand-edit `metadata.yaml` from a test or fixture. Drive transitions through `specify slice transition`, `specify plan transition`, or `stamp_slice_outcome` in `tests/common/mod.rs` when a test needs a stamped phase outcome. The tests in `tests/slice.rs` are the canonical patterns.
- WASI fixture components used by `tests/tool.rs` are rebuilt via `scripts/regen-wasm-fixtures.sh`. The outputs are checked in; only re-run when a fixture source changes.
