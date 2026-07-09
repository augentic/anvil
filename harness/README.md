# Harness — the parked composed-deployment test rig

This directory holds the **parked** end-to-end test rig for the shipped wasm surface: it composes the built `specify.wasm` core guest (plus adapter components) into a real Omnia deployment and drives it, proving the WASI wiring the native test suites cannot reach.

`harness/runtime` and `harness/fixtures` are **workspace members** of the root cargo workspace (shared lockfile, path deps, lint tables) but are **excluded from `default-members`**, so no root gate — `cargo make ci`, `test`, `lint`, `doc`, `vet`, `deny`, `fmt` — ever builds, lints, tests, or audits them. The rig runs only when invoked manually via `cargo make` from this directory. The deterministic dispatch contract it used to gate — grammar, routing, envelopes, exit codes — is covered natively by `crates/dispatch/tests/`; wasmtime and omnia own the argv/exit plumbing itself, and re-proving that on every commit is not worth the wasm build cost.

Run the rig on demand when touching the WIT contract, the guest shim (`crates/specify`), or the omnia pin — all commands from inside `harness/`:

```shell
cargo make build-guests                 # build specify.wasm + the echo fixtures
cargo make fetch-adapters               # populate the adapter store (composed workflow tests)
cargo make test                         # the whole rig
cargo nextest run -p runtime -E 'test(workflow::)'   # one area
cargo make lint                         # clippy over the rig (not covered by the root gate)
```

## Contents

| Path | What |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runtime/` | The rig crate: the `runtime-replay` binary (the `specify` binary's `omnia::runtime!` macro over `ModelDefault`, so no `cursor-agent` is needed) plus the composed-deployment integration tests (`tests/{composed,workflow,widened,mcp}.rs`). |
| `fixtures/` | The echo adapter guests — skeleton `specify:adapter` components the rig links against the workflow guest so the WIT imports resolve without the sibling adapters checkout. |

What the rig still uniquely proves (the reason it is parked rather than deleted):

- **Link dispatch** — `specify:adapter/source` / `target` calls route through omnia's host-mediated dispatch to a real release-built adapter component, surviving a pending `omnia:model/completion` future (`runtime/tests/workflow.rs`).
- **The shared mount** — journal appends, plan stamps, and merge folds land on the `"."` preopen the host seeded (`workflow.rs`, `widened.rs`).
- **Adapter MCP shelves** — each adapter serves its references on `/mcp/<name>` beside the workflow guest (`mcp.rs`, `workflow.rs`).

The adapter version pin the composed workflow tests resolve is `runtime/tests/adapters.pin`, shared with the `fetch-adapters` make task.

Because the rig sits outside the root gates, it can drift compile-wise against the engine crates; `cargo make lint` (or `cargo check -p runtime -p fixtures --all-targets` from the repo root) is the first step of any revival.
