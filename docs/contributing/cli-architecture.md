# CLI Architecture

The `specify` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a resolver-backed dynamic deployment (RFC-70 Stage 3) expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/omnia.rs`) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!(env!("SPECIFY_WASM"))`, resolved by the root `build.rs`) and registered at boot as the sole `wasi:cli/run` exporter; the `launcher` crate contributes the mount and resolver expressions the macro's `mounts:` / `resolver:` keys evaluate — one per-process anchoring (project-root walk, `--project-dir` override, `Locations::from_env` captured once, writable mount directories created pre-run, the optional read-only `adapter add` seed preopen) and the fail-closed adapters-only `GuestResolver`. Adapters are admitted lazily by exact routed id on first dispatch — pinned ids resolve the global store with pull-on-miss install (the launcher anonymously pulls the published Wasm OCI artifact from the compiled first-party GHCR mapping, verifies it, and writes the store entry plus digest sidecar), unpinned ids stay verify-and-load over the seeded project cache; the launcher is the deployment's only downloader. There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the specify (engine) guest through the shared typed command router — help and version displays, grammar rejections (the shared clap grammar compiles into the engine, so its renderings are the product's by construction), and `adapter add` (the operator's component directory reaches the guest as a read-only preopen named by its own absolute host path) included; envelopes and exit codes pass through verbatim. Removed provisioning and bootstrap surfaces are not advertised as deferred commands.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Adapter identities resolve by routed id: the host-owned global store for pinned ids (`<axis>:<name>@<version>`, installed on a miss), the seeded project component cache for unpinned ones (`<axis>:<name>`); distinct pins of one adapter coexist because their routed ids differ. `SPECIFY_HOME` is the single relocation override — store and cache derive together beneath it (the Cargo model) — captured once per invocation into a carried `Locations` value; nothing below the composition root reads the environment.

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md § Crate graph](../../AGENTS.md#the-rust-workspace-specify-cli). The headline shape: `error` is the leaf; the three engine crates own the domain and every command operation (`project` the foundation + init, `slice` the refine-build-merge loop, `change` the plan loop; `Operation<P>` impls in each crate's `<domain>::handlers`, shared plumbing in `project::handler`); `transport` owns the typed command/HTTP route inventories, clap args, explicit conversions, projectors, and exit contract; `launcher` owns the native-only deployment policy (per-process anchoring, the macro-facing mount and resolver expressions, and the fail-closed adapters-only `GuestResolver`); the root binary is one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (Layout, WASI carve-outs, atomic writes) live in [architecture.md](../standards/architecture.md).

Adapter deterministic helpers sit co-located beside their adapter prose in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) as in-guest library code compiled into each adapter's published component.

Vectis does not link an adapter-specific crate into the root `specify` binary. Its deterministic helpers (UI artifact validation, render-only scaffolding) are in-guest library code inside its published component; platform SDK, Cargo, Xcode, Gradle, and registry behavior lives in the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/merge.md) prompts (which carry the Vectis writer / reviewer / template-updater behavior).

## Dispatch pattern

The binary entry point is thin:

```text
src/omnia.rs  →  omnia::runtime! (command mode; embedded engine bytes, launcher mount/resolver expressions)
              →  specify guest  →  typed command router  →  adapter dispatches resolve lazily by routed id
```

The launcher projects argv through `transport::command::selectors::seed_request`, which parses argv against the *same* assembled clap grammar the guest router executes (a grammar-only provider that never dispatches). It projects exactly one pre-boot fact — the `adapter add` seed request, whose `--project-dir` anchors the project mount and whose component path earns the seed preopen; everything else, displays and rejections included, renders in the guest. No adapter selectors are folded into any closure, because no closure exists.

The full operator grammar — unsupported provisioning commands included — is assembled in `crates/transport/src/command.rs` from concrete leaf `Args` and transport-neutral workflow `Operation` types. Explicit `TryFrom<Args>` implementations make conversion drift a compile-time concern; `omnia_guest::api::command` owns clap behavior, completions, inventory, and invocation. `crates/transport/src/http.rs` assembles the matching typed HTTP routes. WASI and native shims only construct providers/invokers and adapt transport output. The operation contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

The guest exports both transports explicitly from `src/lib.rs` — no `guest!` macro. Each shim constructs an `Invoker`; the route inventories remain in `crates/transport`.

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure body is `ErrorBody`. There is no top-level envelope-version stamp.
- **Kebab-case error discriminants** — `adapter-not-installed`, `invalid-project`, `io` (never `missing_prerequisites`); skills and tests grep on the `error` / `code` fields, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `SPECIFY_FORMAT=json` is the environment equivalent.

## Exit codes

The exit-code contract is part of the public interface for operators and skill wrappers; `Exit::from(&Error)` in `crates/transport/src/command/output.rs` is the single source of truth:

| Code | Constant                 | Meaning                                                                                                                              |
| ---- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `0`  | `EXIT_SUCCESS`           | Operation completed successfully                                                                                                     |
| `1`  | `EXIT_GENERIC_FAILURE`   | I/O error, parse error, or any unclassified failure                                                                                  |
| `2`  | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`, or clap usage errors                                                    |
| `3`  | `EXIT_VERSION_TOO_OLD`   | Binary version is below the `specify` floor in `.specify/project.yaml`, or below an adapter's declared `specify` compatibility floor |

Guest commands inherit the same contract: `omnia_guest::api::command` projects parser, conversion, and operation outcomes into a buffered command response; the WASI seam forwards its exit and the binary passes it through verbatim.

## Error handling

Most commands use `error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, permission failures, runtime failures, and more.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, error::Error>`
2. Return a typed body implementing `Serialize + Render`
3. Let the command or HTTP projector render success or apply the shared error contract

## Public Rust API

The root `specify` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `project::plan::Plan`, `project::config::ProjectConfig`, or `error::Error`.
