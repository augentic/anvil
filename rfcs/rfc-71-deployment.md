# Self-Assembling Wasm Deployment

> Status: Stages 1–3 largely landed — Stage 2 diagnostics remain draft
>
> Owns: the operator-facing `emery` executable, deployment assembly, and Emery's fail-closed guest resolver over the adapter store and project cache.
>
> Builds on: [Emery on Omnia](architecture.md). Program: the platform-migration series ([platform.md](platform.md)). Install transport: [RFC-76](archive/rfc-76-adapter-install.md) (archived).

## Landed (Stages 1 + 3, Stage 2 MCP)

- Embedded engine guest bytes in the shipped binary (`include_bytes!` via root `build.rs`; child wasm32 build for plain `cargo install --git`)
- Pure `omnia::runtime!` composition — no `omnia.toml`, no pre-run guest closure
- Fail-closed adapters-only `GuestResolver` with launcher pull-on-miss install from the fixed first-party GHCR mapping
- Mounts + optional read-only `adapter add` seed preopen from `crates/launcher`
- Exact routed identities (`source:<name>@<version>`, `target:<name>`, …) resolve from store / project cache
- MCP `/mcp/<axis>/<name>[@<version>]` projection via `launcher::mcp_route` + per-invocation `http_listener`
- Engine precompile: in release builds `build.rs` AOT-serializes the wasm32 engine to `$OUT_DIR/emery.bin` (target-triple wasmtime artifact); startup deserializes instead of JIT-compiling the engine. Debug builds embed the raw component at the same path and JIT at startup — adapters remain raw wasm and keep the `jit` feature in the host

Live description: [CLI architecture](../docs/contributing/cli-architecture.md), [AGENTS.md § launcher](../AGENTS.md#the-rust-workspace-emery-cli).

Stage 1's pre-run adapter enumeration is **superseded and deleted**. Do not resurrect a host front door or guest table.

## Remaining (Stage 2 diagnostics — draft)

1. `resolution.json` (or equivalent) recording the effective resolved deployment for an invocation
2. Digest pin checks / doctor surface: `deployment show|doctor` (or equivalent read-only verbs)
3. Adapter precompile (local AOT cache or per-triple publication) toward a `jit`-less host — the engine half landed above

## Non-goals (unchanged)

- Teaching Omnia Emery vocabulary
- Statically linking first-party adapters into the released Wasm distribution
- Making the native host the default operator distribution
- Selecting which adapters a project should use ([RFC-88](rfc-88-detached-changes.md))
