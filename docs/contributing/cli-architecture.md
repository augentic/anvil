# CLI Architecture

The `specify` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary, one guest

The shipped binary is a single, domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/main.rs`): it parses no verbs itself. Every verb — `--help` / `--version` included — runs in the specify (core) guest, which parses argv through the shared `cli` grammar; envelopes and exit codes pass through verbatim. Provisioning verbs (`init` without `--scaffold-only`, `adapters sync`, `workspace *`, `upgrade`, `plugins`) parse in the grammar but are refused by the guest router until their in-guest implementations land (DECISIONS.md §"One `specify` binary").

The core guest identity is versioned by the binary (`specify:core@<binary version>`, DECISIONS.md §"Core versioned by the binary"); in development the repo-root `omnia.toml` names the in-repo `specify.wasm` build.

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md §"Crate graph"](https://github.com/augentic/specify/blob/main/AGENTS.md#the-rust-workspace-specify-cli) and [docs/standards/architecture.md §"Workspace layout"](../standards/architecture.md#workspace-layout). The headline shape: `error` is the leaf; `cli` carries the full clap grammar, envelopes, and pure verb handlers consumed by the `workflow` shim; `workflow` owns the workflow domain and stays wasmtime-free; the root binary is a single `omnia::runtime!` invocation and depends on no `specify-*` crate.

Adapter deterministic helpers sit co-located beside their adapter prose in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) as in-guest library code compiled into each adapter's published component.

Vectis does not link an adapter-specific crate into the root `specify` binary. Its deterministic helpers (UI artifact validation, render-only scaffolding) are in-guest library code inside its published component; platform SDK, Cargo, Xcode, Gradle, and registry behavior lives in the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/merge.md) prompts (which carry the Vectis writer / reviewer / template-updater behavior).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs  →  omnia::runtime! (command mode)  →  specify guest (crates/specify)  →  cli::guest::parse + route
```

The full operator grammar — provisioning verbs included — lives in `cli` (`crates/cli/src/cli.rs`), so `--help`, usage errors, and completions are served by the guest with the real binary name. Verb handlers live under `crates/cli/src/commands/`; the handler contract (`Ctx`, `Out` / `Render` / `emit`, exit-code mapping) is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every successful body is the typed `*Body` rendered directly; every failure body is `ErrorBody`. There is no top-level envelope-version stamp.
- **Kebab-case error discriminants** — `adapter-not-installed`, `invalid-project`, `io` (never `missing_prerequisites`); skills and tests grep on the `error` / `code` fields, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `SPECIFY_FORMAT=json` is the environment equivalent.

## Exit codes

The exit-code contract is part of the public interface for skill authors; `Exit::from(&Error)` in `crates/cli/src/output.rs` is the single source of truth:

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `EXIT_SUCCESS` | Operation completed successfully |
| `1` | `EXIT_GENERIC_FAILURE` | I/O error, parse error, or any unclassified failure |
| `2` | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`, or clap usage errors |
| `3` | `EXIT_VERSION_TOO_OLD` | Binary version is below the `specify` floor in `.specify/project.yaml`, or below an adapter's declared `specify` compatibility floor |

Guest verbs inherit the same contract: the guest shim forwards `clap::Error::exit_code()` and handler exits through the WASI exit, and the binary passes them through verbatim.

## Error handling

Most commands use `error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, permission failures, runtime failures, and more.

The pattern for a command handler:

1. Call into a library crate function that returns `Result<T, error::Error>`
2. On success, format the result as text or JSON depending on `--format`
3. On error, emit the error envelope and return the appropriate `Exit`

## Public Rust API

The root `specify-cli` package is a binary-only crate. It does not expose a public library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `workflow::Plan`, `workflow::ProjectConfig`, or `error::Error`.
