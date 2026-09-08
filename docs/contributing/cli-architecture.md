# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a static deployment expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (the native arm of `src/lib.rs`; `src/main.rs` only calls the generated `emery::main()`) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm` — the root `build.rs` child-builds the wasm32 engine, then in release builds ahead-of-time compiles it to a serialized wasmtime artifact, so startup deserializes rather than JIT-compiles the engine; debug builds embed the raw component and JIT at startup) and routed as the explicit `command_guest`. Deployment policy is CWD-rooted and inline in the invocation: the invocation directory mounts as the guest's `.` (no ancestor walk), the storage hosts bind engine state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), and the `plugins:` block declares the source seam (`emery:adapter/source@0.1.0`) with the compiled-in declarative `locations:` list (the `.` path root, then the `omnia.host` registry endpoint — no project cache), so a project-relative local `.wasm` adapter loads dynamically at run time — read fresh from the mount on every load — and an exact package reference fetches from its registry on every run, both verified against the binding's optional digest pin. Statically declared adapter guests remain possible in the same invocation — a built `.wasm` path (or `include_bytes!`) plus a routed id. The shipped runtime embeds the engine only; the journey host in [`examples/runtime.rs`](../../examples/runtime.rs) stays path-only and loads its built mock component by path through the loader. There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. The only argv the guest never sees is the reserved host log flags — Omnia's direct-command entry peels `--debug` / `--quiet` anywhere in argv into the host log preset (bare defaults to muted INFO progress, `--quiet` is off, `--debug` adds backend debug tracing; the flags win over any ambient `RUST_LOG`).

Adapter references need no routes: a judgment over a non-empty embedded corpus declares the `list_docs` / `read_doc` function tools on the completion request, and the model's tool calls stream back to the adapter guest's own closure, which answers them in-process from the compiled-in docs (`emery_adapter::references`). Nothing binds an HTTP listener — the runtime invocation declares guests, mounts, and hosts only.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Kernels never read the environment: paths are fixed constants relative to the named preopens (the `.` project mount — the same strings resolve against the wasm32 preopen table and the native invocation directory).

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md](../../AGENTS.md). The headline shape: `prose` and `source` are the leaves (the embedded-corpus registry and the `emery:adapter/source` contract), `adapter` is the guest-only SDK over them; `engine` owns the domain and the transport-neutral `specify` / `show` operations (path plumbing in `emery_engine::preopen_path`, adapter loading in the engine's `plugin` module) and returns `omnia_guest::Error` from those operations — no clap, no toml, no terminal text; `cli` (`emery-cli`) is the command façade over the engine: clap grammar, binding carriers, `Client` dispatch, the text/JSON projector, and the exit contract; the root package's `src/lib.rs` owns the native deployment policy inline (its native arm) and declares the bare model provider on wasm32 (paths and adapter dispatch are structural, not provider capabilities), running `emery_cli::run`; the root binary runs that one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the `.emery/` layout boundary, WASI carve-outs) live in [architecture.md](../standards/architecture.md).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  emery::main()  →  omnia::runtime! in src/lib.rs (command mode; embedded engine bytes, static guests and mounts)
              →  emery guest  →  typed command router  →  adapter dispatches route by routed id
```

The deployment projects nothing out of argv: no pre-boot fact depends on the parsed grammar — the invocation directory is the project root, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/cli/src/lib.rs` on façade-side `SpecifyArgs` / `ShowArgs` types (`clap::Args`), each decoding into its engine input (`emery_engine::specify::Specify`, `emery_engine::show::Show` — serde DTOs handled by the engine's `specify` / `show` fns, `omnia_guest::api::Handler<P, I>` through omnia's blanket impl) by exhaustive struct literal, so grammar/input drift is a compile error. `emery_cli` owns clap behavior, the binding carriers (argv, `--config`, root discovery), the per-body text render fns, and the hint table; `emery_cli::run(provider, argv)` is the whole entry, and it runs on omnia's command façade (`omnia_guest::api::command`): `parse::<App>` classifies argv, `Command::new(&client, &metadata, format).hints(hint).call(handler, decode, render)` projects each verb, `completions::<App>` answers the completions verb, and the buffered `Response` comes back. The WASI shim (`omnia_guest::command!(dispatch)` in `src/lib.rs`) constructs the provider, runs that grammar, and returns the `Response`; omnia's `Response` implements `IntoExit`, which writes both channels and hands the exit status to `execute_wasi` (telemetry init/flush and exact exit). The handler contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure is the flat `{error, message, exit-code}` envelope (optional `hint`). There is no top-level envelope-version stamp.
- **Error discriminants** — the three kebab recovery codes (`specify-source-required`, `unsupported-version`, `spec-not-generated`), the loader's kebab refusals (`refused`, `already-active`, `unavailable`, `internal`), plus the four snake_case Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`); skills and tests grep on the `error` field, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `EMERY_FORMAT=json` is the environment equivalent. Invocation metadata rides the environment too: `EMERY_REQUEST_ID`, `EMERY_CORRELATION_ID`, and `EMERY_CAUSATION_ID` (`Metadata::from_env("EMERY")`); a missing request id is minted from `wasi:random`, and `Client::call` runs the handler in a `handler` tracing span carrying the request and correlation ids.

Progress is `tracing`, never stdout: the engine emits a handful of INFO events at its slow seams (each source extraction, each synthesis pass) and DEBUG detail (claim counts), rendered by the guest subscriber `execute_wasi` installs and filtered by the guest's `RUST_LOG`. The semantic result stays the buffered `Response`; no engine code writes a process stream.

## Exit codes

The exit-code contract is part of the public interface for operators and skill wrappers; `omnia_guest::Error::exit_code` maps the variants and is the single source of truth, applied by omnia's `Command` projector:

| Code | Variant        | Meaning                                                                                                                                                                               |
| ---- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0`  | `EXIT_SUCCESS` | Operation completed successfully                                                                                                                                                      |
| `1`  | `BadRequest`   | Operator or input refusal. The `error` field is `specify-source-required`, `unsupported-version`, a loader refusal (`refused`, `already-active`), or the Omnia default `bad_request`. |
| `2`  | `NotFound`     | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`.                                                                                          |
| `3`  | `ServerError`  | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error` or the loader's `internal`.                                           |
| `4`  | `BadGateway`   | Upstream, model, or component-acquisition failure. The `error` field is the Omnia default `bad_gateway` or the loader's `unavailable`.                                                |
| `64` | `USAGE_EXIT`   | Clap usage error (unknown verb or flag, missing argument): clap's own text on stderr, no envelope. `EX_USAGE`, so exit 2 always means a `NotFound` envelope.                            |

Guest commands inherit the same contract: omnia's command façade projects parser, decoder, and handler outcomes into a buffered command response; the WASI run export forwards its exit and the binary passes it through verbatim.

## Error handling

Commands return `omnia_guest::Error`. Construct the Omnia class that matches: `BadRequest` for operator or input refusals, `NotFound` for missing resources, `BadGateway` for upstream or model failures; everything else is `ServerError`. Do not introduce a house error type.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, omnia_guest::Error>`
2. Return a typed `Serialize` body; its render fn in `crates/cli/src/text.rs` is its text mode
3. Let omnia's command projector render success or apply the shared error contract

## Public Rust API

The root `emery` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `emery_engine::specify::Specify` or `emery_cli::run`.
