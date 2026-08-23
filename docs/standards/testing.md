# Testing

Root-led integration posture: the bulk of Emery's product coverage lives in the root package's `tests/` directory, driving the engine "from the engine in" — native scenarios that mimic using the `emery` CLI (minus the wasm wrapper) over scripted capabilities. Crate suites survive only for independently useful library contracts, and the unit layer is near-zero. The root scenarios double as documentation: they tell the story of how Emery is used. This posture deliberately diverges from generic unit-test-first guidance; it overrides any external baseline. Read this before adding a test.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --locked --workspace --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The rungs

Emery is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `emery-adapters`; external adapters prove their own behavior against the published WIT package.

The fast rung is **root product integration plus retained crate contracts**: `cargo make test` drives the root scenario suites (`tests/specify.rs`, `tests/command.rs`, `tests/shelf.rs`, `tests/plugin.rs`) through the in-process command router and HTTP listener over a provider that scripts every capability (`Model`, `Source`, and the `StateStore`/`BlobStore` storage seam) — engine orchestration without a built component — alongside the surviving crate suites (the adapter SDK, artifacts, diagnostics, error, prose, and the CLI-impractical engine invariants). It runs on every push as part of `cargo make ci`. The v1 prompt-evaluation and wasm-example rungs are archived at tag `v1`. No test builds or spawns the mock source component.

Engine state is observed **through the storage provider and the success envelope, not the filesystem**: since the storage seam (design/portable-storage.md steps 1–2), engine-owned state (the output home, the project record, the component cache) is written through the `omnia_guest::StateStore`/`BlobStore` capabilities — the `wasi:keyvalue`/`wasi:blobstore` imports in deployment, bound host-side to the durable `omnia-filesystem` store — and the native suites script them with `emery_testkit` (`Memory`, `Namespaced`). No suite changes the process working directory, and no suite asserts an on-disk layout — the backing is host policy, exercised on the live rung, not in tests. Operator-supplied inputs (a local `.wasm` component, a `sources.toml`) stay real files in a `tempfile::TempDir`.

The `source` and `runtime` examples remain the in-tree component-shape fixture; they are not a test rung.

### The mock source example

The `source` example (`examples/source.rs`) is the one mock source adapter: a `emery_adapter::SourceAdapter` implementor. The `runtime` example hosts it the way a static deployment declares adapter guests (`source:source`). Do not add another mock adapter, mock model, or mock-adapter copy — extend the example. The root suites' scripted `Source` (`tests/support/mod.rs`) is a capability double at the seam, not an adapter: it never parses a workspace and carries no extraction behavior beyond the scenario's scripted evidence.

Model doubles come from upstream: `omnia-testkit` owns the FIFO `Scripted` script and the request-recording `Harness` (a native `Model` implementation the root suites script directly). Storage doubles live in `emery-testkit` until that seam moves upstream. Scenario doubles — the scripted `Source` and the shared provider — live in the root package's `tests/support/mod.rs`. Suites own only scenario content and assertions.

## Root-led policy

The root package's `tests/` directory is the default home for every behavior an operator or MCP client can cause and observe. A root scenario arranges operator inputs (temp files, scripted evidence, scripted model answers), acts through the real entry seams — `emery_transport::command::router` for CLI argv, `emery_transport::http::listener` for the MCP shelf — and observes at public boundaries: exit code, stdout/stderr bytes, the JSON envelope, scripted storage contents, shelf replies.

Root suites are organized by operator story, one auto-discovered test binary per verb-level narrative:

- `tests/specify.rs` — the `specify` → `show` product arc: bindings (argv, `--value`, `--sources`), extraction and the extras gate, reconciliation and synthesis outcomes, the generation home (commit, pruning, corruption), re-mine diffs, and multi-project isolation.
- `tests/command.rs` — the CLI wire contract: the route budget, deleted verbs, help/version/completions, argv normalization, argument failures, exit codes, and the text/JSON channel shape.
- `tests/shelf.rs` — the MCP review surface: the post-`specify` shelf read flow, resource/tool parity, the empty store, method gating, unknown resources, and the C3 typed HTTP refusal.
- `tests/plugin.rs` — plugin-rule mentions against the shipped grammar: live verbs and flags, and that every shipped skill is named by the always-applied rule.

