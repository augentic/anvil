# Testing

Integration-first test posture: `cargo nextest` over public crate and binary boundaries, one self-contained mock adapter supplying both `emery:adapter` axes, and structural goldens where bytes are the contract. The unit layer is deliberately thin — integration owns every CLI-reachable behavior and `cargo llvm-cov` is the brake on deletion. This posture deliberately diverges from generic unit-test-first guidance; it overrides any external baseline. Read this before adding a test.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --locked --workspace --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly. The live trial is an opt-in rung (below) so ordinary test runs never call a model.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The two rungs

Emery is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `emery-adapters`; external adapters prove their own behavior against the published WIT package.

| Rung                   | Command           | Proves                                                                                                                                                                                                                                                                                               | Cadence                                                                                                                                                           |
| ---------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Native integration** | `cargo make test` | The complete workflow — reconciliation, synthesis, build, merge, lifecycle — through the engine crates' public operations, the `mock` catalog behind the offline `native` provider, and scripted model doubles                                                                                                           | Every push (part of `cargo make ci`)                                                                                                                              |
| **Prompt evaluation**  | `cargo make eval` | The shared native runner drives the production operator rhythm over linked adapters and checks plan completion plus requirement provenance; per-leg repair counts are reported, not asserted | Operator-invoked: before a release tag and after judgment-prompt (`crates/slice/prompts/`, `crates/change/prompts/`) or answer-schema changes — never ordinary CI |

There is no automated WASM boundary rung: the wasm32 guests are compile-checked (`cargo check --lib -p emery --examples --target wasm32-wasip2`), and component-boundary execution — WIT dispatch on both axes, metadata reads, guest-to-host model wiring, preopens — is exercised by the operator-invoked wasm example (`cargo make wasm-run`, live model; see [`examples/wasm/README.md`](../../examples/wasm/README.md)) and by the matching wasm example in `emery-adapters`.

Each fact has one owning rung. The native suites own workflow behavior; the live test owns only "a real model can do this" — it adds no new workflow assertions. Do not copy an assertion onto another rung for reassurance.

### The mock crate

`crates/mock` is Emery's single mock crate (`publish = false`, dev-dep'd by the suite-bearing crates — a legal Cargo dev-dependency cycle). It owns the SDK-native mock adapter core (`mock::behaviour`, both `emery:adapter` axes): controlled leads (including the adversarial set), controlled evidence with stable authority and claim anchors, deterministic guidance, an observable build output, and typed failure profiles registered as explicit catalog identities (`mock::registry::catalog`). Native tests reach it through the offline `native::Provider` via the host-only `mock::session::Session` and the generalized `mock::invoke::run`; the example components (`examples/wasm/source.rs` / `target.rs`) wire the same core into the SDK's `source!` / `target!` export macros for the operator-run wasm example. Resolver / install / store suites use their suite-local provider under `crates/project/tests/support/` instead — component-metadata resolution is not a catalog concern. Do not add another mock adapter, mock model, or mock-adapter copy — extend the core and let every consumer inherit the behavior.

Model doubles come from upstream: `omnia-testkit` owns the FIFO `Scripted` script and the request-recording `Harness`; `mock::session::Session` binds that harness behind the judgment legs. Emery owns only workflow scenario content — the leads, evidence, the scripted answer corpus (`mock::answers`), and assertions. The scripted double answers regardless of the request; prompt quality is owned by the prompt-evaluation rung, not by the native suites.

The live prompt-evaluation rung is the lab-only `crates/probe` library composed by the root `eval` example (`examples/eval/`). `native` owns the catalog machinery, `DynModel` erasure, and the seam `Provider`; the probe library owns the typed case runner (`probe::case` — workflow and build cases over real `emery` verbs, one stable retained sandbox per case, deterministic gates), telemetry, and grading, receiving its catalog, model factory, and `cases/` root from its composition root; the `client` feature adds the shared cursor composition (`probe::client` — the lazily connected `DevModel`, the process tracing init (console plus an optional `EVAL_LOG` file copy), and the argv dispatch); the root example owns the Tokio runtime and the mock catalog binding, and `cargo make eval <case>` names one case (case data lives in `examples/eval/cases/<id>/case.toml`, never in makefile argv). Only the model is live. The same `probe::client` is consumed by the matching `eval` example in `augentic/emery-adapters`, which also owns the first-party catalog declaration and its cases root.

## Integration-first policy

Integration tests live in each crate's `tests/` directory and assert against public boundaries — stdout JSON, exit codes, filesystem state. Each `tests/<area>.rs` file is its own auto-discovered test binary — `crates/change/tests/handlers.rs`, `crates/change/tests/full_loop.rs`, and so on. Cross-crate helpers come from the `mock` and `native` dev-dependencies (`use mock::…`, `use native::…`); crate-private helpers live in the dir form `tests/<helper>/mod.rs` (invisible to auto-discovery), declared per binary with `mod <helper>;`. Developer Guide link integrity is `mdbook-linkcheck2`'s job (`cargo make links`); fixtures are crate-local under `crates/<name>/tests/fixtures/`.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatch contract"](./handler-shape.md#dispatch-contract-commandrs).

