# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `.emery/` layout boundary, time injection, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "emery"`) at the repo root. [`src/main.rs`](../../src/main.rs) is one `omnia::runtime!` command-mode invocation over the cursor-bound backends: the engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm`, produced by the root `build.rs` — a child wasm32 build into an isolated target directory, then in release builds an ahead-of-time wasmtime serialize so startup deserializes instead of JIT-compiling the engine; debug builds embed the raw component and JIT at startup; there is no placeholder fallback), and `program: "emery"` forwards raw argv to the explicit `command_guest`. Deployment policy is static and CWD-rooted, expressed inline in the invocation: the invocation directory mounts as the guest's `.` (no ancestor walk — a verb run below the project root fails typed), the storage hosts bind engine state — component cache and store included — to a durable filesystem store rooted at the CWD-relative `.emery`, and adapter guests, when present, are declared in that same invocation — a built `.wasm` path (or `include_bytes!`) plus a `/mcp/source/<name>` route, the way the journey host declares its mock source in [`examples/runtime.rs`](../../examples/runtime.rs). The shipped `src/main.rs` embeds the engine only. Dynamic admission — the `GuestResolver`, the seeded-cache and verified-store legs, the pre-bound MCP listener — is deferred until dynamic loading returns; there is no download path (ADR-0002 deletions). Every invocation — help, version, and grammar rejections included — runs in the emery guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`crates/transport`).

The authoritative leaf → root crate graph (with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). Headline shape: `error` is the leaf; `engine` owns the domain operations (`init`, `specify`, adapter resolution); `transport` owns typed routing; the root package's `src/main.rs` owns native deployment policy inline and its wasm32 lib is the one provider (the component seam is the only production seam, ADR-0002). The journey's mock source lives in the `source` example, not a workspace crate.

There is no lint engine or `Check` substrate. Repo consistency is the mdBook links gate (`cargo make links`).

The artifact validation rule registry (`emery_artifacts::validate`) sits on `artifacts`, which depends on none of the engine crates nor anything named lint, so an artifact rule cannot reach workflow lifecycle types. `artifacts` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry the engine layer reads, alongside `diagnostics` and `error` at the bottom. The neutral `Diagnostic` substrate lives in the `diagnostics` crate, so every check producer mints findings without depending on anything named `lint`.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/emery-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `error` re-introduces the cycle the layering was designed to avoid; do not.

**New workspace crates** are an exception, not the default.

The `source` example (`examples/source.rs`) is the one mock source adapter: a `emery_adapter::SourceAdapter` implementor. The `runtime` example hosts it the way a static deployment declares adapter guests. It carries no production lifecycle authority and never enters the shipped guest. Do not add another mock adapter — extend this example.

## Deployment: the Wasm guest

The guest's provider is a bare unit at the composition root (`src/lib.rs`) carrying the engine's capabilities: the model (`omnia_guest::Model`, WASI-backed defaults), the source seam (`emery_adapter::Source`), and — since the storage seam (design/portable-storage.md steps 1–2) — the storage pair (`omnia_guest::StateStore` / `BlobStore`), whose wasm32 default bodies ride the `wasi:keyvalue` / `wasi:blobstore` imports; the runtime binds those hosts to a durable `omnia-filesystem` store rooted at the invocation directory's `.emery` (`src/main.rs`), so the backing is deployment policy, never engine code. Everything else is structural: paths are fixed constants relative to named preopens (`emery_engine::handler::ExecutionPaths::deployed()`), and adapter dispatch — `extract` and `metadata` — rides the `emery:adapter/source` WIT imports directly from the engine's cfg-gated dispatch functions (typed refusals on native, where no seam exists). The native provider was deleted at the Phase 3 spine cut (ADR-0002: "deleted, not demoted"); `init` → `specify` coverage is the in-process native rung (`tests/source.rs`), while pure kernels test over injected runners and scripted inputs.

The root `emery` package carries the Omnia deployment unit under `src/`: the guest cdylib (`src/lib.rs`, the `wasi:cli/run` exporter and `wasi:http/incoming-handler` — the latter answers every mutating path with `emery_transport::http`'s typed refusal, C3 — plus the bare model provider) and the shipped runtime (`src/main.rs`, one `omnia::runtime!` invocation embedding the engine bytes). Commands live in `engine` as transport-neutral `Operation<P>` implementations beside their domain kernels (shared plumbing in `emery_engine::handler`). `crates/transport/src/command.rs` owns the explicit typed command route inventory over `Invoker<P>`; `command::router` binds the caller's provider into that grammar. The WASI shim only constructs the provider and hands the assembled router to `omnia_guest`'s WASI command adapter (`execute_wasi`). The routing design is documented in [handler-shape.md](handler-shape.md).

## Domain modules of note

- **`crates/engine/src/resolve/`** — source-adapter resolution over the typed `AdapterSelector`: the `resolver::Component` kernel (read-only re-resolution) and the `ensure` kernels (the provisioning leg, one component, no manifest). Operations call them over the provider's metadata dispatch (`metadata::runner` closes over the `Source` capability — the WIT `metadata` import on wasm32, a scripted mock natively). `Component` keeps its injected `Runner`, the native kernel-test seam. Non-identity metadata is cached against the component digest.
- **`crates/artifacts/src/evidence.rs`** — the typed Evidence `Document` / `Claim` wire shapes (mirroring the WIT `evidence` / `claim` records) and their deterministic validation (`Document::validate`: kebab grammars, the per-kind claim id requirement). The typed serde parse is the load gate for every on-disk artifact; validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`.

The lenient v1 module trees in `artifacts` (spec parsers, provenance, the task/decision/leads validators) were deleted at the Phase 3 spine cut and are documented at tag `v1`.

## Adapter component resolution

`resolver::Component` dispatches identities by routed id (the WIT `metadata` export) and probes the project component cache first for bare names and persisted component selectors (the `<name>.wasm` object in the `adapters` blobstore container, mirrored at init from an operator-supplied local file). Under the shipped static deployment a dispatch lands only on a guest declared in the runtime invocation; anything else fails at the dispatch seam. The dynamic legs (seeded-cache answer, verified global-store entry, resolve-on-miss) are deferred with the resolver; there is no download path and no sibling-checkout or build-tree probe. Storage locations are fixed key and container-name formulas, carried as the `Locations` math threaded through `ExecutionPaths::deployed()` — the same names resolve against whatever backing the deployment binds. A binding names the axis; a component bound on the wrong axis fails at the dispatch seam — no deployed guest exports the requested `<axis>:<name>` id.

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/emery-adapters`. The carve-out discipline (leaner lint posture and minimal `[workspace.dependencies]`) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. There is no declared-tool surface. No `emery-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.emery/` is framework-managed state every CLI verb writes through. Two owners cover it: `emery_engine::project::Project::path` for `project.yaml`, and `emery_engine::home::Home` for the spec output home (generation directories plus the `current` pointer). Do not hard-code `.emery/` paths elsewhere; a new `.emery/` path lands on one of those owners.

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
