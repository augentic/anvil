# Self-Assembling Wasm Deployment

> Status: Stages 1 and 3 landed — Stage 2 diagnostics remain draft
>
> Owns: the operator-facing `specify` executable, deployment assembly, and Specify's fail-closed guest resolver over the adapter store and project cache.
>
> Builds on: [Specify on Omnia](architecture.md). Program: RFCs 70–74.

## Landed (Stages 1 + 3)

- Embedded engine guest bytes in the shipped binary (`include_bytes!` via root `build.rs`)
- Pure `omnia::runtime!` composition — no `omnia.toml`, no pre-run guest closure
- Fail-closed adapters-only `GuestResolver` (verify-and-load; no launcher download path)
- Mounts + optional read-only `adapter add` seed preopen from `crates/launcher`
- Exact routed identities (`source:<name>@<version>`, `target:<name>`, …) resolve from store / project cache

Live description: [CLI architecture](../docs/contributing/cli-architecture.md), [AGENTS.md § launcher](../AGENTS.md#the-rust-workspace-specify-cli).

Stage 1's pre-run adapter enumeration is **superseded and deleted**. Do not resurrect a host front door or guest table.

## Remaining (Stage 2 — draft)

1. `resolution.json` (or equivalent) recording the effective resolved deployment for an invocation
2. Digest pin checks / doctor surface: `deployment show|doctor` (or equivalent read-only verbs)
3. MCP `/mcp/<name>` projection of resolved guests when useful for operators
4. Optional precompile / `wasm-pkg.toml` polish beyond today's scaffold

## Non-goals (unchanged)

- Teaching Omnia Specify vocabulary
- Statically linking first-party adapters into the released Wasm distribution
- Making the native host the default operator distribution
- Selecting which adapters a project should use ([RFC-71](rfc-71-discovery.md) / [RFC-72](rfc-72-migration.md) / [RFC-74](rfc-74-program.md))
