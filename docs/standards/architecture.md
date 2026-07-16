# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `Layout<'a>` boundary, time injection, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "specify"`) at the repo root. [`src/runtime.rs`](../../src/runtime.rs) is a single `omnia::runtime!` invocation in command mode over the cursor-bound backends — the binary carries no Specify vocabulary. Every verb runs in the specify guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`crates/transport`). Workspace member crates live under `crates/`; the dependency direction is leaf → root:

```text
error                    # leaf — thiserror + serde-saphyr only
diagnostics              # dependency-light leaf (the neutral Diagnostic substrate: report, fingerprint, blocking — plus diagnostics::digest, SHA-256 hex via sha2 + base16ct, and diagnostics::cache)
artifacts                # depends on {error,diagnostics} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; artifacts::validate artifact rule registry — NOT on the workflow crates or anything named lint)
adapter                  # the adapter SDK (leaf over omnia-guest, no workspace-crate deps) — per-axis operations traits, WIT package + wasm export macros, seam DTOs, embedded prose registry
project                  # foundation — depends on {error,diagnostics,artifacts,omnia-guest}; init (+ agents), adapter resolution, config/Layout, journal, registry, the plan and slice data models, seam capability traits, the judgment kernel, shared handler plumbing
slice                    # the slice loop — depends on project; refine/build/merge orchestration, synthesis, validation, the delta-merge engine, the specify slice operations, and its own prompts/ corpus (synthesize.md + synthesis/*)
change                   # the change loop — depends on {project,slice}; plan author/execute orchestration, the specify plan operations, and its own prompts/ corpus (propose.md)
transport                # wasm-clean transport assembly — shared typed command/HTTP routers, Args conversions, projectors, and exit contract; depends on {project,slice,change}
prose                    # build-dependency — embed-time prompt-corpus walk + link check, generating each crate's DOCS table
testkit                  # dev-only test-support — fixture adapter core + its SDK operations-trait implementors, unified Provider, scripted answers
checks                   # dev-only repo invariants — boundaries, links, authoring (plain cargo tests)
harness                  # native eval-harness core — linked-adapter catalog over the SDK traits, seam provider, model bridge, trial/scenario/command/HTTP plumbing; no concrete adapter deps
eval                     # live-model prompt-evaluation wrapper — native-only bin binding the testkit fixture catalog and engine trial profile over harness
specify (root crate)     # Omnia deployment unit under src/: wasm32 guest lib exporting wasi:cli/run + wasi:http/incoming-handler, plus the omnia::runtime! binary — depends on no specify-* crate natively; carries the examples/change cargo example (the fixture adapter guest)
```

The repo checks run as plain cargo tests in the lightweight [`crates/checks`](../../crates/checks/) package; there is no lint engine or `Check` substrate.

The artifact validation rule registry (`artifacts::validate`) sits on `artifacts`, which depends on none of the workflow crates nor anything named lint, so an artifact rule cannot reach workflow lifecycle types. `artifacts` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry the workflow layer reads, alongside `diagnostics` and `error` at the bottom. The neutral `Diagnostic` substrate lives in the `diagnostics` crate, so every check producer mints findings without depending on anything named `lint`.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/specify-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `error` re-introduces the cycle the layering was designed to avoid; do not.

**New workspace crates** are an exception, not the default.

`crates/testkit` is the single test-support crate that prevents adapter test behavior from being copied across the native workflow suites and the change example: `testkit::adapter` is the shared fixture core, `testkit::fixture` is its canonical SDK operations-trait implementors (`adapter::Source` / `adapter::Target`), `testkit::wit` (wasm32-only) is the `adapter`-world export bindings plus seam mappings, and `examples/change/guest.rs` is the thin WIT component shim over both. The native live-model engine trial (`plan → execute → finalize`) is `crates/eval` — a declarative binding of the fixture catalog and the engine trial profile over the shared `crates/harness` runtime (the same runtime `specify-dev` in `augentic/specify-adapters` binds with the first-party adapters). None of these carry production lifecycle authority and none are linked into the shipped guest. Generic model doubles (the FIFO `Scripted` script), temporary deployment mechanics, and HTTP driving remain in upstream `omnia-testkit`; this repository's change example uses Omnia's public `runtime!` and `omnia.toml` surfaces directly.