## The three layers — minimize the unit layer

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **fewer unit tests**: integration owns every CLI-reachable behavior, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer                         | Location                                                                                                                           | Required when                                                                                                                                                                                                                                            | Forbidden when                                                                                                                                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Kernel unit**               | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code                                                                | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense parse/projection edge matrix whose case-per-cell integration port would inflate the 4-wide subprocess pool | The behavior is reachable through the binary and an integration test already covers it — or could, without a matrix explosion                                                    |
| **Crate integration**         | `crates/<name>/tests/`                                                                                                             | The behavior spans modules within one crate and is unreachable (or impractical to reach) through the binary — internal invariants, filesystem-shape corner cases, registry-pinned schema compilation                                                     | The same observable behavior is already asserted through the binary; if a CLI test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process |
| **Wire-contract integration** | `crates/transport/tests/` (the routing crate; the root package carries no test binaries) | The behavior is part of the CLI wire contract: flag parsing, exit codes, stdout JSON shape, route dispatch                                                                                                                                               | The assertion re-tests kernel logic already covered elsewhere — wire tests buy routing/projection confidence, not rule-by-rule behavior matrices                                 |

Rules of thumb:

- **Default to deletion.** A unit test survives only if it covers a CLI-unreachable branch worth testing, or it is the cheap home for a dense edge matrix that would otherwise bloat integration. Everything reachable belongs to integration.
- **Collapse matrices, don't enumerate them.** A unit test that walks a closed set of `(input → code)` cases is one table-driven `#[test]` with a block per case, not one `#[test]` per case. The five workflow matrices reduced in the 2026-06 sweep (`config`, `adapter/core`, `build/wire`, plan `validate`, `propose`) each collapsed this way with **zero** `cargo llvm-cov` movement.
- **Re-home, don't 1:1 port.** When deleting a unit test removes the only coverage of a CLI-reachable behavior, add a *small number* of representative integration cases — never a case-per-cell port (the subprocess pool is the scarce budget).
- **Don't promote pure-library tests into the binary harness.** A test that never spawns the binary belongs in the crate that owns the code (this is a policy violation the harness comment cannot excuse).

### Reaching the behavior: design against the public surface

Before writing a unit test, decide whether integration can reach the behavior. Ask three questions, then check visibility:

1. **Reachable?** Does some CLI input (or a crate-`pub` fn) actually run this code?
2. **Observable?** Does its effect surface at a public boundary — stdout JSON, exit code, filesystem, a `pub` return value?
3. **Affordable?** Can you construct the input and observe the effect through that surface without a subprocess-pool explosion (a case-per-cell CLI port) or compiling a mock per case?

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
- Workflow behavior belongs to the native suites; runtime wiring has no automated home here (the operator-run wasm example exercises it). Name the seam an assertion owns before writing it.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `tests/<area>.rs` binary supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 30-char cap below counts the bare `fn` identifier, not the module path.

- Drop tokens the binary name or enclosing module already supplies: in `layout.rs`, write `different_skeletons_error`, not `layout_different_skeletons_is_an_error`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix: six `mark_complete_*` tests become `mod mark_complete { fn idempotent() … }`.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the test body or a `//` comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns; keep identifiers short anyway. The 30-char cap is house style enforced in review, not by a CI check.

## Patterns to follow

- Spin up a real scaffold in a `tempfile::TempDir`. Reach for the shared helpers in `mock` — `mock::session::Session` (its constructors mint the tempdir and pin the project cache), the generalized `mock::invoke::run::<Op, _, _>` invoker helper, and the `mock::answers` corpus.
- Compare structured output against checked-in goldens (the crate-local `spec-*` cases under `crates/artifacts/tests/fixtures/` and `crates/slice/tests/fixtures/`, the generated answer schemas under `crates/project/answers/` and `crates/slice/answers/`). Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -p <crate>` and `git diff` before committing.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.
- Scripted model answers live in `mock::answers` as the single shared copy the suites consume; keep them concise and structural, not a scenario format.

## Golden file discipline

There is exactly one supported regeneration switch. Always use `cargo nextest run`, never bare `cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

`REGENERATE_GOLDENS=1` regenerates every checked-in golden — the structural artifact goldens and the generated answer schemas. After regenerating, run `git diff` on the outputs and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors)).

| Crate               | Test binary     | Fixture / golden dir(s)                            |
| ------------------- | --------------- | -------------------------------------------------- |
| `slice`             | `merge_goldens` | `crates/slice/tests/fixtures/spec-*`               |
| `project` / `slice` | `answers`       | `crates/project/answers/`, `crates/slice/answers/` |

Binaries not listed here assert structurally and carry no regenerable goldens.

## Test-side gotchas

- Never hand-edit `metadata.yaml` from a test or fixture. Drive transitions through the orchestration verbs (`emery slice refine` / `build` / `merge` / `drop`) or `emery plan undo` when a test needs a stamped phase outcome.
- The live test retains its temporary project tree on failure and prints the path at start — inspect it rather than re-running blind.
