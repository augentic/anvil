# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `.emery/` layout boundary, and time injection. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "emery"`) at the repo root. The native arm of [`src/lib.rs`](../../src/lib.rs) is one `omnia::runtime!` command-mode invocation over the cursor-bound backends — declared in the library so the generated `main`, `manifest`, and `Hooks` are one deployment the binary ([`src/main.rs`](../../src/main.rs), `emery::main()`) runs and the component rung overlays: the engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm`, produced by the root `build.rs` — a child wasm32 build into an isolated target directory, then in release builds an ahead-of-time wasmtime serialize so startup deserializes instead of JIT-compiling the engine; debug builds embed the raw component and JIT at startup; there is no placeholder fallback), and `program: "emery"` forwards raw argv to the explicit `command_guest`. Deployment policy is static and CWD-rooted, expressed inline in the invocation: the invocation directory mounts read-only as the guest's `.` — nothing writes the working tree, the C3 least-authority posture (no ancestor walk — a verb run below the project root fails typed), the storage hosts bind the generation store to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), and the `plugins:` block declares the source seam (`emery_source::SOURCE_INTERFACE`, `emery:adapter/source@0.1.0` — the WIT owner pins the string once) plus the compiled-in acquisition policy as a declarative `locations:` list (the `.` path root — preopen-aligned reads, fresh on every load, never cached — then the `omnia.host` registry endpoint, likewise read fresh: the runtime declares no project cache). The shipped runtime embeds the engine only: a local `.wasm` adapter loads at run time through the `omnia:plugins/loader` capability (the journey host in [`examples/runtime.rs`](../../examples/runtime.rs) stays path-only and loads its built mock component the same way), an exact package reference fetches from its registry, and bare names still dispatch only statically declared guests. Every invocation — help, version, and grammar rejections included — runs in the emery guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`emery_engine::cli`).

The authoritative leaf → root crate graph (with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). Headline shape: `prose` (the embedded-corpus registry) and `source` (the `emery:adapter/source` contract, both sides) are the leaves; `adapter` is the guest-only SDK over them; `engine` owns the domain operations (`specify`, `show`, adapter resolution) and the typed command routing (`emery_engine::cli`) and returns `omnia_guest::Error` from those operations; the root package's `src/lib.rs` owns native deployment policy inline (its native arm) and is the one provider on wasm32 (the component contract is the only production boundary). The journey's mock adapter is the root-package `adapter` example (`examples/adapter/lib.rs` plus its embedded `prose/`), the same `SourceAdapter` + `source!` anatomy as a first-party adapter. The `runtime` example stays on the root package because it embeds `$OUT_DIR/emery.cwasm`.

There is no lint engine or `Check` substrate. Repo consistency is the mdBook links gate (`make links`).

Evidence claim types live in `source` (the WIT-shaped DTOs, re-exported by the adapter SDK); the fail-closed `spec.md` parser lives in `engine`. Neither depends on anything named lint.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/emery-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `prose` and `source` are the leaves and depend on no other workspace crate; `adapter` depends on both and nothing else; no production crate depends on `adapter` (the root package links it as a dev-dependency for `examples/adapter` only). The engine reaches the contract and the corpus registry through the leaves, never through the SDK — before this split the engine guest linked the whole judgment SDK to reach the wire types. Adding a workspace dep to a leaf, or an `adapter` dep to `engine` or the root's production arms, re-introduces that coupling; do not.

**New workspace crates** are an exception, not the default. Capability doubles are not a local crate: the native suites take them from omnia's `omnia-test` (`guest::{Scripted, Memory, Namespaced}`, `delegate!`). The `component` package under `examples/component/` is the one unpublished exception, and it is a workspace member rather than root test code only because it needs a build script of its own: the component rung's fixture build (`build.rs` drives `omnia_test::build::Components` over `--example adapter` for `wasm32-wasip2` — a fixture the root build script, which runs for every build of the shipped binary, must not carry), the `run` overlay of the shipped deployment (`omnia_test::host::Deployment::from(emery::manifest())` driven through `emery::Hooks`), and the suite itself in its `tests/` — a test-only package, never a dependency of production code, the shape the adapters repo uses for `examples/conformance`.

The `adapter` example (`examples/adapter/`) is the one mock source adapter: a `emery_adapter::SourceAdapter` implementor whose extract calls the host `Model` over its fixture tree (`examples/docs/`). The `runtime` example hosts it with the Cursor backend and a path-only plugin `locations:` list; the component rung hosts it under scripted backends. It carries no production lifecycle authority and never enters the shipped guest. Do not add another mock adapter — extend this example.

## Deployment: the Wasm guest

The guest's provider is a bare unit at the composition root (`src/lib.rs`) carrying the engine's capabilities: the model (`omnia_guest::Model`, WASI-backed defaults), the `Source` capability (`emery_source::Source`), the storage pair (`omnia_guest::StateStore` / `BlobStore`, since design/portable-storage.md steps 1–2), whose wasm32 default bodies ride the `wasi:keyvalue` / `wasi:blobstore` imports, and the plugin loader (`omnia_guest::Plugins`, whose wasm32 default rides the `omnia:plugins/loader` import); the runtime binds those hosts to the durable `omnia-filesystem` store (compiled-in root `.omnia/storage`; the native arm of `src/lib.rs`), so the backing is deployment policy, never engine code — alternative bindings, project-id-keyed multi-project hosts included, are documented in [deployment profiles](../reference/deployment-profiles.md). Everything else is structural: paths are fixed constants relative to named preopens (the `.` project mount; `emery_engine::handler::preopen_path` normalizes operator paths inside it), and adapter dispatch — `extract` and `metadata` — rides the `emery:adapter/source` WIT imports directly from the engine's cfg-gated dispatch functions (typed refusals on native, where no WIT import exists). The native provider was deleted at the Phase 3 spine cut; the product arc's native coverage is the root scenario rung (`tests/specify.rs`), while pure kernels test over scripted capabilities.

