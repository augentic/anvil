# Architecture

Workspace shape, crate dependency direction, the WASI carve-out, the `Layout<'a>` boundary, time injection, network hardening, and the rationale behind atomic writes. Read this before adding a new crate or shifting where state lives.

## Workspace layout

Deployment crate (`name = "specify"`) at the repo root. [`src/runtime.rs`](../../src/runtime.rs) is a single `omnia::runtime!` invocation in command mode over the cursor-bound backends — the binary carries no Specify vocabulary. Every verb runs in the specify guest ([`src/lib.rs`](../../src/lib.rs)) through the shared command grammar (`crates/transport`). Workspace member crates live under `crates/`; the dependency direction is leaf → root:

```text
error                    # leaf — thiserror + serde-saphyr only
schema                   # depends on error (embedded JSON Schemas + jsonschema plumbing; owns schema::digest — SHA-256 hex via sha2 + base16ct — and schema::diagnostics, the neutral Diagnostic substrate: report, fingerprint, validator, renderers, blocking)
artifacts                # depends on {error,schema} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; artifacts::validate artifact rule registry — NOT on workflow or anything named lint)
workflow                 # workflow layer — depends on {error,schema,artifacts,omnia-guest}; owns transport-neutral Operation implementations and workflow::agents; no wasmtime in its graph
transport                # wasm-clean transport assembly — shared typed command/HTTP routers, Args conversions, projectors, and exit contract
testkit                  # dev-only shared test support (the scripted Model mock); [dev-dependencies] only, never shipped
specify (root crate)     # Omnia deployment unit under src/: wasm32 guest lib exporting wasi:cli/run + wasi:http/incoming-handler, plus the omnia::runtime! binary — depends on no specify-* crate natively
```

The framework authoring checks run as plain cargo tests at [`tests/framework/`](../../tests/framework/); there is no lint engine or `Check` substrate.

The artifact validation rule registry (`artifacts::validate`) sits on `artifacts`, which depends on neither `workflow` nor anything named lint, so an artifact rule cannot reach workflow lifecycle types. `artifacts` is the lifecycle-free leaf carrying the artifact types, parsers, and validation registry the workflow layer reads, alongside `schema` and `error` at the bottom. The neutral `Diagnostic` substrate lives at `schema::diagnostics`, so every check producer mints findings without depending on anything named `lint`.

### Engineering standards live in the adapters

There is no standards crate: engineering-standards rules (`UNI-*` and per-adapter overlays) are authored in `augentic/specify-adapters` and ship embedded in each target adapter's component, applied by its build review prompts. The "no lifecycle authority in review" rule is structural — no engine crate parses or resolves rules, so standards prose cannot reach slice or plan transitions.

Every crate uses the shared `[workspace.package]` (`edition = "2024"`, `rust-version = "1.95"`, MIT/Apache-2.0) and the shared `[workspace.lints]` block in the root `Cargo.toml` (clippy `all`/`cargo`/`nursery`/`pedantic` warned, plus a hand-picked `restriction` subset and a tightened rust lint set — `missing_debug_implementations`, `single_use_lifetimes`, `redundant_lifetimes`).

**Hard dependency rule:** `error` is the leaf and depends on no other workspace crate. Adding a workspace dep to `error` re-introduces the cycle the layering was designed to avoid; do not.

**New workspace crates** are an exception, not the default.

The root `specify` package carries the Omnia deployment unit under `src/`: the guest lib (`src/lib.rs`, with the WIT-backed provider in `src/provider.rs`, command entry in `src/command.rs`, and HTTP entry in `src/http.rs`) and the shipped runtime (`src/runtime.rs`). The guest exports both transports explicitly from those files: `wasi:cli/run` through `command.rs` (`CliGuest` + `Guest::run`), and `wasi:http/incoming-handler` through `http.rs` (`Http` + `Guest::handle` + `omnia_guest::api::http::serve`). `lib.rs` is module wiring only — no `guest!` macro. Commands live in `crates/workflow` as transport-neutral `Operation<P>` implementations, each family in a `handlers` submodule beside its domain kernels (shared plumbing in `workflow::handler`). `crates/transport/src/command.rs` and `crates/transport/src/http.rs` own the explicit typed command and HTTP route inventories over `Invoker<P>`; the WASI and native shims only construct invokers and adapt transport output. See [rfcs/handler-routing.md](../../rfcs/handler-routing.md) for the routing design.

## workflow domain modules

