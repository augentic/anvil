# Harness — workflow-core support surfaces

The day-to-day command surface is the three-rung `cargo make dev -- <command>` loop documented in [the developer loop](../docs/contributing/dev-loop.md). The linked-adapter `specify-dev` runtime and its deterministic full-loop suite live with the adapters at `specify-adapters/harness/native`; this repository keeps only workflow-core WebAssembly fixtures and composed tests.

## Harness commands

Run these from `harness/`:

```shell
cargo make test-wasm # workflow-core WASM/WIT init and replayed full loop
cargo make lint         # harness clippy gate
cargo make guests # WASM core + echo fixtures; not needed for the native loop
```

## Contents

This directory holds non-shipped workspace surfaces. They are workspace members of the root Cargo workspace (outside the default-member test group) and use its shared lockfile, path dependencies, and lint tables. The scheduled/manual composed workflow (`.github/workflows/composed.yaml`; locally `cargo make test-wasm` from the repo root) builds their guests explicitly; per-push CI only checks that the guest crates compile for `wasm32-wasip2`, and the ordinary per-repository `cargo make ci` gate requires no sibling checkout.

| Path        | What                                                                                                                                                                                                                                                                                                                                      |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `composed/` | The workflow-core WASM/WIT profile: hosts `specify.wasm` with echo source/target fixtures, proves adapter links, ID dispatch, and writable preopens, then drives a full loop with checked-in Omnia replay fixtures.                                                                                                                            |
| `fixtures/` | The echo adapter guests — skeleton `specify:adapter` components usable in composed deployments so the WIT imports resolve without the sibling adapters checkout.                                                                                                                                                                          |

The larger composed-deployment rig that previously lived at `harness/runtime/` remains retired: the focused `harness/composed/` package owns only the workflow core's WASM boundary, while `specify-adapters/harness/native` owns deterministic linked-adapter behavior.

**Coverage boundary.** The composed profile exercises WIT bindings, Omnia's dispatch-by-id, mount/preopen wiring, and one replay-backed full loop without a live model or sibling checkout. The native suite remains the cheaper surface for the broader workflow matrix. See [`composed/README.md`](composed/README.md).