The root `emery` package carries the Omnia deployment unit under `src/`: the guest cdylib (the wasm32 arm of `src/lib.rs`, the `wasi:cli/run` exporter plus the bare model provider — there is no HTTP export; C3 is satisfied by binding no listener at all) and the shipped runtime (the native arm of `src/lib.rs`, one `omnia::runtime!` invocation embedding the engine bytes, run by `src/main.rs`). Commands live in `engine` as `Handler<P>` implementations on each verb's input type (shared plumbing in `emery_engine::handler`). `emery_engine::cli` owns the clap grammar, route inventory, and format projection; `cli::router` binds the caller's provider into a `Client` and returns the executable `Cli`. The WASI shim (`omnia_guest::command!(dispatch)`) constructs the provider, runs that grammar, and hands the exit status to `omnia_guest::api::command::execute_wasi`. The routing design is documented in [handler-shape.md](handler-shape.md).

## Domain modules of note

- **`crates/engine/src/resolve.rs`** — source-adapter resolution over the typed `AdapterSelector`: selector parsing, the `omnia:plugins/loader` load request for local components and registry packages (over the provider's `Plugins` capability, threading the binding's digest pin and registry override and returning the resolved digest), and the adapter `emery`-floor gate, dispatched directly over the provider's `Source` capability (the WIT `metadata` import on wasm32, a scripted mock natively).
- **`crates/engine/src/spec.rs`** — the fail-closed `spec.md` AST (`### Requirement:` blocks, `ID` / `Sources` / `Status`, heading tags). Synthesis refuses a model answer that does not parse; the re-mine diff uses the same parser for section subjects.

The lenient v1 module trees (provenance, the task/decision/leads validators) were deleted at the Phase 3 spine cut and are documented at tag `v1`.

## Adapter component resolution

Resolution dispatches identities by routed id (the WIT `metadata` export). A local component selector or an exact package reference loads through the deployment's `omnia:plugins/loader` capability once per adapter identity on every `specify` that names it: the host's acquirer reads a component file fresh (nothing is mirrored, so a deleted file refuses typed on the next run) or fetches a package from the binding's `registry` override or the compiled-in default endpoint, verifies the binding's optional sha256 pin before validation, validates the component against the declared source seam, and registers it — a component under its `source:<name>` routed id, a package under the package reference itself; the resolved digest rides the success envelope so an unpinned first run can be committed as the pin (trust-on-first-use). Bare selectors still dispatch only guests declared in the runtime invocation; anything else fails at dispatch. There is no sibling-checkout or build-tree probe. Storage locations are fixed key and container-name formulas — the same names resolve against whatever backing the deployment binds. A binding names the axis; a component bound on the wrong axis fails at dispatch — no deployed guest exports the requested `<axis>:<name>` id.

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/emery-adapters`. The carve-out discipline (leaner lint posture and minimal `[workspace.dependencies]`) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. There is no declared-tool surface. No `emery-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.emery/` is framework-managed state every CLI verb writes through, and `emery_engine::home::Home` is its one owner: the spec output home (generation documents plus the `current` pointer) over the deployment's storage capabilities. Do not hard-code `.emery/` paths elsewhere; a new `.emery/` location lands on that owner.

## Time injection

Functions that record a timestamp into a serialised artifact accept `now: jiff::Timestamp` from the operation boundary. Domain kernels do not call `Timestamp::now()`; operation call sites inject time so tests can pin it deterministically.

## Toolchain

Rust stable per `rust-toolchain.toml` (channel `stable`, components `clippy`, `rust-src`, `rustfmt`). WASM targets pre-installed via `targets = ["aarch64-apple-darwin", "wasm32-wasip2", "x86_64-apple-darwin"]`.

`rustfmt.toml` uses unstable nightly features (`unstable_features = true`, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`). Format with nightly:

```bash
cargo +nightly fmt --all
```

`make fmt` does this for you.

## Supply chain

`cargo-vet` and `cargo-deny` gate `make ci`; `cargo-audit`, `cargo-outdated`, and `cargo-udeps` are advisory tasks run on demand (`make audit` / `outdated` / `deps`). The vet task is check-only (`cargo vet --locked`) — regeneration is deliberately not part of the gate, since regenerating exemptions before checking would auto-exempt anything unaudited. When a new dependency lands:

1. Add it to `[workspace.dependencies]` in the root `Cargo.toml` with a major-version pin (e.g. `serde = { version = "1", features = ["derive"] }`). Per-crate `Cargo.toml` references it as `serde.workspace = true`.
2. Run `cargo vet regenerate imports`, `cargo vet regenerate exemptions`, and `cargo vet regenerate unpublished`; review the `supply-chain/` diff, then commit it.
3. Check `deny.toml` allows the dependency's licence. The current allowlist is in `deny.toml`; add a new SPDX id only after confirming compatibility with MIT-OR-Apache-2.0.

`clippy::multiple_crate_versions` is silenced workspace-wide (`Cargo.toml`'s `[workspace.lints.clippy]`); duplicate transitive versions are audited by hand via `cargo tree --duplicates` on each `cargo update`, not gated through a ratchet.

## Skill / CLI responsibility split

Every deterministic operation lives in this CLI: kebab-case validation, adapter resolution and loading, and schema validation. The surviving `/emery:specify` skill shells out for all of those.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

[`AGENTS.md`](../../AGENTS.md) is the source of truth for vocabulary and the crate map.
