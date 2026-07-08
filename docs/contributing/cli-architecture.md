# CLI Architecture

The `specify` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## Two layers, one binary

The shipped binary is two strictly separated layers:

- **A native provisioning front** — the closed verb set that must run natively (`init`, `adapters sync`, `upgrade`, `plugins`, plus the acknowledged `workspace` residue and the hidden framework-repo `lint framework`). These are the only verbs the binary parses; the grammar lives in `src/runtime/cli.rs`.
- **Blind forwarding** — every other argv forwards **unparsed** to the workflow (core) guest through `specify_runtime::drive` and the generated deployment manifest, `--help` / `--version` included. Envelopes and exit codes pass through verbatim.

The core guest resolves from the global adapter store by the binary's own version (`specify:core@<binary version>`), with a `SPECIFY_CORE_PATH` / in-repo dev-build override for core-guest iteration.

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md §"Crate graph"](https://github.com/augentic/specify/blob/main/AGENTS.md#the-rust-workspace-specify-cli) and [docs/standards/architecture.md §"Workspace layout"](../standards/architecture.md#workspace-layout). The headline shape: `specify-error` is the leaf; `specify-dispatch` carries the full clap grammar, envelopes, and pure verb handlers consumed by the `specify-workflow` shim; `specify-workflow-lib` owns the workflow domain and stays wasmtime-free; `specify-standards` is its sibling (neither imports the other); the root binary is a single `omnia::runtime!` invocation and depends on no `specify-*` crate.

Adapter deterministic helpers no longer live in a sibling `wasi-tools/` workspace; they sit co-located beside their adapter prose in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) as in-guest library code compiled into each adapter's published component.

Vectis does not link an adapter-specific crate into the root `specify` binary. Its deterministic helpers (UI artifact validation, render-only scaffolding) are in-guest library code inside its published component; platform SDK, Cargo, Xcode, Gradle, and registry behavior lives in the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/merge.md) prompts (which carry the Vectis writer / reviewer / template-updater behavior).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs  →  runtime::run(argv)  →  first-token triage  →  native provisioning handler
                                                            ↘  specify_runtime::drive (everything else)
```

The full operator grammar — provisioning verbs included — lives in `specify-dispatch` (`crates/dispatch/src/cli.rs`), so `--help`, usage errors, and completions are served by the guest with the real binary name. Native handlers live under `src/runtime/commands/`; the handler contract (`Ctx`, `Out` / `Render` / `emit`, exit-code mapping) is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure body is `ErrorBody`. There is no top-level envelope-version stamp.
- **Kebab-case error discriminants** — `adapter-not-installed`, `invalid-project`, `io` (never `missing_prerequisites`); skills and tests grep on the `error` / `code` fields, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `SPECIFY_FORMAT=json` is the environment equivalent.

## Exit codes

The exit-code contract is part of the public interface for skill authors; `Exit::from(&Error)` in `src/runtime/output.rs` is the single source of truth:

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `EXIT_SUCCESS` | Operation completed successfully |
| `1` | `EXIT_GENERIC_FAILURE` | I/O error, parse error, or any unclassified failure |
| `2` | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`, or clap usage errors |
| `3` | `EXIT_VERSION_TOO_OLD` | Binary version is below the `specify` floor in `.specify/project.yaml`, or below an adapter's declared `specify` compatibility floor |

Forwarded verbs inherit the same contract: the guest shim forwards `clap::Error::exit_code()` and handler exits through the WASI exit, and the binary passes them through verbatim.

## Error handling

Most commands use `specify_error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, permission failures, runtime failures, and more.

The pattern for a command handler:

1. Call into a library crate function that returns `Result<T, specify_error::Error>`
2. On success, format the result as text or JSON depending on `--format`
3. On error, emit the error envelope and return the appropriate `Exit`

## Public Rust API

The root `specify` package is a binary-only crate. It does not expose a public library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `specify_workflow_lib::Plan`, `specify_workflow_lib::ProjectConfig`, or `specify_error::Error`.
