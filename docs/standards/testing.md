# Testing

Integration-first test posture: `cargo nextest` over public crate and binary boundaries, one self-contained harness adapter supplying both `specify:adapter` axes, and structural goldens where bytes are the contract. The unit layer is deliberately thin — integration owns every CLI-reachable behavior and `cargo llvm-cov` is the brake on deletion. Read this before adding a test.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --locked --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly. The selection is the default workspace members — `crates/*` (including `testkit`), `examples`, and the `tests` (`checks`) package; the WASM boundary smoke and the live trial are opt-in rungs (below) so ordinary test runs never compile Wasmtime or call a model.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The three rungs

Specify is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `specify-adapters`; external adapters prove their own behavior against the published WIT package.

| Rung | Command | Proves | Cadence |
| ---- | ------- | ------ | ------- |
| **Native integration** | `cargo make test` | The complete workflow — reconciliation, synthesis, build, merge, lifecycle — through the workflow crates' public operations, `testkit`'s fixture adapter seams, and scripted / committed-replay model doubles | Every push (part of `cargo make ci`) |
| **WASM boundary** | `cd examples && cargo make test-wasm` | The WASM seam only: the combined `adapter_wasm.wasm` loads, both axes dispatch through generated bindings, metadata reads, the guest calls the model host, preopens/cache are wired, and the typed error lift works across the component boundary | Weekly / path-filtered / manual (`.github/workflows/wasm.yaml`); required before release tags; per-push CI keeps the `wasm32-wasip2` compile check |
| **Live model** | `cd examples && cargo make test-live` | One native example: the configured live model produces validator-clean reconciliation and synthesis over an adversarial fixture lead set (cross-source overlap, authority disagreement, evidence gap); per-leg repair counts are reported, not asserted | Operator-invoked: before a release tag and after judgment-prompt (`crates/slice/prompts/`, `crates/change/prompts/`) or answer-schema changes — never ordinary CI |

Each fact has one owning rung. The native suites own workflow behavior; the WASM smoke owns only facts unique to the component boundary; the live test owns only "a real model can do this" — it adds no new workflow assertions. Do not copy an assertion onto another rung for reassurance.

### The testkit crate

`crates/testkit` is Specify's single test-support crate (`publish = false`, dev-dep'd by the suite-bearing crates — a legal Cargo dev-dependency cycle). It owns the fixture adapter core (`testkit::adapter`, both `specify:adapter` axes): controlled leads (including the adversarial set), controlled evidence with stable authority and claim anchors, deterministic guidance, an observable build output, and typed failures. Native tests reach it through `SourceSeam` / `TargetSeam` via the unified `testkit::Provider` (`ScriptedProvider` / `ReplayProvider`); `examples/wasm/adapter.rs` wraps the same core as the `adapter-wasm` component example used by the boundary smoke. Do not add another mock adapter, mock model, or fixture-adapter copy — extend the core and let both rungs inherit the behavior.

Model doubles come from upstream: `omnia-testkit` owns the scripted/replay/recorder harness and runtime hosting, re-exported through `testkit::model`. Specify owns only workflow scenario content — the leads, evidence, the scripted answer corpus (`testkit::answers`), the committed replay fixtures, and assertions.

### Replay fixtures and `REGENERATE_FIXTURES=1`

The model-dispatching `change` suites run against committed replay fixtures at `crates/change/tests/fixtures/replay/<suite>/<test>/` — one directory per test, because replay keys are canonical rendered prompts and two tests answering the same prompt differently would collide. Fixture rows churn whenever a judgment prompt changes; that is by design. Re-record them from the scripted corpus with:

```bash
REGENERATE_FIXTURES=1 cargo nextest run -p change
```

then `git diff` the fixture rows and review the prompt-side changes like any other golden. Suites that intentionally feed schema-invalid answers to exercise the repair loop (e.g. `judgment`) stay on `ScriptedProvider` — the replay engine rejects invalid answers by design.

## Integration-first policy

Integration tests live in each crate's `tests/` directory and assert against public boundaries — stdout JSON, exit codes, filesystem state. Each `tests/<area>.rs` file is its own auto-discovered test binary — `crates/change/tests/handlers.rs`, `crates/change/tests/full_loop.rs`, and so on. Cross-crate helpers come from the `testkit` dev-dependency (`use testkit::…`); crate-private helpers live in the dir form `tests/<helper>/mod.rs` (invisible to auto-discovery), declared per binary with `mod <helper>;`. The repo-root `tests/` carries only the lightweight `checks` package (`boundaries`, `links`, `authoring`); fixtures are crate-local under `crates/<name>/tests/fixtures/`.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatch contract"](./handler-shape.md#dispatch-contract-commandrs).

## The three layers — minimize the unit layer

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **fewer unit tests**: integration owns every CLI-reachable behavior, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer | Location | Required when | Forbidden when |
| ----- | -------- | ------------- | -------------- |
| **Kernel unit** | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense parse/projection edge matrix whose case-per-cell integration port would inflate the 4-wide subprocess pool | The behavior is reachable through the binary and an integration test already covers it — or could, without a matrix explosion |
| **Crate integration** | `crates/<name>/tests/` | The behavior spans modules within one crate and is unreachable (or impractical to reach) through the binary — internal invariants, filesystem-shape corner cases, registry-pinned schema compilation | The same observable behavior is already asserted through the binary; if a CLI test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process |
| **Wire-contract integration** | `crates/transport/tests/` (the routing crate; the root package carries no test binaries — repo-root `tests/` holds only the `checks` package) | The behavior is part of the CLI wire contract: flag parsing, exit codes, stdout JSON shape, route dispatch | The assertion re-tests kernel logic already covered elsewhere — wire tests buy routing/projection confidence, not rule-by-rule behavior matrices |

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

- Spin up a real scaffold in a `tempfile::TempDir`. Reach for the shared helpers in `testkit` — the unified `Provider` (its owned constructors mint and enter the tempdir), the `run::<Op, _, _>` invoker helper, and the `testkit::answers` corpus.
- Compare structured output against checked-in goldens (the crate-local `spec-*` cases under `crates/artifacts/tests/fixtures/` and `crates/slice/tests/fixtures/`, the generated answer schemas under `crates/project/answers/` and `crates/slice/answers/`). Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -p <crate>` and `git diff` before committing.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.
- Scripted model answers live in `testkit::answers` as the replay fixtures' regeneration source of truth; keep them concise and structural, not a scenario format.

## Golden file discipline

`REGENERATE_GOLDENS=1` regenerates checked-in goldens; `REGENERATE_FIXTURES=1` re-records the committed replay fixture rows (above). After regenerating, run `git diff` on the outputs and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors)).

## Test-side gotchas

- Never hand-edit `metadata.yaml` from a test or fixture. Drive transitions through the orchestration verbs (`specify slice refine` / `build` / `merge` / `drop`) or `specify plan transition` when a test needs a stamped phase outcome.
- The live test retains its temporary project tree on failure and prints the path at start — inspect it rather than re-running blind.
