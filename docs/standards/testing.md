# Testing

Root-led integration posture: the bulk of Emery's product coverage lives in the root package's `tests/` directory, driving the engine "from the engine in" — native scenarios that mimic using the `emery` CLI (minus the wasm wrapper) over scripted capabilities. Crate suites survive only for independently useful library contracts, and the unit layer is near-zero. The root scenarios double as documentation: they tell the story of how Emery is used. This posture deliberately diverges from generic unit-test-first guidance; it overrides any external baseline. Read this before adding a test.

## Posture

Use `make test` rather than `cargo test`. It runs `cargo nextest run --locked --workspace --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The rungs

Emery is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `emery-adapters`; external adapters prove their own behavior against the published WIT package.

The fast rung is **root product integration plus retained crate contracts**: `make test` drives the root scenario suites (`tests/specify.rs`, `tests/command.rs`, `tests/plugin.rs`) through the in-process command router over a provider that scripts every capability (`Model`, `Source`, and the `StateStore`/`BlobStore` storage capabilities) — engine orchestration without a built component — alongside the surviving crate suites (the adapter SDK, prose, and the CLI-impractical engine invariants). It runs on every push as part of `make ci`. The v1 prompt-evaluation and wasm-example rungs are archived at tag `v1`.

The **component rung** rides the same `make test` invocation as its own unpublished workspace package, `examples/component/` (the shape the adapters repo uses for `examples/conformance`): `tests/component.rs` there runs the shipped deployment — the engine component the runtime embeds (the root build script's `$OUT_DIR/emery.cwasm`) under the policy `src/lib.rs` declares through `omnia::runtime!` — and the built mock adapter component under the real omnia runtime. The package's `run` harness overlays that manifest (`omnia_test::host::Deployment::from(emery::manifest())`) with the scenario's project mount, path root, arguments, and declared adapter guests, and drives it through the runtime's own wiring (`emery::Hooks`) over omnia-test's `Backends<ScriptedModel>`: a scripted host-side model (`WasiModelCtx` double) and the in-memory `wasi:keyvalue`/`wasi:blobstore` defaults; `engine` hands a scenario the embedded engine bytes. It owns only what the native scenarios cannot reach — the `wasi:cli/run` wrapper, the WIT lowering on both sides of the `emery:adapter/source` seam, the real path loader and its digest pin, the reference-tool closure over real wasi-model tool streams — and never re-asserts a `tests/specify.rs` behavior. The runtime inherits stdout, so component scenarios observe the exit status and the storage handles, not the envelope. This is the one rung that builds a wasm component: `examples/component/build.rs` drives `omnia_test::build::Components` to compile `--example adapter` for `wasm32-wasip2` into its `OUT_DIR`, incremental after the first build — a separate package because the root build script runs for every build of the shipped binary, and a test fixture has no place there.

Engine state is observed **through the storage provider and the success envelope, not the filesystem**: since the storage capabilities (design/portable-storage.md steps 1–2), engine-owned state (the output home, the generation store) is written through the `omnia_guest::StateStore`/`BlobStore` capabilities — the `wasi:keyvalue`/`wasi:blobstore` imports in deployment, bound host-side to the durable `omnia-filesystem` store — and the native suites script them with omnia-test's `guest::{Memory, Namespaced}`; local-component loads are scripted through the support module's `Plugins` double, recording each load request and its resolved digest. The component rung reads the same state back through the in-memory host backends (`Backends::state`, `Backends::object`). No suite changes the process working directory — except the config-discovery scenarios, which enter a `tempfile::TempDir` to control what a bindingless run finds (safe because nextest runs each test in its own process) — and no suite asserts an on-disk layout: the backing is host policy, exercised on the live rung, not in tests. Operator-supplied inputs (a local `.wasm` component, an `emery.toml`) stay real files in a `tempfile::TempDir` (the component rung's `omnia_test::host::scratch()`).

### The mock adapter

The `adapter` example (`examples/adapter/`, a root-package `[[example]]` cdylib) is the one mock source adapter: a `emery_adapter::SourceAdapter` implementor whose extract calls the host `Model` over its fixture tree, with its embedded prose under `examples/adapter/prose/`. It is both the live-journey fixture (the `runtime` example hosts the built component with the Cursor backend and a path-only plugin `locations:` list) and the component rung's happy-path fixture. Do not add another mock adapter, mock model, or mock-adapter copy — extend this example. The root suites' scripted `Source` (`tests/support/mod.rs`) is a capability double, not an adapter: it never parses a workspace and carries no extraction behavior beyond the scenario's scripted evidence.

Capability doubles come from omnia's `omnia-test` crate (a native-only dev-dependency): `guest::Scripted`, the FIFO request-recording `Model` the root suites script directly, `guest::ScriptedLoader`, the keyed `Plugins` loader (seeded with `defaulting(digest("ab"))`, the fixed digest the `specify` scenarios assert in their envelopes), and the storage pair `guest::{Memory, Namespaced}`; `omnia_test::delegate!` implements the capability traits on a provider by forwarding to those fields. Both `Scripted` and the host-side `ScriptedModel` are strict: running past the script panics, and dropping a script with unconsumed turns fails the test — a scenario scripts exactly what its run consumes (a refusal before the model is reached scripts no answers; a transport failure is scripted as `Err(..)`, not as an empty script). Scenario doubles — the scripted `Source` and the shared provider — live in the root package's `tests/support/mod.rs`. Host-side doubles for the component rung are omnia-test's `host::{ScriptedModel, Backends, Scratch, scratch}`, re-exported by the `component` package beside its `run` overlay. Suites own only scenario content and assertions.

## Root-led policy

The root package's `tests/` directory is the default home for every behavior an operator can cause and observe. A root scenario arranges operator inputs (temp files, scripted evidence, scripted model answers), acts through the real entry point — `emery_cli::run` for CLI argv — and observes at public boundaries: exit code, stdout/stderr bytes, the JSON envelope, scripted storage contents.

Root suites are organized by operator story, one auto-discovered test binary per verb-level narrative:

- `tests/specify.rs` — the `specify` → `show` product arc: bindings (argv, `--description`, `--config`, root discovery), extraction and the extras gate, reconciliation and synthesis outcomes, the generation home (commit, pruning, corruption), re-mine diffs, and multi-project isolation.
- `tests/command.rs` — the CLI wire contract: the route budget, deleted verbs, help/version/completions, argv normalization, argument failures, exit codes, and the text/JSON channel shape.
- `tests/plugin.rs` — plugin-rule mentions against the shipped grammar: live verbs and flags, and that every shipped skill is named by the always-applied rule.
The component rung, `examples/component/tests/component.rs`, sits beside them as the fourth story — bare-name dispatch to a statically declared adapter, path loads with an agreeing and a disagreeing digest pin, a seamless component refused by the loader, and the reference-tool round-trip served in-process — in its own package only because its fixture build needs a build script the shipped binary must not carry.

Shared scenario plumbing lives in the dir form `tests/support/mod.rs` (invisible to auto-discovery), declared per binary with `mod support;`. `command.rs` and `plugin.rs` path-include `tests/support/verbs.rs` for the live verb set parsed from `emery --help`. Fixtures are root-local under `tests/<binary>/` and embedded with `include_str!` (the component rung reaches the `specify` answers the same way). Root binaries are gated `#![cfg(not(target_arch = "wasm32"))]`.