The root `specify` package carries the Omnia deployment unit under `src/`: the guest lib (`src/lib.rs`, with the WIT-backed provider in `src/provider.rs`) and the shipped runtime (`src/runtime.rs`). The guest exports both transports explicitly from `lib.rs` — `wasi:cli/run` (`CliGuest` + `Guest::run`) and `wasi:http/incoming-handler` (`Http` + `Guest::handle` + `omnia_guest::api::http::serve`) — no `guest!` macro. Commands live in the workflow crates (`project`, `slice`, `change`) as transport-neutral `Operation<P>` implementations, each family in a `handlers` submodule beside its domain kernels (shared plumbing in `project::handler`). `crates/transport/src/command.rs` and `crates/transport/src/http.rs` own the explicit typed command and HTTP route inventories over `Invoker<P>`; the WASI and native shims only construct invokers and adapt transport output. The routing design is documented in [handler-shape.md](handler-shape.md).

## workflow domain modules

Four module trees carry the workflow contract — two in `project`, plus `spec/provenance.rs` and `evidence.rs` in `artifacts`; touching any of them requires a cross-repo `rg` sweep per [AGENTS.md §"When working in the Rust workspace"](../../AGENTS.md#when-working-in-the-rust-workspace).

- **`crates/project/src/adapter/`** — deployment-neutral `Resolver` capability plus the shipped `resolver::Component` implementation (one component, no manifest). Operations and kernels receive the resolver through their provider; the WASI provider delegates to component resolution, while the native harness provides its own linked-crate implementation outside the production workspace code. `resolver::Component` resolves a pinned identity from the global store and a bare name from the project component cache or the project's own in-repo development release build (no sibling-checkout probe). Non-identity metadata comes from the component's deterministic `metadata` export through the provider-supplied runner and is cached against the component digest. Operation prompts are compiled into each adapter's guest — the CLI never resolves or reads prompt bodies.
- **`crates/artifacts/src/spec/provenance.rs`** — parser and validator for the requirement-block provenance metadata (`ID:`, `Sources:`, `Status:`) that core synthesis emits at the top of every `spec.md` requirement. `RequirementStatus` is closed (`agreed | unknown | conflict | divergence`); the inline `[…]` tag on the requirement heading must agree with the `Status:` line. Findings aggregate so one malformed block does not mask later problems.
- **`crates/project/src/journal.rs`** — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`. Closed `Event` / `EventKind` taxonomy; kebab-case dotted wire ids (`plan.transition.approved`, `plan.amend.divergence`, `slice.transition.refined`, `slice.extract.completed`, `slice.synthesis.{conflict,divergence,unknown}`) bridge to `snake_case` Rust variants via `#[serde(rename = "…")]`. Append is atomic and is the only mutation; readers tail the file and skip blank lines.
- **`crates/artifacts/src/evidence.rs`** — the typed Evidence `Document` / `Claim` wire shapes (mirroring the WIT `evidence` / `claim` records) and their deterministic validation (`Document::validate`: kebab grammars, the per-kind claim id requirement). The typed serde parse is the load gate for every on-disk workflow artifact; validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`; surfaces that render findings (`slice validate`) emit a `DiagnosticReport` on stdout first.

## Adapter component resolution

`resolver::Component` probes — in order — the global single-file store (pinned identities: `<store-root>/<name>@<version>.wasm`, verify-on-read), the project component cache (`<project-cache>/components/<name>.wasm`, mirrored at init from an operator-supplied local file), and the project's own development release build (`target/wasm32-wasip2/release/<name>.wasm`). There is no sibling-checkout probe. A binding names the axis; the metadata dispatch checks the component's exports before instantiating (`adapter-axis-mismatch` on the wrong axis).

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/specify-adapters`. The carve-out discipline (leaner lint posture and minimal `[workspace.dependencies]`) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

The repo checks are not WASI components either — they are plain cargo tests in `crates/checks`, dev-only and outside every shipped crate.

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

Every deterministic operation lives in this CLI: kebab-case validation, `metadata.yaml` reads/writes, lifecycle transitions, plugin resolution (`specify source resolve` / `specify target resolve`), artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, plan/registry validation, schema validation of `plan.yaml` and per-source `Evidence`, journal event append. The plugin repo's `/spec:` skills (`/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:finalize`, `/spec:init`, `/spec:drop`) shell out for all of those; the execute loop is the CLI verb `specify plan execute`.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

The parent repo's [`AGENTS.md`](https://github.com/augentic/specify/blob/main/AGENTS.md) is the source of truth for workflow vocabulary (slice / change), skill family, plan-driven loop, and contract skills.
