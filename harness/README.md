# Harness — workflow-core support surfaces

The day-to-day command surface is the three local rungs documented in [the developer loop](../docs/contributing/dev-loop.md): `cargo make test` (native), `cargo make test-wasm` (WASM boundary smoke), and `cargo make test-live` (the explicit live trial). The harness is grouped by execution boundary: `native/` contains the native adapter core and live-model test, while `wasm/` contains the component shim and hosted boundary smoke.

## Harness commands

Run these from `harness/`:

```shell
cargo make test-wasm # WASM/WIT boundary smoke
cargo make lint      # harness clippy gate
cargo make guests    # WASM core + harness adapter; not needed for the native loop
```

The explicit live-model rung runs from the repository root: `cargo make test-live` (see [`native/live-model/README.md`](native/live-model/README.md)).

## Contents

This directory holds non-shipped workspace surfaces. They are workspace members of the root Cargo workspace and use its shared lockfile, path dependencies, and lint tables; `native/adapter/` is a default member, while `native/live-model/`, `wasm/adapter/`, and `wasm/wasm-smoke/` stay outside the default-member test group. The weekly/path-filtered WASM workflow (`.github/workflows/wasm.yaml`; locally `cargo make test-wasm` from the repo root) builds the guests explicitly; per-push CI only checks that the guest crates compile for `wasm32-wasip2`, and the ordinary per-repository `cargo make ci` gate requires no sibling checkout.

- `native/adapter/` — deterministic adapter core used by native workflow tests and the WASM shim.
- `native/live-model/` — explicit live-model workflow test (`cargo make test-live`).
- `wasm/adapter/` — combined adapter component wrapping the native core.
- `wasm/wasm-smoke/` — hosted WASM boundary smoke.

**Coverage boundary.** The WASM smoke exercises WIT bindings, Omnia's dispatch-by-id, mount/preopen wiring, and model-host invocation without a live model or sibling checkout. The native suite remains the cheaper surface for the broader workflow matrix. See [`wasm/wasm-smoke/README.md`](wasm/wasm-smoke/README.md).
