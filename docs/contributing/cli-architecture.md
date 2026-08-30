# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a static deployment expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/main.rs`) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm` — the root `build.rs` child-builds the wasm32 engine, then in release builds ahead-of-time compiles it to a serialized wasmtime artifact, so startup deserializes rather than JIT-compiles the engine; debug builds embed the raw component and JIT at startup) and routed as the explicit `command_guest`. Deployment policy is CWD-rooted and inline in the invocation: the invocation directory mounts as the guest's `.` (no ancestor walk), the storage hosts bind engine state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), and the `plugins:` block declares the source seam (`emery:adapter/source@0.1.0`) with the compiled-in acquirer composition (`MountAcquire`, then the `omnia.host` registry acquirer over the project CAS at `.omnia/cache/wasm-pkg`), so a project-relative local `.wasm` adapter loads dynamically at run time — read fresh from the mount on every load — and an exact package reference fetches from its registry, both verified against the binding's optional digest pin. Statically declared adapter guests remain possible in the same invocation — a built `.wasm` path (or `include_bytes!`) plus a routed id. The shipped `src/main.rs` embeds the engine only; the journey host in [`examples/runtime.rs`](../../examples/runtime.rs) stays `MountAcquire`-only and loads its built mock component by path through the loader. There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. The only argv the guest never sees is the reserved host log flags — Omnia's direct-command entry peels `--debug` / `--quiet` anywhere in argv into the host log preset (bare defaults to muted INFO progress, `--quiet` is off, `--debug` adds backend debug tracing; the flags win over any ambient `RUST_LOG`).

Adapter references need no routes: a judgment over a non-empty embedded corpus declares the `list_docs` / `read_doc` function tools on the completion request, and the model's tool calls stream back to the adapter guest's own closure, which answers them in-process from the compiled-in docs (`emery_adapter::references`). Nothing binds an HTTP listener — the runtime invocation declares guests, mounts, and hosts only.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Kernels never read the environment: paths are fixed constants relative to the named preopens (the `.` project mount — the same strings resolve against the wasm32 preopen table and the native invocation directory).

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). The headline shape: `adapter` is the publishing leaf; `engine` owns the domain and the `specify` / `show` operations (shared plumbing in `emery_engine::handler`, resolution in `emery_engine::resolve`) plus the CLI surface (`emery_engine::cli`: the typed command route inventory, clap grammar carried on the operation inputs, projector, and exit contract) and returns `omnia_guest::Error` from those operations; the root package's `src/main.rs` owns the native deployment policy inline and its wasm32 lib declares the bare model provider (paths and adapter dispatch are structural, not provider capabilities); the root binary is one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the `.emery/` layout boundary, WASI carve-outs) live in [architecture.md](../standards/architecture.md).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  omnia::runtime! (command mode; embedded engine bytes, static guests and mounts)
              →  emery guest  →  typed command router  →  adapter dispatches route by routed id
```

The deployment projects nothing out of argv: no pre-boot fact depends on the parsed grammar — the invocation directory is the project root, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/engine/src/cli.rs` directly over the handler input types: each input derives `clap::Args` and implements `omnia_guest::api::Handler`, so the grammar and the handler input are one type and route decoding is infallible by construction. `emery_engine::cli` owns clap behavior, completions, inventory, `Client` dispatch, and the buffered `CommandResponse`. The WASI shim constructs the provider, runs that grammar, writes both channels, and hands the exit status to `omnia_guest::api::command::execute_wasi` (telemetry init/flush and exact exit). The handler contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure body is `ErrorBody`. There is no top-level envelope-version stamp.
- **Error discriminants** — the three kebab recovery codes (`specify-source-required`, `adapter-cli-too-old`, `spec-not-generated`), the loader's kebab refusals (`digest-mismatch` foremost), plus the four snake_case Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`); skills and tests grep on the `error` field, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `EMERY_FORMAT=json` is the environment equivalent.

## Exit codes

The exit-code contract is part of the public interface for operators and skill wrappers; `exit_code` in `crates/engine/src/cli.rs` maps `omnia_guest::Error` variants and is the single source of truth:

| Code | Variant          | Meaning                                                                                                                                        |
| ---- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `0`  | `EXIT_SUCCESS`   | Operation completed successfully                                                                                                               |
| `1`  | `BadRequest`     | Operator or input refusal. The `error` field is `specify-source-required`, `adapter-cli-too-old`, a loader refusal (`digest-mismatch`, `invalid-digest`, …), or the Omnia default `bad_request`. |
| `2`  | `NotFound`       | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`. Clap usage and unknown-verb also exit 2 (framework). |
| `3`  | `ServerError`    | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error`.                               |
| `4`  | `BadGateway`     | Upstream, model, or component-acquisition failure. The `error` field is the Omnia default `bad_gateway` or the loader's `acquire-failed`.        |

Guest commands inherit the same contract: `emery_engine::cli` projects parser and handler outcomes into a buffered command response; the WASI run export forwards its exit and the binary passes it through verbatim.

## Error handling

Commands return `omnia_guest::Error`. Construct the Omnia class that matches: `BadRequest` for operator or input refusals, `NotFound` for missing resources, `BadGateway` for upstream or model failures; everything else is `ServerError`. Do not introduce a house error type.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, omnia_guest::Error>`
2. Return a typed body implementing `Serialize + Render`
3. Let the command projector render success or apply the shared error contract

## Public Rust API

The root `emery` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `emery_engine::home::Home`.