Root scenarios read like usage documentation, the same way `credibil/dwn`'s `tests/` directory demonstrates its product: a module doc naming the story, a `//` requirement comment above each test, a short identifier, and section comments separating arrange, act, and observe. A new contributor should be able to learn the CLI from `tests/specify.rs` alone.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatch contract"](./handler-shape.md#dispatch-contract-commandrs).

## The three layers — root owns the product

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **root first, fewer crate tests, near-zero unit tests**: root scenarios own every CLI-reachable behavior, crate suites own independently useful library contracts, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer                        | Location                                                            | Required when                                                                                                                                                                                                                                                        | Forbidden when                                                                                                                                                                                     |
| ---------------------------- | ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Root product integration** | root `tests/` (`specify.rs`, `command.rs`, `plugin.rs`) and the component rung (`examples/component/tests/component.rs`) | The behavior is reachable through CLI argv and observable at a public boundary — exit code, stdout/stderr, the JSON envelope, scripted storage. This is the default and covers the large majority. The component rung takes only what needs the wasm boundary itself | The behavior is a library contract with no product projection, or arranging it through the entry points needs private state no CLI input can produce                                                  |
| **Crate integration**        | `crates/<name>/tests/`                                              | The crate's public API is an independent contract (the published adapter SDK, the prose walker), **or** the behavior is a product invariant impractical to arrange through the entry points (e.g. a CAS race)                                              | The same observable behavior is already asserted through a root scenario; if a root test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process               |
| **Kernel unit**              | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense private parse/projection matrix whose integration port would be a case-per-cell explosion                                 | The behavior is reachable through the binary or a retained crate contract and an integration test already covers it — or could, without a matrix explosion                                            |

Rules of thumb:

- **Default to the root.** A behavior the operator can reach belongs in a root scenario. Write the crate test only when the behavior is a standalone library contract or genuinely impractical to arrange at the entry points — and say why in a comment.
- **Default to deletion.** A unit test survives only if it covers a CLI-unreachable branch worth testing, or it is the cheap home for a dense private edge matrix. Everything reachable belongs to integration.
- **Collapse matrices, don't enumerate them.** A closed set of `(input → code)` cases is one table-driven `#[test]` (or one scenario looping a table), not one `#[test]` per case. Dense parse matrices with no distinct product behavior stay collapsed wherever they live.
- **Re-home, don't 1:1 port.** When deleting a crate or unit test removes the only coverage of a product-reachable behavior, add a *small number* of representative root scenarios — never a case-per-cell port.
- **Don't promote pure-library tests into the root harness.** A test that asserts a crate `pub` fn's contract without touching the entry points belongs in the crate that owns the code.

