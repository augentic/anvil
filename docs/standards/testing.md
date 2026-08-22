# Testing

Integration-first test posture: `cargo nextest` over public crate and binary boundaries, one self-contained mock source adapter, and structural goldens where bytes are the contract. The unit layer is deliberately thin — integration owns every CLI-reachable behavior and `cargo llvm-cov` is the brake on deletion. This posture deliberately diverges from generic unit-test-first guidance; it overrides any external baseline. Read this before adding a test.

## Posture

Use `cargo make test` rather than `cargo test`. It runs `cargo nextest run --locked --workspace --all-features --no-tests=pass` with `RUSTFLAGS=-Dwarnings` and a clean prelude, matching CI exactly.

`cargo nextest` and `cargo test` differ on `--no-tests=pass`. CI uses nextest with `--no-tests=pass`, so an empty test target is fine — cross-check `cargo test` output if you suspect a target is being skipped.

## The rungs

Emery is tested as a self-contained engine against its own WIT contract. No rung resolves, builds, or inspects `emery-adapters`; external adapters prove their own behavior against the published WIT package.

The fast rung is **native kernel and wire coverage**: `cargo make test` drives the pure engine kernels (reconciliation, the extras gate, the output home) through their public functions with scripted doubles, the CLI wire contract (grammar, exit codes, the C3 HTTP refusal) through the transport router over an inert provider, and the in-process `init` → `specify` journey (`tests/source.rs`) over a provider that scripts every capability (`Model`, `Source`, and the `StateStore`/`BlobStore` storage seam) — engine orchestration without a built component. It runs on every push as part of `cargo make ci`. The v1 prompt-evaluation and wasm-example rungs are archived at tag `v1`. No test builds or spawns the mock source component.

Engine state is observed **through the storage provider and the success envelope, not the filesystem**: since the storage seam (design/portable-storage.md steps 1–2), engine-owned state (the output home, the project record, the component cache) is written through the `omnia_guest::StateStore`/`BlobStore` capabilities — the `wasi:keyvalue`/`wasi:blobstore` imports in deployment, bound host-side to the durable `omnia-filesystem` store — and the native suites script them with the shared in-memory store (`crates/engine/tests/support/storage.rs`, `#[path]`-included per suite). No suite changes the process working directory, and no suite asserts an on-disk layout — the backing is host policy, exercised on the live rung, not in tests. Operator-supplied inputs (a local `.wasm` component) stay real files in a `tempfile::TempDir`.