Shared scenario plumbing lives in the dir form `tests/support/mod.rs` (invisible to auto-discovery), declared per binary with `mod support;`. Fixtures are root-local under `tests/<binary>/` and embedded with `include_str!`. Root binaries are gated `#![cfg(not(target_arch = "wasm32"))]`.

Root scenarios read like usage documentation, the same way `credibil/dwn`'s `tests/` directory demonstrates its product: a module doc naming the story, a `//` requirement comment above each test, a short identifier, and section comments separating arrange, act, and observe. A new contributor should be able to learn the CLI from `tests/specify.rs` alone.

If a function needs unit tests, it belongs in a workspace crate, not the binary — see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout) and [handler-shape.md §"Dispatch contract"](./handler-shape.md#dispatch-contract-commandrs).

## The three layers — root owns the product

Every behavior gets a home in exactly one of three layers. Decide the layer **before** writing the test; duplicating an assertion across layers is a defect, not extra safety. The standing bias is **root first, fewer crate tests, near-zero unit tests**: root scenarios own every CLI- or MCP-reachable behavior, crate suites own independently useful library contracts, and the unit layer is reserved for what integration genuinely cannot reach.

| Layer                        | Location                                                            | Required when                                                                                                                                                                                                                                                        | Forbidden when                                                                                                                                                                                     |
| ---------------------------- | ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Root product integration** | root `tests/` (`specify.rs`, `command.rs`, `shelf.rs`, `plugin.rs`) | The behavior is reachable through CLI argv or the MCP listener and observable at a public boundary — exit code, stdout/stderr, the JSON envelope, scripted storage, a shelf reply. This is the default and covers the large majority                                   | The behavior is a library contract with no product projection, or arranging it through the entry seams needs private state no CLI input can produce                                                  |
| **Crate integration**        | `crates/<name>/tests/`                                              | The crate's public API is an independent contract (the published adapter SDK, artifact parsers, diagnostics, error display, the prose walker), **or** the behavior is a product invariant impractical to arrange through the entry seams (e.g. a CAS race)             | The same observable behavior is already asserted through a root scenario; if a root test exists, the crate test must cover a *different* edge, not re-derive the happy path in-process               |
| **Kernel unit**              | `#[cfg(test)] mod tests` (or a sibling `tests.rs`) next to the code | The branch is genuinely unreachable through the CLI (an error variant no flag can trigger, a defensive guard), **or** the behavior is a dense private parse/projection matrix whose integration port would be a case-per-cell explosion                                 | The behavior is reachable through the binary or a retained crate contract and an integration test already covers it — or could, without a matrix explosion                                            |

Rules of thumb:

- **Default to the root.** A behavior the operator can reach belongs in a root scenario. Write the crate test only when the behavior is a standalone library contract or genuinely impractical to arrange at the entry seams — and say why in a comment.
- **Default to deletion.** A unit test survives only if it covers a CLI-unreachable branch worth testing, or it is the cheap home for a dense private edge matrix. Everything reachable belongs to integration.
- **Collapse matrices, don't enumerate them.** A closed set of `(input → code)` cases is one table-driven `#[test]` (or one scenario looping a table), not one `#[test]` per case. Dense parse matrices with no distinct product behavior stay collapsed wherever they live.
- **Re-home, don't 1:1 port.** When deleting a crate or unit test removes the only coverage of a product-reachable behavior, add a *small number* of representative root scenarios — never a case-per-cell port.
- **Don't promote pure-library tests into the root harness.** A test that asserts a crate `pub` fn's contract without touching the entry seams belongs in the crate that owns the code.

### Triage buckets

Applied to every existing (or proposed) test, one bucket per behavior cluster — the same taxonomy `emery-adapters` uses:

- **Delete** — the observable behavior is already asserted by a root scenario, or the test is tautological, mock-heavy, or an internal snapshot that gives no boundary signal.
- **Collapse** — a dense pure `(input → output/code)` matrix becomes one table-driven test with a block per case; coverage-neutral by construction.
- **Re-home** — product-reachable behavior lands in a root scenario, arranged through the entry seams.
- **Keep** — an independent library contract (crate integration) or a genuinely unreachable defensive branch (unit), carrying a one-line comment naming which clause it survives under.

**Re-home is not a 1:1 port.** Re-homed coverage is a scenario contract: arrange through the real entry (CLI argv, a shelf request, a temp file), act once, and assert at the seam — exit code, JSON `error` discriminant, storage contents, shelf reply — never private struct fields re-exposed for the test. A small number of representative scenarios replaces the matrix; the dense edges either stay collapsed in their owning crate or are dropped as redundant.

### Reaching the behavior: design against the entry seams

Before writing a test below the root, decide whether a root scenario can reach the behavior. Ask three questions, then check visibility:

1. **Reachable?** Does some CLI input or shelf request (with scripted capabilities) actually run this code?
2. **Observable?** Does its effect surface at a public boundary — exit code, stdout/stderr, the JSON envelope, scripted storage, a shelf reply?
3. **Affordable?** Can you construct the input and observe the effect through that surface without a case-per-cell explosion or compiling a mock per case?

- **Reachable + observable + affordable** → write the root scenario against the **existing** entry seams. No new API; this is the default and covers the large majority.
- **Reachable + observable but cheap only in-process** (proptests, dense matrices) → if the kernel is an independent `pub` contract, the test lives in `crates/<crate>/tests/`; if it is private, **collapse and keep** a table-driven unit test in place.
- **Unreachable or unobservable** → it is dead code or an implementation detail: make the state un-representable (`unreachable!`, typestate) or delete the assertion. Don't test it.

**Widening production API to test a private kernel is a last resort, not the lever.** It trades durable public-surface stability for coverage you already have, so prefer collapse-and-keep over widening. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. `cargo llvm-cov nextest` is the brake that guards behavior when deleting tests.

### Coverage is the brake on deletion

`cargo llvm-cov` line/region coverage on still-live code is the safety net — not edge-matrix preservation. Before and after a reduction, run the coverage gate:

```bash
cargo make cov                       # workspace-wide: cargo llvm-cov nextest --workspace --summary-only
CRATE=emery-<crate> cargo make cov-crate   # focused: cargo llvm-cov nextest -p <package> --summary-only
```

Root scenarios exercise engine and transport code from the root package, so the workspace-wide run is the default gate for re-homing; the focused form audits a leaf crate's own contract suite. A `TOTAL` drop on lines that are still live means real coverage was lost: backfill it with a root scenario (preferred) or revert that specific deletion. A reduction lands only when coverage holds (a pure collapse of redundant cases is coverage-neutral by construction). Use `cargo llvm-cov nextest` (not bare `cargo test`/`cargo llvm-cov`): nextest's process isolation is what makes the CWD/env-mutating suites pass, and it is the runner CI uses.

## Assertion ownership

- A behavior reducible to a CLI result, shelf reply, storage predicate, crate API, validator, or compiler is a **hard assertion**. It executes automatically on the rung that owns its seam.
- Product orchestration belongs to the root suites; independent library contracts belong to their crates. Name the seam an assertion owns before writing it.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `tests/<area>.rs` binary supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 25-char cap below counts the bare `fn` identifier, not the module path.

- Drop tokens the binary name or enclosing module already supplies: in `shelf.rs`, write `empty_store_hints`, not `shelf_empty_store_hints_at_specify`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the `//` requirement comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns; keep identifiers short anyway. The 25-char cap is review-only ([coding-standards.md §"Naming"](./coding-standards.md#naming)).

## Patterns to follow

- Script engine state with `emery_testkit::Memory` (or `Namespaced`) and assert on its contents plus the envelope (`tests/support/mod.rs`); reserve `tempfile::TempDir` for operator-supplied inputs.
- Open every root scenario with a `//` requirement comment and separate arrange/act/observe with section comments — the scenario is documentation first.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons, except where bytes are the contract (`show` renders the committed document alone).
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.
