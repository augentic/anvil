# Testing

Integration-first test posture: `cargo nextest` over public crate and binary boundaries, one self-contained fixture adapter supplying both `specify:adapter` axes, and structural goldens where bytes are the contract. The unit layer is deliberately thin — integration owns every CLI-reachable behavior and `cargo llvm-cov` is the brake on deletion. Read this before adding a test.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --locked --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly. The selection is the default workspace members — `crates/*`, `harness/fixtures`, and the `tests/framework` package; the WASM boundary smoke and the live test are opt-in rungs (below) so ordinary test runs never compile Wasmtime or call a model.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The three rungs

Specify is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `specify-adapters`; external adapters prove their own behavior against the published WIT package.

| Rung | Command | Proves | Cadence |
| ---- | ------- | ------ | ------- |
| **Native integration** | `cargo make test` | The complete workflow — reconciliation, synthesis, build, merge, lifecycle — through the `workflow` crate's public operations, the fixture adapter's native seams, and `omnia-testkit` scripted models | Every push (part of `cargo make ci`) |
| **WASM boundary** | `cargo make test-wasm` | The WASM seam only: the combined `fixture_adapter.wasm` loads, both axes dispatch through generated bindings, metadata reads, the guest calls the model host, preopens/cache are wired, and the typed error lift works across the component boundary | Weekly / path-filtered / manual (`.github/workflows/wasm.yaml`); required before release tags; per-push CI keeps the `wasm32-wasip2` compile check |
| **Live model** | `cargo make test-live` | One ignored native test: the configured live model produces validator-clean reconciliation and synthesis over an adversarial fixture lead set (cross-source overlap, authority disagreement, evidence gap); per-leg repair counts are reported, not asserted | Operator-invoked: before a release tag and after judgment-prompt (`crates/slice/prompts/`, `crates/change/prompts/`) or answer-schema changes — never ordinary CI |

Each fact has one owning rung. The native suites own workflow behavior; the WASM smoke owns only facts unique to the component boundary; the live test owns only "a real model can do this" — it adds no new workflow assertions. Do not copy an assertion onto another rung for reassurance.

### The fixture adapter

One test-support package (`harness/fixtures`) implements both adapter axes as ordinary native Rust: controlled leads (including the adversarial set), controlled evidence with stable authority and claim anchors, deterministic guidance, an observable build output, and typed failures. Native tests reach it through `SourceSeam` / `TargetSeam` via the shared provider in `crates/change/tests/common/`; the WASM smoke reaches the identical core through the combined-world WIT shim (`fixture_adapter.wasm`). Do not add another mock adapter, mock model, or fixture copy — extend the fixture core and let both rungs inherit the behavior.

Model doubles come from upstream: `omnia-testkit` owns the scripted/recording harness and runtime hosting. Specify owns only workflow scenario content — the leads, evidence, scripted answers, and assertions.

## Integration-first policy

Integration tests live in each crate's `tests/` directory and assert against public boundaries — stdout JSON, exit codes, filesystem state. Each `tests/<area>.rs` file is its own auto-discovered test binary — `crates/change/tests/handlers.rs`, `crates/change/tests/full_loop.rs`, and so on. Shared helpers live in the dir form `tests/<helper>/mod.rs` (invisible to auto-discovery) and are declared per binary with `mod <helper>;`. The repo-root `tests/` carries the framework-quality gate (the lightweight `framework` package at `tests/framework/`) and the shared fixture trees under `tests/fixtures/`.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatch contract"](./handler-shape.md#dispatch-contract-commandrs).

