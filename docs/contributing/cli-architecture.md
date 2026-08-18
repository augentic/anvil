# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a resolver-backed dynamic deployment expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/main.rs`) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.bin` — the root `build.rs` child-builds the wasm32 engine, then in release builds ahead-of-time compiles it to a serialized wasmtime artifact, so startup deserializes rather than JIT-compiles the engine; debug builds embed the raw component and JIT at startup) and registered at boot as the sole `wasi:cli/run` exporter; the `launcher` crate contributes the mount and resolver expressions the macro's `mounts:` / `resolver:` keys evaluate — one per-process anchoring (project-root walk, `Locations::from_env` captured once, writable mount directories created pre-run) and the fail-closed adapters-only `GuestResolver`. Adapters are admitted lazily by exact routed id on first dispatch — a seeded project-cache entry answers first for pinned and bare ids alike (the co-dev seed always wins), then the embedded first-party registry (empty until Phase 4), then a verified global-store entry for pins (verify-on-read against the digest sidecar) — with each settled identity logged to stderr. There is no download path (ADR-0002 deletions): nothing local is the typed `adapter-not-found`. There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. The only argv the guest never sees is the reserved host log flags — Omnia's direct-command entry peels `--debug` / `--quiet` anywhere in argv into the host log preset (bare defaults to muted INFO progress, `--quiet` is off, `--debug` adds backend debug tracing; the flags win over any ambient `RUST_LOG`).

The deployment also installs the adapter MCP route through the macro's `http_paths:` key: `launcher::mcp_route` maps `/mcp/<axis>/<name>[@<version>]` back onto the routed adapter id, so the loopback URL a judgment dispatch grants reaches the adapter component's own `wasi:http` handler and its embedded references shelf, faulting the guest in through the same fail-closed resolver when needed. The port is coordinated per invocation through the macro's `http_listener:` key: `launcher::http_listener` pre-binds this invocation's listener (an operator-set `HTTP_ADDR` must bind, else an ephemeral loopback port), the runtime's HTTP trigger adopts it, and the launcher injects the fully-formed shelf base as `MCP_URL_BASE`. A path outside the grammar is declined and a claimed identity nothing supplies stays an ordinary 404, while a genuine fault on a claimed route is an error-logged 500, never a mis-routed dispatch.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. `EMERY_HOME` is the single relocation override — store and cache derive together beneath it (the Cargo model) — captured once per invocation into a carried `Locations` value; nothing below the composition root reads the environment.

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). The headline shape: `error` is the leaf; `engine` owns the domain and the `init` / `specify` operations (shared plumbing in `engine::handler`, resolution in `engine::resolve`); `transport` owns the typed command/HTTP route inventories, clap args, explicit conversions, projectors, and exit contract; `launcher` owns the native-only deployment policy; the root binary is one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the `.emery/` layout boundary, WASI carve-outs, atomic writes) live in [architecture.md](../standards/architecture.md).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  omnia::runtime! (command mode; embedded engine bytes, launcher mount/resolver expressions)
              →  emery guest  →  typed command router  →  adapter dispatches resolve lazily by routed id
```

The launcher projects nothing out of argv: with the registry refresh surface deleted (ADR-0002), no pre-boot fact depends on the parsed grammar — anchoring walks the working directory, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/transport/src/command.rs` from concrete leaf `Args` and transport-neutral `Operation` types. Explicit `TryFrom<Args>` implementations make conversion drift a compile-time concern; `omnia_guest::api::command` owns clap behavior, completions, inventory, and invocation. `crates/transport/src/http.rs` is the HTTP refusal surface — the guest serves only MCP reference shelves over HTTP (C3). The WASI shim only constructs the provider/invoker and adapts transport output. The operation contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure body is `ErrorBody`. There is no top-level envelope-version stamp.
- **Kebab-case error discriminants** — `adapter-not-found`, `invalid-project`, `io` (never `missing_prerequisites`); skills and tests grep on the `error` / `code` fields, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `EMERY_FORMAT=json` is the environment equivalent.

## Exit codes

The exit-code contract is part of the public interface for operators and skill wrappers; `Exit::from(&Error)` in `crates/transport/src/command/output.rs` is the single source of truth:

| Code | Constant                 | Meaning                                                                                                                        |
| ---- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| `0`  | `EXIT_SUCCESS`           | Operation completed successfully                                                                                               |
| `1`  | `EXIT_GENERIC_FAILURE`   | I/O error, parse error, or any unclassified failure                                                                            |
| `2`  | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`, or clap usage errors                                              |
| `3`  | `EXIT_VERSION_TOO_OLD`   | Binary version is below the `emery` floor in `.emery/project.yaml`, or below an adapter's declared `emery` compatibility floor |

Guest commands inherit the same contract: `omnia_guest::api::command` projects parser, conversion, and operation outcomes into a buffered command response; the WASI seam forwards its exit and the binary passes it through verbatim.

## Error handling

Most commands use `error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, permission failures, runtime failures, and more.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, error::Error>`
2. Return a typed body implementing `Serialize + Render`
3. Let the command or HTTP projector render success or apply the shared error contract

## Public Rust API

The root `emery` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `engine::project::Project` or `error::Error`.
