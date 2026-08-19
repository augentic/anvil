# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `.emery/` layout boundary, time injection, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "emery"`) at the repo root. [`src/main.rs`](../../src/main.rs) is one `omnia::runtime!` command-mode invocation over the cursor-bound backends: the engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.bin`, produced by the root `build.rs` — a child wasm32 build into an isolated target directory, then in release builds an ahead-of-time wasmtime serialize so startup deserializes instead of JIT-compiling the engine; debug builds embed the raw component and JIT at startup; there is no placeholder fallback), the mounts, the fail-closed adapters-only `GuestResolver`, the pre-bound HTTP trigger listener (`http_listener: launcher::http_listener()` — split bind policy, any bind failure is a startup failure, its local address injected as the guest-visible `HTTP_ADDR` and the fully-formed shelf base as `MCP_URL_BASE`), and the adapter MCP path hook (`http_paths: launcher::mcp_route` — `/mcp/<axis>/<name>[@<version>]` back onto the routed adapter id; declined paths and definitive resolver misses 404, genuine claimed-route faults 500) are `crates/launcher` expressions, and `program: "emery"` forwards raw argv. Adapters fault in mid-run by exact routed id — a seeded project-cache entry answers first for pinned and bare ids alike (the co-dev seed always wins); a seedless pinned id resolves a verified global-store entry. There is no download path (ADR-0002 deletions): the embedded first-party registry (staged from `EMERY_EMBED_DIR` at build time — release builds embed the published first-party components pinned in `scripts/first-party.txt`; unstaged builds carry an empty table) and the local store are the only other legs, and nothing local is a typed miss. Every invocation — help, version, and grammar rejections included — runs in the emery guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`crates/transport`).

The authoritative leaf → root crate graph (with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). Headline shape: `error` is the leaf; `engine` owns the domain operations (`init`, `specify`, adapter resolution); `transport` owns typed routing; `launcher` owns native deployment policy; `guest` is the one provider (the component seam is the only production seam, ADR-0002); `mock` is the test-support adapter core behind the journey's `mock-component` fixture.

There is no lint engine or `Check` substrate. Repo consistency is the mdBook links gate (`cargo make links`).

The artifact validation rule registry (`artifacts::validate`) sits on `artifacts`, which depends on none of the engine crates nor anything named lint, so an artifact rule cannot reach workflow lifecycle types. `artifacts` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry the engine layer reads, alongside `diagnostics` and `error` at the bottom. The neutral `Diagnostic` substrate lives in the `diagnostics` crate, so every check producer mints findings without depending on anything named `lint`.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/emery-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `error` re-introduces the cycle the layering was designed to avoid; do not.

**New workspace crates** are an exception, not the default.

`crates/mock` is the single test-support crate that prevents adapter test behavior from being copied across suites. It owns the shared wasm-free mock source-adapter core (canonical `adapter::Source` implementors over `mock::behaviour`), exported as the journey's seam fixture by `crates/mock-component`. It carries no production lifecycle authority and never enters the shipped guest.

## Deployment: the Wasm provider

