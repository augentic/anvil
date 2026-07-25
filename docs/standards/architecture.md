# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `Layout<'a>` boundary, time injection, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "specify"`) at the repo root. [`src/main.rs`](../../src/main.rs) is one `omnia::runtime!` command-mode invocation over the cursor-bound backends: the engine guest is embedded as static component bytes (`include_bytes!(env!("SPECIFY_WASM"))`, resolved by the root `build.rs` — explicit override, else a child wasm32 build into an isolated target directory; there is no placeholder fallback), the mounts and the fail-closed adapters-only `GuestResolver` are `crates/launcher` expressions (RFC-70 Stage 3), and `program: "specify"` forwards raw argv. Adapters fault in mid-run by exact routed id — pinned ids install on a store miss from the fixed first-party GHCR mapping (launcher pull-on-miss), bare names stay verify-and-load over the project cache. Every invocation — help, version, grammar rejections, and `adapter add` (over its read-only self-named seed preopen) included — runs in the specify guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`crates/transport`).

The authoritative leaf → root crate graph (with per-crate roles) lives in [AGENTS.md § Crate graph](../../AGENTS.md#the-rust-workspace-specify-cli). Headline shape: `error` is the leaf; `project` / `slice` / `change` own domain operations; `transport` owns typed routing; `launcher` owns native deployment policy; `native` / `guest` are the two providers; `mock` / `probe` are the lab rung.

There is no lint engine or `Check` substrate. Repo consistency is the mdBook links gate (`cargo make links`).

The artifact validation rule registry (`artifacts::validate`) sits on `artifacts`, which depends on none of the engine crates nor anything named lint, so an artifact rule cannot reach workflow lifecycle types. `artifacts` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry the engine layer reads, alongside `diagnostics` and `error` at the bottom. The neutral `Diagnostic` substrate lives in the `diagnostics` crate, so every check producer mints findings without depending on anything named `lint`.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/specify-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `error` re-introduces the cycle the layering was designed to avoid; do not.

**New workspace crates** are an exception, not the default.

`crates/mock` is the single test-support crate that prevents adapter test behavior from being copied across the native engine suites and the wasm example. It owns the shared mock core, canonical SDK operations-trait implementors (`adapter::Source` / `adapter::Target`), wasm32-only WIT mappings, scripted answers, and native session helpers. The lab-only `crates/probe` package is a library: the core receives catalog and model factory from its composition root; the `client` feature adds the shared cursor composition (`probe::client` — the lazily connected `DevModel` and the argv dispatch). Each repository composes it as a root example — `examples/eval/` here owns the Tokio runtime and the mock catalog binding; the matching example in the adapters repository binds the first-party catalog. None of these carry production lifecycle authority or enter the shipped guest.

## Deployments: Wasm provider and native host

The engine core (`project` / `slice` / `change` / `transport`) is deployment-neutral: it consumes model, adapter, ensure/resolve, and anchoring capabilities through provider traits and never knows whether they are satisfied by WIT imports or linked Rust implementations. Two providers exist. The **Wasm deployment** (the `guest` crate behind the root cdylib's `guest::export!()`) satisfies them over WIT imports: component adapters, the global store with digest verification, and Omnia hosting. The **native host** (`crates/native`) satisfies them over a statically compiled catalog: adapter identity is the compile-time `AdapterIdentity`, ensure is a static catalog match (never component I/O), and references serve from an owned loopback listener. Native execution makes no component, WIT, isolation, digest, or store claims — linked adapters are trusted in-process code with full process authority; untrusted or dynamically supplied adapters belong on the Wasm deployment. The native command path is single-flight (one command per provider graph). A linked operator distribution is unresolved follow-up work; until then each repository's `eval` composition example owns catalog composition.

The root `specify` package carries the Omnia deployment unit under `src/`: the guest cdylib (`src/lib.rs`, one `guest::export!()` invocation) and the shipped runtime (`src/main.rs`, one `omnia::runtime!` invocation embedding the engine bytes). The `guest` crate (`crates/guest`) owns the `workflow`-world WIT bindings, the WIT-backed `Provider`, and the `export!` macro that wires both transports — `wasi:cli/run` (`CliGuest` + `Guest::run`) and `wasi:http/incoming-handler` (`Http` + `Guest::handle` + `omnia_guest::api::http::serve`) — so downstream deployments (the wasm example in `augentic/specify-adapters`) build the identical guest from one macro invocation instead of vendoring sources. Commands live in the engine crates (`project`, `slice`, `change`) as transport-neutral `Operation<P>` implementations, each family in a `handlers` submodule beside its domain kernels (shared plumbing in `project::handler`). `crates/transport/src/command.rs` and `crates/transport/src/http.rs` own the explicit typed command and HTTP route inventories over `Invoker<P>`; the WASI and native shims only construct invokers and adapt transport output. The routing design is documented in [handler-shape.md](handler-shape.md).

## workflow domain modules

Four module trees carry the workflow contract — two in `project`, plus `spec/provenance.rs` and `evidence.rs` in `artifacts`; touching any of them requires a cross-repo `rg` sweep per [AGENTS.md §"When working in the Rust workspace"](../../AGENTS.md#when-working-in-the-rust-workspace).

- **`crates/project/src/adapter/`** — deployment-neutral `Resolver` capability (`resolve_*` plus the async `ensure_*` provisioning legs over `AdapterSelector`) plus the shipped `resolver::Component` implementation and its `ensure` kernels (one component, no manifest). Operations and kernels receive the resolver through their provider; the WASI provider delegates to component resolution, while `crates/native` provides the static catalog-match implementation. `resolver::Component` resolves a pinned identity from the global store and a bare name from the seeded project component cache only (no sibling-checkout or build-tree probe; `specify adapter add` seeds the cache). Non-identity metadata comes from the component's deterministic `metadata` export through the provider-supplied runner and is cached against the component digest. Operation prompts are compiled into each adapter's guest — the CLI never resolves or reads prompt bodies.
- **`crates/artifacts/src/spec/provenance.rs`** — parser and validator for the requirement-block provenance metadata (`ID:`, `Sources:`, `Status:`) that core synthesis emits at the top of every `spec.md` requirement. `RequirementStatus` is closed (`agreed | unknown | conflict | divergence`); the inline `[…]` tag on the requirement heading must agree with the `Status:` line. Findings aggregate so one malformed block does not mask later problems.
- **`crates/project/src/journal.rs`** — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`. Closed `Event` / `EventKind` taxonomy; kebab-case dotted wire ids (`plan.transition.approved`, `plan.amend.divergence`, `slice.transition.refined`, `slice.extract.completed`, `slice.synthesis.{conflict,divergence,unknown}`) bridge to `snake_case` Rust variants via `#[serde(rename = "…")]`. Append is atomic and is the only mutation; readers tail the file and skip blank lines.
- **`crates/artifacts/src/evidence.rs`** — the typed Evidence `Document` / `Claim` wire shapes (mirroring the WIT `evidence` / `claim` records) and their deterministic validation (`Document::validate`: kebab grammars, the per-kind claim id requirement). The typed serde parse is the load gate for every on-disk workflow artifact; validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`; surfaces that render findings (`slice validate`) emit a `DiagnosticReport` on stdout first.

## Adapter component resolution

`resolver::Component` dispatches pinned identities by routed id (the WIT `metadata` export; under the shipped deployment the host launcher resolves the component from the global single-file store `<store-root>/<name>@<version>.wasm` and installs a miss from the fixed first-party GHCR mapping — pull-on-miss with verify-on-read) and probes the project component cache for bare names and persisted component selectors (`<project-cache>/components/<name>.wasm`, seeded by `specify adapter add` or mirrored at init from an operator-supplied local file). There is no sibling-checkout or build-tree probe. Both roots derive from the carried `Locations` value (`SPECIFY_HOME`, else `~/.specify`, with `store/` and `cache/` beneath it), captured once at each composition root and threaded through `ExecutionPaths`. A binding names the axis; a component bound on the wrong axis fails at the dispatch seam — no deployed guest exports the requested `<axis>:<name>` id.

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/specify-adapters`. The carve-out discipline (leaner lint posture and minimal `[workspace.dependencies]`) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. There is no declared-tool surface. No `specify-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.specify/` is framework-managed state every CLI verb writes through (configuration under `project.yaml`, `slices/`, `archive/`, `scratch/`, the journal, the `guest.lock` marker). Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root. The boundary is enforced by the `Layout<'a>` newtype in `project` (`crates/project/src/config.rs`): path helpers are inherent methods on `Layout<'a>`, and call sites write `Layout::new(&dir).plan_path()`. Do not hard-code `.specify/registry.yaml` or sibling paths, and do not declare free path-helper functions outside `crates/project/src/config/`; any new `.specify/` path lands on `Layout`.

## Time injection

Functions that record a timestamp into a serialised artifact accept `now: jiff::Timestamp` from the operation boundary. Domain kernels do not call `Timestamp::now()`; operation call sites inject time so tests can pin it deterministically. The current carve-out — `slice_actions::*` and friends still consume an injected `now` argument — is the canonical shape to follow.

## Atomic writes

Use `yaml_write` (in `crates/artifacts/src/atomic.rs`) for any file a concurrent reader may observe mid-write: `plan.yaml`, `metadata.yaml`, and the registry. It serialises to `NamedTempFile::new_in(parent)` and `persist`-renames over the target so readers either see the prior bytes or the new bytes. Plain `fs::write` is reserved for files no other process reads concurrently with the writer (one-shot scratch output, fixtures inside a tempdir test).

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

Every deterministic operation lives in this CLI: kebab-case validation, `metadata.yaml` reads/writes, lifecycle transitions, plugin resolution (`specify source resolve` / `specify target resolve`), artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, plan/registry validation, schema validation of `plan.yaml` and per-source `Evidence`, journal event append. The plugin repo's `/spec:` skills (`/spec:init`, `/spec:plan`, `/spec:execute`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:finalize`) shell out for all of those; the execute loop is the CLI verb `specify plan execute`, wrapped by `/spec:execute`.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

The parent repo's [`AGENTS.md`](https://github.com/augentic/specify/blob/main/AGENTS.md) is the source of truth for workflow vocabulary (slice / change), skill family, plan-driven loop, and contract skills.
