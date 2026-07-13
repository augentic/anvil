# Harness — workflow-core support surfaces

The day-to-day command surface is the three local rungs documented in [the developer loop](../docs/contributing/dev-loop.md): `cargo make test` (native), `cargo make test-wasm` (WASM boundary smoke), and `cargo make test-live` (the explicit live trial). The linked-adapter `specify-dev` runtime and its suites live with the adapters at `specify-adapters/harness/native`; this repository keeps the harness adapter, the WASM boundary smoke, and the live-model workflow test.

## Harness commands

Run these from `harness/`:

```shell
cargo make test-wasm # WASM/WIT boundary smoke
cargo make lint      # harness clippy gate
cargo make guests    # WASM core + harness adapter; not needed for the native loop
```

The explicit live-model rung runs from the repository root: `cargo make test-live` (see [`live-model/README.md`](live-model/README.md)).

## Contents

This directory holds non-shipped workspace surfaces. They are workspace members of the root Cargo workspace and use its shared lockfile, path dependencies, and lint tables; `adapter/` is a default member (its dependency-free `native/` core is exercised via the workflow suites under `cargo make test`), while `wasm-smoke/` and `live-model/` stay outside the default-member test group. The weekly/path-filtered WASM workflow (`.github/workflows/wasm.yaml`; locally `cargo make test-wasm` from the repo root) builds their guests explicitly; per-push CI only checks that the guest crates compile for `wasm32-wasip2`, and the ordinary per-repository `cargo make ci` gate requires no sibling checkout.

| Path | What |
| ---- | ---- |
| `wasm-smoke/` | WASM boundary smoke: hosts `specify.wasm` with the combined adapter component bound at both `source:fixture` and `target:fixture`, proving WIT linking, dispatch on both axes, writable preopens, model-host wiring, and the typed error lift. |
| `adapter/` | The Specify-owned harness adapter: parallel `native/` and `wasm/` halves supplying both `specify:adapter` axes — the library for engine integration tests, plus the combined `adapter` component for hosted deployments. |
| `live-model/` | The explicit live-model workflow test (`cargo make test-live`): one ignored native trial over the adversarial fixture lead set against the configured cursor model, graded by the deterministic validators, reporting per-leg repair counts. |

**Coverage boundary.** The WASM smoke exercises WIT bindings, Omnia's dispatch-by-id, mount/preopen wiring, and model-host invocation without a live model or sibling checkout. The native suite remains the cheaper surface for the broader workflow matrix. See [`wasm-smoke/README.md`](wasm-smoke/README.md).