The engine core (`engine` / `transport`) is deployment-neutral: it consumes model, adapter, ensure/resolve, and anchoring capabilities through provider traits. One provider exists — the **Wasm deployment** (the `guest` crate behind the root cdylib's `guest::export!()`) satisfies them over WIT imports: component adapters, the global store with digest verification, and Omnia hosting. The native provider was deleted at the Phase 3 spine cut (ADR-0002: "deleted, not demoted"); integration coverage runs over the component seam via the dev-only journey host, while pure kernels test natively against the provider traits directly.

The root `emery` package carries the Omnia deployment unit under `src/`: the guest cdylib (`src/lib.rs`, one `guest::export!()` invocation) and the shipped runtime (`src/main.rs`, one `omnia::runtime!` invocation embedding the engine bytes). The `guest` crate (`crates/guest`) owns the `workflow`-world WIT bindings, the WIT-backed `Provider`, and the `export!` macro that wires both transports — `wasi:cli/run` (`CliGuest` + `Guest::run`) and `wasi:http/incoming-handler` (`Http` + `Guest::handle`, which answers every mutating path with `transport::http`'s typed refusal — C3) — so downstream deployments build the identical guest from one macro invocation instead of vendoring sources. Commands live in `engine` as transport-neutral `Operation<P>` implementations beside their domain kernels (shared plumbing in `engine::handler`). `crates/transport/src/command.rs` owns the explicit typed command route inventory over `Invoker<P>`; the WASI and native shims only construct invokers and adapt transport output. The routing design is documented in [handler-shape.md](handler-shape.md).

## Domain modules of note

- **`crates/engine/src/resolve/`** — deployment-neutral `Resolver` capability (`resolve_source` plus the async `ensure_source` provisioning leg over `AdapterSelector`) plus the shipped `resolver::Component` implementation and its `ensure` kernels (one component, no manifest). Operations and kernels receive the resolver through their provider; the WASI provider delegates to component resolution. Non-identity metadata comes from the component's deterministic `metadata` export through the provider-supplied runner and is cached against the component digest.
- **`crates/artifacts/src/evidence.rs`** — the typed Evidence `Document` / `Claim` wire shapes (mirroring the WIT `evidence` / `claim` records) and their deterministic validation (`Document::validate`: kebab grammars, the per-kind claim id requirement). The typed serde parse is the load gate for every on-disk artifact; validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`.

The lenient v1 module trees in `artifacts` (spec parsers, provenance, the task/decision/leads validators) were deleted at the Phase 3 spine cut and are documented at tag `v1`.

## Adapter component resolution

`resolver::Component` dispatches pinned identities by routed id (the WIT `metadata` export; under the shipped deployment the host launcher answers a pinned id from the seeded project cache when an entry with that name exists — the co-dev seed always wins — and otherwise resolves the component from the global single-file store `<store-root>/<name>@<version>.wasm` with verify-on-read) and probes the project component cache first for bare names and persisted component selectors (`<project-cache>/components/<name>.wasm`, mirrored at init from an operator-supplied local file). A bare name whose cache probe misses dispatches its unversioned routed id, which the shipped launcher resolves through the embedded first-party registry (`crates/launcher/build.rs` over `EMERY_EMBED_DIR`; the pins live in `scripts/first-party.txt`), logging every settled identity to stderr; there is no download path and nothing local is a typed miss (`adapter-not-found`). There is no sibling-checkout or build-tree probe. Both roots derive from the carried `Locations` value (`EMERY_HOME`, else `~/.emery`, with `store/` and `cache/` beneath it), captured once at each composition root and threaded through `ExecutionPaths`. A binding names the axis; a component bound on the wrong axis fails at the dispatch seam — no deployed guest exports the requested `<axis>:<name>` id.

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/emery-adapters`. The carve-out discipline (leaner lint posture and minimal `[workspace.dependencies]`) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. There is no declared-tool surface. No `emery-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.emery/` is framework-managed state every CLI verb writes through. Two owners cover it: `engine::project::Project::path` for `project.yaml`, and `engine::home::Home` for the spec output home (generation directories plus the `current` pointer). Do not hard-code `.emery/` paths elsewhere; a new `.emery/` path lands on one of those owners.

## Time injection

Functions that record a timestamp into a serialised artifact accept `now: jiff::Timestamp` from the operation boundary. Domain kernels do not call `Timestamp::now()`; operation call sites inject time so tests can pin it deterministically.

## Atomic writes

Use `yaml_write` (in `crates/artifacts/src/atomic.rs`) for any file a concurrent reader may observe mid-write (e.g. `project.yaml`). It serialises to `NamedTempFile::new_in(parent)` and `persist`-renames over the target so readers either see the prior bytes or the new bytes. Plain `fs::write` is reserved for files no other process reads concurrently with the writer (one-shot scratch output, fixtures inside a tempdir test).

The standards-side phrasing of the rule lives in [coding-standards.md §"YAML, JSON, and atomic writes"](./coding-standards.md#yaml-json-and-atomic-writes).

## Toolchain

Rust stable per `rust-toolchain.toml` (channel `stable`, components `clippy`, `rust-src`, `rustfmt`). WASM targets pre-installed via `targets = ["aarch64-apple-darwin", "wasm32-wasip2", "x86_64-apple-darwin"]`.

`rustfmt.toml` uses unstable nightly features (`unstable_features = true`, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`). Format with nightly:

```bash
cargo +nightly fmt --all
```

`cargo make fmt` does this for you.

## Supply chain

`cargo-vet` and `cargo-deny` gate `cargo make ci`; `cargo-audit`, `cargo-outdated`, and `cargo-udeps` are advisory tasks run on demand (`cargo make audit` / `outdated` / `deps`). The vet task is check-only (`cargo vet --locked`) — regeneration is deliberately not part of the gate, since regenerating exemptions before checking would auto-exempt anything unaudited. When a new dependency lands:

1. Add it to `[workspace.dependencies]` in the root `Cargo.toml` with a major-version pin (e.g. `serde = { version = "1", features = ["derive"] }`). Per-crate `Cargo.toml` references it as `serde.workspace = true`.
2. Run `cargo vet regenerate imports`, `cargo vet regenerate exemptions`, and `cargo vet regenerate unpublished`; review the `supply-chain/` diff, then commit it.
3. Check `deny.toml` allows the dependency's licence. The current allowlist is in `deny.toml`; add a new SPDX id only after confirming compatibility with MIT-OR-Apache-2.0.

`clippy::multiple_crate_versions` is silenced workspace-wide (`Cargo.toml`'s `[workspace.lints.clippy]`); duplicate transitive versions are audited by hand via `cargo tree --duplicates` on each `cargo update`, not gated through a ratchet.

## Skill / CLI responsibility split

Every deterministic operation lives in this CLI: kebab-case validation, project scaffolding, adapter resolution and caching, and schema validation. The surviving `/emery:init` skill shells out for all of those.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

[`AGENTS.md`](../../AGENTS.md) is the source of truth for vocabulary and the crate map.