The wasm32 guest is linted under the guest deny-list (`cargo make lint`'s wasm leg: clippy over `--target wasm32-wasip2` with `clippy-wasm/clippy.toml`), which subsumes the old compile check. The `source` and `runtime` examples remain the in-tree component-shape fixture; they are not a test rung.

### The mock source example

The `source` example (`examples/source.rs`) is the one mock source adapter: a `emery_adapter::SourceAdapter` implementor. The `runtime` example hosts it the way a static deployment declares adapter guests (`source:source`). Do not add another mock adapter, mock model, or mock-adapter copy — extend the example.

Model doubles come from upstream: `omnia-testkit` owns the FIFO `Scripted` script and the request-recording `Harness` (a native `Model` implementation the engine suites script directly). Emery owns only scenario content and assertions.

## Integration-first policy

Integration tests live in each crate's `tests/` directory and assert against public boundaries — stdout JSON, exit codes, scripted storage contents. Each `tests/<area>.rs` file is its own auto-discovered test binary — `crates/engine/tests/synthesise.rs`, `crates/transport/tests/router.rs`, and so on. Crate-private helpers live in the dir form `tests/<helper>/mod.rs` (invisible to auto-discovery), declared per binary with `mod <helper>;`. Developer Guide link integrity is `mdbook-linkcheck2`'s job (`cargo make links`); fixtures are crate-local under `crates/<name>/tests/fixtures/`.

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

### Triage buckets

Applied to every existing (or proposed) `src` `#[cfg(test)]` / `tests.rs`, one bucket per behavior cluster — the same taxonomy `emery-adapters` uses:

- **Delete** — the observable behavior is already asserted by an integration test, or the test is tautological, mock-heavy, or an internal snapshot that gives no boundary signal.
- **Collapse (stay unit)** — a dense pure `(input → output/code)` matrix becomes one table-driven `#[test]` with a block per case; coverage-neutral by construction.
- **Re-home** — behavior reachable through a public seam lands in `crates/<name>/tests/` (or `crates/transport/tests/` for wire behavior).
- **Keep** — a genuinely unreachable defensive branch or private kernel with no public projection, carrying a one-line comment naming which clause it survives under.

**Re-home is not a 1:1 port.** Re-homed coverage is a scenario contract: arrange through the real entry (a CLI verb, a crate `pub` fn, a temp scaffold), act once, and assert at the seam — exit code, JSON `error` discriminant, filesystem artifact shape — never private struct fields re-exposed for the test. A small number of representative scenarios replaces the matrix; the dense edges either stay collapsed in `src` or are dropped as redundant.

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
CRATE=<crate> cargo make cov   # cargo llvm-cov nextest -p <crate> --summary-only
```

A `TOTAL` drop on lines that are still live means real coverage was lost: backfill it with an integration assertion (preferred) or revert that specific deletion. A reduction lands only when coverage holds (a pure collapse of redundant cases is coverage-neutral by construction). Use `cargo llvm-cov nextest` (not bare `cargo test`/`cargo llvm-cov`): nextest's process isolation is what makes the CWD/env-mutating suites pass, and it is the runner CI uses.

## Assertion ownership

- A behavior reducible to a crate API, CLI result, filesystem predicate, validator, or compiler is a **hard assertion**. It executes automatically on the rung that owns its seam.
- Pure kernel behavior belongs to the native suites; `init` / `specify` orchestration belongs to `tests/source.rs`. Name the seam an assertion owns before writing it.

## Test naming

Test function names are identifiers, not sentences — the same brevity rules as production code ([coding-standards.md §"Naming"](./coding-standards.md#naming)) apply. The enclosing context already names the subject: the `tests/<area>.rs` binary supplies `<area>`, and an in-file `mod tests` (or `mod doctor`) supplies its module. Don't restate it in every `fn`. The 25-char cap below counts the bare `fn` identifier, not the module path.

- Drop tokens the binary name or enclosing module already supplies: in `layout.rs`, write `different_skeletons_error`, not `layout_different_skeletons_is_an_error`.
- Group a cluster that shares a subject under a nested `mod <subject>` rather than repeating the subject as a prefix: six `mark_complete_*` tests become `mod mark_complete { fn idempotent() … }`.
- Compress outcome tails to the assertion's shape: `_is_an_error` / `_returns_…_error` → `_errors`; `_validates_cleanly` → `_validates`; `_surfaces_as_a_single_error_entry` → `_one_error`.
- Push the full narrative into the test body or a `//` comment above the `fn`, not the identifier.

`module_name_repetitions` does not fire on `#[test]` fns; keep identifiers short anyway. The 25-char cap is review-only ([coding-standards.md §"Naming"](./coding-standards.md#naming)).

## Patterns to follow

- Script engine state with the shared in-memory storage provider and assert on its contents plus the envelope (`tests/source.rs`); reserve `tempfile::TempDir` for operator-supplied inputs.
- Compare structured output against checked-in goldens (the committed answer schemas under `crates/adapter/schemas/answers/`). Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -p <crate>` and `git diff` before committing.
- Prefer structural assertions (status fields, exit codes, JSON shape) over byte-for-byte prose comparisons.
- Tests that need git operations set deterministic `GIT_*` author/committer env vars so authorship is stable.

## Golden file discipline

There is exactly one supported regeneration switch. Always use `cargo nextest run`, never bare `cargo test`:

```text
REGENERATE_GOLDENS=1 cargo nextest run -p <crate> --test <binary>
```

`REGENERATE_GOLDENS=1` regenerates every checked-in golden — the structural artifact goldens and the generated answer schemas. After regenerating, run `git diff` on the outputs and review every change — a diff that updates a kebab-case error `code` field is a public-contract change (see [coding-standards.md §"Errors"](./coding-standards.md#errors)).

| Crate     | Test binary | Fixture / golden dir(s)          |
| --------- | ----------- | -------------------------------- |
| `adapter` | `answers`   | `crates/adapter/schemas/answers/` |

Binaries not listed here assert structurally and carry no regenerable goldens.
