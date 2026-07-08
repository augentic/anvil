# Testing

Integration-first test posture: `cargo nextest` over the binary, **one integration binary per crate** (each crate's `tests/it.rs` pulls every area in as a `#[path]` submodule), golden JSON checked in. The unit layer is deliberately thin — integration owns every CLI-reachable behavior and `cargo llvm-cov` is the brake on deletion. A ratchet gate (`tests/rust_quality.rs`, seeded by `tests/rust_quality_budget.toml`) holds the per-crate `src` unit-test count down — the lever is designing tests against the public surface, not widening it. Read this before adding a new test or harness.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --all --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — but a missing `[[test]]` declaration that should exist will silently produce no output. Cross-check `cargo test` output if you suspect a target is being skipped.

## Integration-first policy

Integration tests under `tests/` use `assert_cmd::Command::cargo_bin("specify")`, drive the binary through clap, and assert against stdout JSON or filesystem state. Each crate exposes **one** integration binary, `tests/it.rs`, which pulls every area in as a `#[path]` submodule (`mod plan;`, `mod slice;`, …). The per-area files stay on disk — `tests/archive.rs`, `tests/plan.rs`, `tests/workspace.rs`, and so on — but they are modules of `it`, not standalone binaries; areas with several themed suites collapse their submodules under a sibling `tests/<area>/` directory via `#[path]` (e.g. `tests/slice/`, `tests/source/`, `tests/plan/`; a hub may also pull submodules from more than one such directory, as `plan` does from both `tests/plan/` and `tests/workflow/`). `tests/rust_quality/` stays a separate dev-gate binary. Both targets are declared explicitly because each crate sets `autotests = false` in its `Cargo.toml` to suppress per-file auto-discovery.

One integration binary per crate is the intentional layout — see [DECISIONS.md "Integration tests"](../../DECISIONS.md#integration-tests-one-binary-per-crate-path-submodules). The crate-under-test links once per crate instead of once per area, which is the build cost the per-area layout paid 31 times over; the per-crate `it` keeps `cargo test -p <crate>` and nextest's `-E 'test(<area>::)'` module filters useful for local iteration, so consolidation does not cost the per-area selectivity the earlier mega-binary attempt would have. Never group across crates — each crate's `tests/` is its own compilation unit, so `specify`, `workflow`, `standards`, and `schema` each own a distinct `it`.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatcher contract"](./handler-shape.md#dispatcher-contract).

## The three layers — minimize the unit layer

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **fewer unit tests**: integration owns every CLI-reachable behavior, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer | Location | Required when | Forbidden when |
| ----- | -------- | ------------- | -------------- |
| **Kernel unit** | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense parse/projection edge matrix whose case-per-cell integration port would inflate the 4-wide subprocess pool | The behavior is reachable through the binary and an integration test already covers it — or could, without a matrix explosion |
| **Crate integration** | `crates/<name>/tests/` | The behavior spans modules within one crate and is unreachable (or impractical to reach) through the binary — internal invariants, filesystem-shape corner cases, registry-pinned schema compilation | The same observable behavior is already asserted through the binary; if a CLI test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process |
| **Binary integration** | `tests/it.rs`, area module `tests/<area>.rs` | The behavior is part of the CLI wire contract: flag parsing, exit codes, stdout JSON shape, journal events, filesystem effects of a verb | The assertion re-tests kernel logic already covered unit-side — binary tests buy wiring confidence, not rule-by-rule behavior matrices |

Rules of thumb:

- **Default to deletion.** A unit test survives only if it covers a CLI-unreachable branch worth testing, or it is the cheap home for a dense edge matrix that would otherwise bloat integration. Everything reachable belongs to integration.
- **Collapse matrices, don't enumerate them.** A unit test that walks a closed set of `(input → code)` cases is one table-driven `#[test]` with a block per case, not one `#[test]` per case. The five workflow matrices reduced in the 2026-06 sweep (`config`, `adapter/core`, `build/wire`, plan `validate`, `propose`) each collapsed this way with **zero** `cargo llvm-cov` movement.
- **Re-home, don't 1:1 port.** When deleting a unit test removes the only coverage of a CLI-reachable behavior, add a *small number* of representative integration cases — never a case-per-cell port (the subprocess pool is the scarce budget).
- **Don't promote pure-library tests into the binary harness.** A test that never spawns the binary belongs in the crate that owns the code (this is a policy violation the harness comment cannot excuse).

### Reaching the behavior: design against the public surface

Before writing a unit test, decide whether integration can reach the behavior. Ask three questions, then check visibility:

1. **Reachable?** Does some CLI input (or a crate-`pub` fn) actually run this code?
2. **Observable?** Does its effect surface at a public boundary — stdout JSON, exit code, filesystem, a `pub` return value?
3. **Affordable?** Can you construct the input and observe the effect through that surface without a subprocess-pool explosion (a case-per-cell CLI port) or compiling a fixture per case?

- **Reachable + observable + affordable** → write the integration test against the **existing** public surface. No new API; this is the default and covers the large majority.
- **Reachable + observable but cheap only in-process** (proptests, dense matrices) → if the kernel is already `pub`, relocate the test to `crates/<crate>/tests/`; if it is private, **collapse and keep** a table-driven unit test in place.
- **Unreachable or unobservable** → it is dead code or an implementation detail: make the state un-representable (`unreachable!`, typestate) or delete the assertion. Don't test it.

**Widening production API to test a private kernel is a last resort, not the lever.** It trades durable public-surface stability for coverage you already have, so prefer collapse-and-keep over widening. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero.

This is enforced by a strict ratchet in [`tests/rust_quality.rs`](../../tests/rust_quality.rs): `unit_test_budget_holds` holds each crate's `src` `#[test]` / `#[tokio::test]` count to the committed budget in [`tests/rust_quality_budget.toml`](../../tests/rust_quality_budget.toml). Adding a `src` unit test fails CI unless you raise the budget (a reviewable edit that must be justified); removing one fails until you ratchet the number down. The gate counts tests; `cargo llvm-cov` still guards behavior — they are independent and both stay.

### Phase 2 (deferred): per-test waiver markers

The count budgets are the phase-1 ratchet — they catch *new* `src` unit tests but don't force a justification for each surviving one. Once the residue is small (the re-triaged `Keep` set from the unit-test reduction review), phase 2 swaps the numeric budgets for a per-test marker: every surviving `src` `#[cfg(test)]` must carry a `// rust-quality:allow(unit-test) reason: …` line saying why integration cannot get the same signal. The gate then flags any unmarked `src` `#[test]` as `rust.unit-test-missing-waiver`, and the `rust_quality_budget.toml` files retire. This trades a coarse per-crate count for a reviewable per-test rationale. It is **not built yet** — it waits on the reduction reaching the residue.

### Coverage is the brake on deletion

`cargo llvm-cov` line/region coverage on still-live code is the safety net — not edge-matrix preservation. Before and after a reduction, run the coverage gate over the crate you touched:

```bash
cargo llvm-cov nextest -p <crate> --summary-only
```

A `TOTAL` drop on lines that are still live means real coverage was lost: backfill it with an integration assertion (preferred) or revert that specific deletion. A reduction lands only when coverage holds (a pure collapse of redundant cases is coverage-neutral by construction). Use `cargo llvm-cov nextest` (not bare `cargo test`/`cargo llvm-cov`): nextest's process isolation is what makes the CWD/env-mutating suites pass, and it is the runner CI uses.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `<area>` module of `tests/it.rs` supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 40-char cap below counts the bare `fn` identifier, not the (now deeper) `it::<area>::…` module path, so consolidation does not change the budget — but a merge that renames an area module is a good moment to drop any tokens the new module path already supplies.

- Drop tokens the binary name or enclosing module already supplies: in `layout.rs`, write `different_skeletons_error`, not `layout_different_skeletons_is_an_error`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix: six `mark_complete_*` tests become `mod mark_complete { fn idempotent() … }`.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the test body or a `//` comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns, so the dev-only predicate in `tests/rust_quality.rs` enforces a 40-char cap instead. It scans an upward attribute window, so `#[tokio::test]` / `async fn` and tests behind intervening attributes (`#[ignore]`, `#[case(..)]`) are covered. `tests/rust_quality.rs::no_gated_rust_quality_findings` fails CI on any `rust.test-fn-name-too-long` finding.

## Patterns to follow

- Spin up a real `specify init` in a `tempfile::TempDir`. Reach for the shared helpers in `tests/common.rs` (`init_workspace`, `copy_dir`, `run_git`/`GIT_ENV`) and follow the fake-forge bare-repo patterns in `tests/workspace.rs` for multi-repo / fake-forge work; do not invent a parallel harness.
- Compare stdout JSON against checked-in goldens under `tests/fixtures/e2e/goldens/`. Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -E 'test(e2e::)'` and `git diff` before committing. The harness substitutes tempdir paths to `<TEMPDIR>` so goldens stay machine-independent.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set the four `GIT_*` env vars from `tests/common::GIT_ENV` so authorship is deterministic.

The end-to-end fan-in-twice / fan-out-once path now runs through the guest orchestrators (`orchestrate::{author,execute,refine,build,merge}`) — its coverage lives in `crates/workflow/tests/` (orchestrate, merge_slice) and the guest-routing tests in `tests/guest.rs`; the exhaustive reconcile-code coverage over the same fan-out shape lives in `tests/workflow/`.

## Golden file discipline

`REGENERATE_GOLDENS=1` is the single supported regeneration switch. After regenerating, run `git diff` on the goldens and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors) and [DECISIONS.md §"Wire compatibility"](../../DECISIONS.md#wire-compatibility)).

## Test-side gotchas

- Never hand-edit `metadata.yaml` from a test or fixture. Drive transitions through `specify slice transition`, `specify plan transition`, or `stamp_slice_outcome` in `tests/common.rs` when a test needs a stamped phase outcome. The tests in `tests/slice.rs` are the canonical patterns.
