# Harness — non-default-member support surfaces

The day-to-day command surface over this harness is the three-rung `make dev-*` loop — `dev-check` (model-free), `dev-live` (live model), `dev-full` (WASM boundary) — documented in [the developer loop](../docs/contributing/dev-loop.md).

## Quick start

The native loop requires sibling checkouts so the shim can link the adapter crates directly:

```text
<parent>/
├── specify/
└── specify-adapters/
```

Build the shim once, then run it from the project under test. Its CLI anchors at the current directory, just like the shipped CLI:

```shell
cd <parent>/specify
cargo build -p specify-dev

cd /path/to/project
<parent>/specify/target/debug/specify-dev target resolve omnia
<parent>/specify/target/debug/specify-dev plan status
```

The resolve command should report `location: native` and a reference such as `rust:target:omnia`. Bare names resolve linked Rust crates; pinned identities such as `omnia@1.0.0` remain component-store concerns and are intentionally rejected by the native resolver. The linked sources are `captures`, `documentation`, `intent`, `screenshots`, and `typescript`; the linked targets are `contracts`, `omnia`, and `vectis`. No adapter `.wasm` build or staging is needed.

Judgment operations use `cursor-agent` by default, connected lazily on first use. Install and authenticate `cursor-agent` for a live run. To use recorded model answers instead:

```shell
SPECIFY_DEV_MODEL=replay MODEL_REPLAY_DIR=/path/to/fixtures \
  <parent>/specify/target/debug/specify-dev plan execute
```

Serve the same operations over HTTP, together with the linked adapters' MCP reference shelves:

```shell
<parent>/specify/target/debug/specify-dev serve \
  --project-dir /path/to/project \
  --port 7737
```

## Harness commands

Run these from `harness/`:

```shell
cargo make dev-run -- target resolve omnia --project-dir /path/to/project
cargo make test-native  # full loop, seams, replay, and MCP shelves
cargo make test-composed # workflow-core WASM/WIT init and replayed full loop
cargo make lint         # harness clippy gate
cargo make guests # WASM core + echo fixtures; not needed for the native loop
```

`cargo make dev-run` itself runs from `harness/`; use the built `specify-dev` binary from the project directory for workflow commands that do not accept `--project-dir`. From either repo root, `make dev-run PROJECT=/path/to/project ARGS='plan status'` is the same shim without changing directory (see `scripts/dev.rs`).

## Contents

This directory holds non-shipped workspace surfaces. They are **workspace members** of the root Cargo workspace (shared lockfile, path deps, lint tables) but are **excluded from `default-members`**. Dedicated CI jobs run the native cross-repository and composed WebAssembly profiles; the ordinary per-repository `cargo make ci` gate does not require sibling checkouts or guest builds.

| Path        | What                                                                                                                                                                                                                                                                                                                                      |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `composed/` | The workflow-core WASM/WIT profile: hosts `specify.wasm` with echo source/target fixtures, proves adapter links, ID dispatch, and writable preopens, then drives a full loop with checked-in Omnia replay fixtures.                                                                                                                            |
| `fixtures/` | The echo adapter guests — skeleton `specify:adapter` components usable in composed deployments so the WIT imports resolve without the sibling adapters checkout.                                                                                                                                                                          |
| `native/`   | The Rust-native shim: the `specify-dev` binary and its provider — the same typed command/HTTP routers and workflow operations as the wasm guest, with a native-only linked adapter catalog, cursor plus Omnia replay/scripted model backends, and per-adapter MCP reference shelves. Requires the `../specify-adapters` sibling checkout. |

The larger composed-deployment rig that previously lived at `harness/runtime/` remains retired: the focused `harness/composed/` package owns only the workflow core's WASM boundary, while the native harness owns deterministic full-loop behavior.

**Coverage boundary.** The composed profile exercises WIT bindings, Omnia's dispatch-by-id, mount/preopen wiring, and one replay-backed full loop without a live model or sibling checkout. The native suite remains the cheaper surface for the broader workflow matrix. See [`composed/README.md`](composed/README.md).
