# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a static deployment expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/main.rs`) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm` — the root `build.rs` child-builds the wasm32 engine, then in release builds ahead-of-time compiles it to a serialized wasmtime artifact, so startup deserializes rather than JIT-compiles the engine; debug builds embed the raw component and JIT at startup) and routed as the explicit `command_guest`. Deployment policy is CWD-rooted and inline in the invocation: the invocation directory mounts as the guest's `.` (no ancestor walk), and the storage hosts bind engine state to the durable omnia-filesystem store (default `.omnia/storage`). Adapter guests, when present, are declared in that same invocation — a built `.wasm` path (or `include_bytes!`) plus a routed id — the way the journey host declares its mock `source` in [`examples/runtime.rs`](../../examples/runtime.rs). The shipped `src/main.rs` embeds the engine only. Dynamic admission (the `GuestResolver`, cache-seed and verified-store legs) is deferred until dynamic loading returns; there is no download path (ADR-0002 deletions). There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. The only argv the guest never sees is the reserved host log flags — Omnia's direct-command entry peels `--debug` / `--quiet` anywhere in argv into the host log preset (bare defaults to muted INFO progress, `--quiet` is off, `--debug` adds backend debug tracing; the flags win over any ambient `RUST_LOG`).

A runtime that declares adapter guests also installs their MCP routes through the macro's `routes:` key: `/mcp/source/<name>` maps to the adapter guest, so the loopback URL a judgment dispatch grants reaches the adapter component's own `wasi:http` handler and its embedded references shelf. The adapter SDK derives grant URLs from the HTTP trigger's guest-visible `HTTP_ADDR`. A path outside the grammar is declined and stays an ordinary 404, while a genuine fault on a claimed route is an error-logged 500, never a mis-routed dispatch.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Kernels never read the environment: paths are fixed constants relative to the named preopens (`ExecutionPaths::deployed()` — the same strings resolve against the wasm32 preopen table and the native invocation directory).

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). The headline shape: `error` is the leaf; `engine` owns the domain and the `specify` / `show` operations (shared plumbing in `emery_engine::handler`, resolution in `emery_engine::resolve`); `transport` owns the typed command/HTTP route inventories, clap args, explicit conversions, projectors, and exit contract; the root package's `src/main.rs` owns the native deployment policy inline and its wasm32 lib declares the bare model provider (superseding ADR-0013's WIT-backed `Provider` — paths and adapter dispatch are structural, not provider capabilities); the root binary is one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the `.emery/` layout boundary, WASI carve-outs) live in [architecture.md](../standards/architecture.md).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  omnia::runtime! (command mode; embedded engine bytes, static guests and mounts)
              →  emery guest  →  typed command router  →  adapter dispatches route by routed id
```

The deployment projects nothing out of argv: no pre-boot fact depends on the parsed grammar — the invocation directory is the project root, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/transport/src/command.rs` from concrete leaf `Args` and transport-neutral `Operation` types. Explicit `TryFrom<Args>` implementations make conversion drift a compile-time concern; `omnia_guest::api::command` owns clap behavior, completions, inventory, invocation, and the WASI last mile (`execute_wasi`: argv, telemetry init/flush, output channels, exact exit). `crates/transport/src/http.rs` is the HTTP refusal surface — the guest serves only MCP reference shelves over HTTP (C3). The WASI shim only constructs the provider and hands the assembled router to that adapter. The operation contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

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
| `3`  | `EXIT_VERSION_TOO_OLD`   | Binary version is below an adapter's declared `emery` compatibility floor                                                      |

Guest commands inherit the same contract: `omnia_guest::api::command` projects parser, conversion, and operation outcomes into a buffered command response; the WASI seam forwards its exit and the binary passes it through verbatim.

## Error handling

Most commands use `emery_error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, permission failures, runtime failures, and more.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, emery_error::Error>`
2. Return a typed body implementing `Serialize + Render`
3. Let the command or HTTP projector render success or apply the shared error contract

## Public Rust API

The root `emery` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `emery_engine::project::Project` or `emery_error::Error`.
