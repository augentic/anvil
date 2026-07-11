# Harness — non-default-member support surfaces

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
cargo make lint         # harness clippy gate
cargo make build-guests # WASM core + echo fixtures; not needed for the native loop
```

`cargo make dev-run` itself runs from `harness/`; use the built `specify-dev` binary from the project directory for workflow commands that do not accept `--project-dir`. From either repo root, `make dev-run PROJECT=/path/to/project ARGS='plan status'` is the same shim without changing directory (see `scripts/dev.sh`).

## Contents

This directory holds workspace surfaces that sit outside the root CI gates: they are **workspace members** of the root cargo workspace (shared lockfile, path deps, lint tables) but are **excluded from `default-members`**, so no root gate — `cargo make ci`, `test`, `lint`, `doc`, `vet`, `deny`, `fmt` — ever builds, lints, tests, or audits them.

| Path        | What                                                                                                                                                                                                                                                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `fixtures/` | The echo adapter guests — skeleton `specify:adapter` components usable in composed deployments so the WIT imports resolve without the sibling adapters checkout.                                                                                                                                                               |
| `native/`   | The Rust-native shim: the `specify-dev` binary and its provider — the same typed command/HTTP routers and workflow operations as the wasm guest, with a native-only linked adapter catalog, cursor / replay / mock model backends, and per-adapter MCP reference shelves. Requires the `../specify-adapters` sibling checkout. |

The composed-deployment rig that previously lived at `harness/runtime/` is retired: the native harness (`harness/native/`) owns the dev loop and full-loop integration coverage, and the crate-level suites keep the deterministic coverage.

**Coverage boundary.** The native suite exercises everything except the wasm-only surface: WIT bindings, Omnia's dispatch-by-id, and mount/preopen wiring. Those stay with the shipped guest — the `evals/drivers/guest-execute-loop.sh` composed run and targeted adapter tests — so a green native suite plus a green guest eval is the full picture, and neither alone is.
