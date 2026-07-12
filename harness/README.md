# Harness — workflow-core support surfaces

The day-to-day command surface is the three local rungs documented in [the developer loop](../docs/contributing/dev-loop.md): `cargo make test` (native), `cargo make test-wasm` (composed smoke), and `cargo make test-live` (the explicit live trial). The linked-adapter `specify-dev` runtime and its suites live with the adapters at `specify-adapters/harness/native`; this repository keeps the fixture adapter, the composed smoke, and the live-model workflow test.

## Harness commands

Run these from `harness/`:

```shell
cargo make test-wasm # workflow-core composed WASM/WIT smoke
cargo make lint      # harness clippy gate
cargo make guests    # WASM core + fixture adapter; not needed for the native loop
```

The explicit live-model rung runs from the repository root: `cargo make test-live` (see [`live/README.md`](live/README.md)).

## Contents

This directory holds non-shipped workspace surfaces. They are workspace members of the root Cargo workspace and use its shared lockfile, path dependencies, and lint tables; `fixtures/` is a default member (its dependency-free native core and crate-boundary test run with `cargo make test`), while `composed/` and `live/` stay outside the default-member test group. The scheduled/manual composed workflow (`.github/workflows/composed.yaml`; locally `cargo make test-wasm` from the repo root) builds their guests explicitly; per-push CI only checks that the guest crates compile for `wasm32-wasip2`, and the ordinary per-repository `cargo make ci` gate requires no sibling checkout.

| Path        | What                                                                                                                                                                                                                                                          |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `composed/` | The composed WASM smoke: hosts `specify.wasm` with the combined fixture-adapter component bound at both `source:fixture` and `target:fixture`, proving WIT linking, dispatch on both axes, writable preopens, and a scripted drained full loop.               |
| `fixtures/` | The Specify-owned fixture adapter: a deterministic native core supplying both `specify:adapter` axes for the engine's integration tests, plus the combined `fixture_adapter` guest — one wasm32-wasip2 component exporting both axes for composed deployments. |
| `live/`     | The explicit live-model workflow test (`cargo make test-live`): one ignored native trial over the adversarial fixture lead set against the configured cursor model, graded by the deterministic validators, reporting per-leg repair counts.                   |

The larger composed-deployment rig that previously lived at `harness/runtime/` remains retired: the focused `harness/composed/` package owns only the workflow core's WASM boundary, while the native suites under `crates/workflow/tests/` own deterministic workflow behavior.

**Coverage boundary.** The composed smoke exercises WIT bindings, Omnia's dispatch-by-id, mount/preopen wiring, and one scripted full loop without a live model or sibling checkout. The native suite remains the cheaper surface for the broader workflow matrix. See [`composed/README.md`](composed/README.md).
