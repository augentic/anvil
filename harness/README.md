# Harness — non-default-member support surfaces

This directory holds workspace surfaces that sit outside the root CI gates: they are **workspace members** of the root cargo workspace (shared lockfile, path deps, lint tables) but are **excluded from `default-members`**, so no root gate — `cargo make ci`, `test`, `lint`, `doc`, `vet`, `deny`, `fmt` — ever builds, lints, tests, or audits them.

## Contents

| Path | What |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fixtures/` | The echo adapter guests — skeleton `specify:adapter` components usable in composed deployments so the WIT imports resolve without the sibling adapters checkout. |
| `native/` | The Rust-native shim: the `specify-dev` binary and its `NativeProvider` — the same typed command/HTTP routers and workflow operations as the wasm guest, with cursor / replay / mock model backends and the per-adapter MCP reference shelves. Requires the `../specify-adapters` sibling checkout. |

The composed-deployment rig that previously lived at `harness/runtime/` is retired: the native harness (`harness/native/`) owns the dev loop and full-loop integration coverage, and the crate-level suites keep the deterministic coverage.

**Coverage boundary.** The native suite exercises everything except the wasm-only surface: WIT bindings, Omnia's dispatch-by-id, and mount/preopen wiring. Those stay with the shipped guest — the `evals/drivers/guest-execute-loop.sh` composed run and targeted adapter tests — so a green native suite plus a green guest eval is the full picture, and neither alone is.

Manual entry points (from inside `harness/`):

```shell
cargo make build-guests                 # build specify.wasm + the echo fixtures
cargo make dev -- <specify args>        # run the native specify-dev shim (e.g. `-- serve --port 7737`)
cargo make test-native                  # the native harness suite (full loop, seams, replay, MCP shelves)
cargo make lint                         # clippy over the harness crates (not covered by the root gate)
```