Four module trees carry the workflow contract — three in `workflow`, plus `spec/provenance.rs` in `artifacts`; touching any of them requires a cross-repo `rg` sweep per [AGENTS.md §"When working in the Rust workspace"](../../AGENTS.md#when-working-in-the-rust-workspace).

- **`crates/workflow/src/adapter/`** — deployment-neutral `Resolver` capability plus the shipped `resolver::Component` implementation (one component, no manifest). Operations and kernels receive the resolver through their provider; the WASI provider delegates to component resolution, while the native harness provides its own linked-crate implementation outside the production workspace code. `resolver::Component` resolves a pinned identity from the global store and a bare name from the project component cache or sibling/in-repo development release build. Non-identity metadata comes from the component's deterministic `metadata` export through the provider-supplied runner and is cached against the component digest. Operation prompts are compiled into each adapter's guest — the CLI never resolves or reads prompt bodies.
- **`crates/artifacts/src/spec/provenance.rs`** — parser and validator for the requirement-block provenance metadata (`ID:`, `Sources:`, `Status:`) that core synthesis emits at the top of every `spec.md` requirement. `RequirementStatus` is closed (`agreed | unknown | conflict | divergence`); the inline `[…]` tag on the requirement heading must agree with the `Status:` line. Findings aggregate so one malformed block does not mask later problems.
- **`crates/workflow/src/journal.rs`** — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`. Closed `Event` / `EventKind` taxonomy; kebab-case dotted wire ids (`plan.transition.approved`, `plan.amend.divergence`, `slice.transition.refined`, `slice.extract.completed`, `slice.synthesis.{conflict,divergence,unknown}`) bridge to `snake_case` Rust variants via `#[serde(rename = "…")]`. Append is atomic and is the only mutation; readers tail the file and skip blank lines.
- **`crates/workflow/src/schema_gate.rs`** — workflow-aware validation wrappers for the on-disk workflow artifacts (`schemas/plan/plan.schema.json`, `schemas/evidence.schema.json`, `schemas/discovery/lead.schema.json`). The raw embedded schema constants and the generic `jsonschema` plumbing live in `crates/schema/` (`schema`); this module imports them and adds the workflow-shaped error aggregation (the `rule_id` strings the CLI surfaces, joined into the payload-free error `detail`). Validators return the payload-free `Error::Validation { code, detail }` so the CLI exits with code 2 (`Exit::ValidationFailed`) with the specific discriminant as the wire `error`; surfaces that render findings (`slice validate`) emit a `DiagnosticReport` on stdout first. `specify plan add` / `plan amend` / `slice validate` are the first-use hooks.

## Adapter component resolution

`resolver::Component` probes — in order — the global single-file store (pinned identities: `<store-root>/<name>@<version>.wasm`, verify-on-read), the project component cache (`<project-cache>/components/<name>.wasm`, mirrored at init from an operator-supplied local file), and the sibling/in-repo development release build (`target/wasm32-wasip2/release/<name>.wasm`). A binding names the axis; the metadata dispatch checks the component's exports before instantiating (`adapter-axis-mismatch` on the wrong axis).

## WASI carve-outs

The two adapter validators — `contract` and `vectis` — are in-guest adapter library code compiled into each adapter's published component in `augentic/specify-adapters`. The carve-out discipline (leaner lint posture, minimal `[workspace.dependencies]`, no `error` / `wasmtime` / `tokio` / `ureq` dependency) lives in that repo's workspace. Crux shell presence and launcher-icon heuristics live in the vectis adapter's in-guest core: the host performs no plan-time shell detection, so this repo carries no shell-detect crate.

The framework checks over this repo's prose are not WASI components either — they are plain cargo tests at `tests/framework/`, dev-only and outside every shipped crate.

**Host runner invariant.** The host CLI dispatches no adapter-owned tool: adapter validation, scaffold, and rendering logic lives entirely in the adapters repo as in-guest library code. There is no declared-tool surface. No `specify-*` workspace crate may import adapter-specific validation, scaffold, or rendering logic.

## Layout boundary

`.specify/` is framework-managed state every CLI verb writes through (configuration under `project.yaml`, `slices/`, `archive/`, `scratch/`, the journal, the `guest.lock` marker). Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root. The boundary is enforced by the `Layout<'a>` newtype in `workflow` (`crates/workflow/src/config.rs`): path helpers are inherent methods on `Layout<'a>`, and call sites write `Layout::new(&dir).plan_path()`. Do not hard-code `.specify/registry.yaml` or sibling paths, and do not declare free path-helper functions outside `crates/workflow/src/config/`; any new `.specify/` path lands on `Layout`.

## Time injection

Functions that record a timestamp into a serialised artifact accept `now: jiff::Timestamp` from the operation boundary. Domain kernels do not call `Timestamp::now()`; operation call sites inject time so tests can pin it deterministically. The current carve-out — `slice_actions::*` and friends still consume an injected `now` argument — is the canonical shape to follow.

## ureq fetch hardening

Any `ureq` HTTP path in this workspace (today: the channel-aware self-update probe in `crates/workflow/src/upgrade.rs`) runs with explicit per-call timeouts, a response-size cap checked on both the `Content-Length` header and the streamed body, and streams large bodies to a tempfile before persisting. Any new HTTP path must adopt the same shape (timeouts + size cap + stream-to-tempfile); do not buffer arbitrary remote bodies into memory.

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
2. Run `cargo make vet-regenerate` to refresh the supply-chain audits, review the `supply-chain/` diff, then commit it.
3. Check `deny.toml` allows the dependency's licence. The current allowlist is in `deny.toml`; add a new SPDX id only after confirming compatibility with MIT-OR-Apache-2.0.

`clippy::multiple_crate_versions` is silenced workspace-wide (`Cargo.toml`'s `[workspace.lints.clippy]`); duplicate transitive versions are audited by hand via `cargo tree --duplicates` on each `cargo update`, not gated through a ratchet.

## Skill / CLI responsibility split

Every deterministic operation lives in this CLI: kebab-case validation, `metadata.yaml` reads/writes, lifecycle transitions, plugin resolution (`specify source resolve` / `specify target resolve`), artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive moves, plan/registry validation, schema validation of `plan.yaml` and per-source `Evidence`, journal event append. The plugin repo's `/spec:` skills (`/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:finalize`, `/spec:init`, `/spec:drop`) shell out for all of those; the execute loop is the CLI verb `specify plan execute`.

The corollary: when a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb here and have the skill call it. The wrong fix is to make the skill smarter.

The parent repo's [`AGENTS.md`](https://github.com/augentic/specify/blob/main/AGENTS.md) is the source of truth for workflow vocabulary (slice / change), skill family, plan-driven loop, and contract skills.