## The three layers — minimize the unit layer

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **fewer unit tests**: integration owns every CLI-reachable behavior, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer | Location | Required when | Forbidden when |
| ----- | -------- | ------------- | -------------- |
| **Kernel unit** | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense parse/projection edge matrix whose case-per-cell integration port would inflate the 4-wide subprocess pool | The behavior is reachable through the binary and an integration test already covers it — or could, without a matrix explosion |
| **Crate integration** | `crates/<name>/tests/` | The behavior spans modules within one crate and is unreachable (or impractical to reach) through the binary — internal invariants, filesystem-shape corner cases, registry-pinned schema compilation | The same observable behavior is already asserted through the binary; if a CLI test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process |
| **Wire-contract integration** | `crates/transport/tests/` (the routing crate; the root package carries no test binaries — repo-root `tests/` holds only `framework/` and `fixtures/`) | The behavior is part of the CLI wire contract: flag parsing, exit codes, stdout JSON shape, route dispatch | The assertion re-tests kernel logic already covered elsewhere — wire tests buy routing/projection confidence, not rule-by-rule behavior matrices |

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

**Widening production API to test a private kernel is a last resort, not the lever.** It trades durable public-surface stability for coverage you already have, so prefer collapse-and-keep over widening. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. `cargo llvm-cov nextest` is the brake that guards behavior when deleting unit tests.

### Coverage is the brake on deletion

`cargo llvm-cov` line/region coverage on still-live code is the safety net — not edge-matrix preservation. Before and after a reduction, run the coverage gate over the crate you touched:

```bash
cargo llvm-cov nextest -p <crate> --summary-only
```

A `TOTAL` drop on lines that are still live means real coverage was lost: backfill it with an integration assertion (preferred) or revert that specific deletion. A reduction lands only when coverage holds (a pure collapse of redundant cases is coverage-neutral by construction). Use `cargo llvm-cov nextest` (not bare `cargo test`/`cargo llvm-cov`): nextest's process isolation is what makes the CWD/env-mutating suites pass, and it is the runner CI uses.

## Assertion ownership

- A behavior reducible to a crate API, CLI result, filesystem predicate, validator, compiler, or journal query is a **hard assertion**. It executes automatically on the rung that owns its seam.
- The live test carries **no semantic grading**: its pass condition is the same deterministic validators the native suites use (coverage, schema gates, provenance completeness, tag checks, lifecycle). A judgment about meaning or usefulness that no deterministic predicate can decide has no automated home in this repository.
- Runtime wiring belongs to the WASM smoke; workflow behavior belongs to the native suites. Name the seam an assertion owns before writing it.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `tests/<area>.rs` binary supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 30-char cap below counts the bare `fn` identifier, not the module path.

- Drop tokens the binary name or enclosing module already supplies: in `layout.rs`, write `different_skeletons_error`, not `layout_different_skeletons_is_an_error`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix: six `mark_complete_*` tests become `mod mark_complete { fn idempotent() … }`.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the test body or a `//` comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns; keep identifiers short anyway. The 30-char cap is house style enforced in review, not by a CI check.

## Patterns to follow

- Spin up a real scaffold in a `tempfile::TempDir`. Reach for the shared helpers in `crates/change/tests/common/mod.rs` — the `Project` anchor, the fixture provider, and the scripted-answer builders.
- Compare structured output against checked-in goldens (the merge-engine goldens under `tests/fixtures/merge/`, the wire-schema fixtures under `tests/fixtures/plan/v2/`, the generated answer schemas under `schemas/answers/`). Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -p <crate>` and `git diff` before committing.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.
- Scripted model answers are concise checked-in test inputs colocated with their consuming tests — not a replay catalogue, fixture corpus, or scenario format.

## Golden file discipline

`REGENERATE_GOLDENS=1` is the single supported regeneration switch. After regenerating, run `git diff` on the goldens and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors)).

## Test-side gotchas

- Never hand-edit `metadata.yaml` from a test or fixture. Drive transitions through the orchestration verbs (`specify slice refine` / `build` / `merge` / `drop`), `specify plan transition`, or the stamped-outcome helpers in `crates/change/tests/common/mod.rs` when a test needs a stamped phase outcome.
- The live test retains its temporary project tree on failure and prints the path at start — inspect it rather than re-running blind.