### Triage buckets

Applied to every existing (or proposed) test, one bucket per behavior cluster — the same taxonomy `emery-adapters` uses:

- **Delete** — the observable behavior is already asserted by a root scenario, or the test is tautological, mock-heavy, or an internal snapshot that gives no boundary signal.
- **Collapse** — a dense pure `(input → output/code)` matrix becomes one table-driven test with a block per case; coverage-neutral by construction.
- **Re-home** — product-reachable behavior lands in a root scenario, arranged through the entry points.
- **Keep** — an independent library contract (crate integration) or a genuinely unreachable defensive branch (unit), carrying a one-line comment naming which clause it survives under.

**Re-home is not a 1:1 port.** Re-homed coverage is a scenario contract: arrange through the real entry (CLI argv, a temp file), act once, and assert at the public boundary — exit code, JSON `error` discriminant, storage contents — never private struct fields re-exposed for the test. A small number of representative scenarios replaces the matrix; the dense edges either stay collapsed in their owning crate or are dropped as redundant.

### Reaching the behavior: design against the entry points

Before writing a test below the root, decide whether a root scenario can reach the behavior. Ask three questions, then check visibility:

1. **Reachable?** Does some CLI input (with scripted capabilities) actually run this code?
2. **Observable?** Does its effect surface at a public boundary — exit code, stdout/stderr, the JSON envelope, scripted storage?
3. **Affordable?** Can you construct the input and observe the effect through that surface without a case-per-cell explosion or compiling a mock per case?

- **Reachable + observable + affordable** → write the root scenario against the **existing** entry points. No new API; this is the default and covers the large majority.
- **Reachable + observable but cheap only in-process** (proptests, dense matrices) → if the kernel is an independent `pub` contract, the test lives in `crates/<crate>/tests/`; if it is private, **collapse and keep** a table-driven unit test in place.
- **Unreachable or unobservable** → it is dead code or an implementation detail: make the state un-representable (`unreachable!`, typestate) or delete the assertion. Don't test it.

**Widening production API to test a private kernel is a last resort, not the lever.** It trades durable public-surface stability for coverage you already have, so prefer collapse-and-keep over widening. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. `cargo llvm-cov nextest` is the brake that guards behavior when deleting tests.

### Coverage is the brake on deletion

`cargo llvm-cov` line/region coverage on still-live code is the safety net — not edge-matrix preservation. Before and after a reduction, run the coverage gate:

```bash
make cov                       # workspace-wide: cargo llvm-cov nextest --workspace --summary-only
CRATE=emery-<crate> make cov-crate   # focused: cargo llvm-cov nextest -p <package> --summary-only
```

Root scenarios exercise engine and transport code from the root package, so the workspace-wide run is the default gate for re-homing; the focused form audits a leaf crate's own contract suite. A `TOTAL` drop on lines that are still live means real coverage was lost: backfill it with a root scenario (preferred) or revert that specific deletion. A reduction lands only when coverage holds (a pure collapse of redundant cases is coverage-neutral by construction). Use `cargo llvm-cov nextest` (not bare `cargo test`/`cargo llvm-cov`): nextest's process isolation is what makes the CWD/env-mutating suites pass, and it is the runner CI uses.

## Assertion ownership

- A behavior reducible to a CLI result, storage predicate, crate API, validator, or compiler is a **hard assertion**. It executes automatically on the rung that owns its surface.
- Product orchestration belongs to the root suites; independent library contracts belong to their crates. Name the surface an assertion owns before writing it.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `tests/<area>.rs` binary supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 25-char cap below counts the bare `fn` identifier, not the module path.

- Drop tokens the binary name or enclosing module already supplies: in `command.rs`, write `unknown_verb_refuses`, not `command_unknown_verb_refuses_typed`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the `//` requirement comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns; keep identifiers short anyway. The 25-char cap is review-only ([coding-standards.md §"Naming"](./coding-standards.md#naming)).

## Patterns to follow

- Script engine state with `omnia_test::guest::Memory` (or `Namespaced`) and assert on its contents plus the envelope (`tests/support/mod.rs`); reserve `tempfile::TempDir` for operator-supplied inputs.
- Put a scenario in the component rung (`examples/component/tests/component.rs`) only when the wasm boundary is the subject; anything the in-process router can reach belongs in the native suites, where the envelope is observable.
- Open every root scenario with a `//` requirement comment and separate arrange/act/observe with section comments — the scenario is documentation first.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons, except where bytes are the contract (`show` renders the committed document alone).
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.
