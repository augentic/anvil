# CLI Architecture

The `specify` CLI lives in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repository. It is a Rust workspace producing a single host binary that skills invoke as a subprocess for core deterministic operations. Adapter-specific deterministic helpers run as declared WASI tools through `specify tool run`.

## Core crate dependency graph

The workspace is leaf → root with three crates plus the root binary:

```text
specify (binary)
└── specify-workflow      Change orchestration, plan/slice lifecycle, registry,
    │                   merge engine, spec/task parsing, adapter resolution,
    │                   journal, schema validation (every domain module)
    ├── specify-tool    Declared WASI tool resolution and execution
    └── specify-error   thiserror + serde-saphyr error variants (leaf)
```

WASI tools live in the sibling `wasi-tools/` workspace (`wasi-tools/contract`, `wasi-tools/vectis`) and are intentionally carved out of the host workspace's discipline.

The crates form a layered dependency graph with `specify-error` at the base:

![specify-cli crate dependency graph](../assets/diagrams/contributing/cli-crate-graph.svg)

Vectis does not link an adapter-specific crate into the root `specify` binary. Its deterministic helpers are published as WASI command components declared by [`adapters/targets/vectis/adapter.yaml`](../../adapters/targets/vectis/adapter.yaml) (`tools[]`): `vectis` (`validate`) for UI artifact validation and `vectis` (`scaffold`) for render-only scaffolding. The root CLI is responsible for resolving, caching, permissioning, and running those tools; platform SDK, Cargo, Xcode, Gradle, and registry behavior lives in the Vectis target's [`build`](../../adapters/targets/vectis/briefs/build.md) and [`merge`](../../adapters/targets/vectis/briefs/merge.md) briefs (which carry the Vectis writer / reviewer / template-updater behavior).

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs  →  Cli::parse()  →  commands::run(cli)  →  ExitCode
```

The CLI definition lives in `src/cli.rs`:

- **`Cli`** -- top-level struct with a global `--format text|json` flag and a `Commands` subcommand
- **`Commands`** -- enum with one variant per top-level subcommand (`Init`, `Status`, `Context`, `Adapter`, `Codex`, `Tool`, `Compatibility`, `Slice`, `Change`, `Registry`, `Workspace`, and hidden `Completions`). The standalone `Validate`, `Merge`, `Spec`, and `Task` families have been folded into `Slice`; `Schema` has been renamed to `Adapter`.
- **Nested enums** -- subcommands with their own variants (e.g. `ChangeAction`, `RegistryAction`, `WorkspaceAction`, `ToolAction`, `AdapterAction`)

The dispatcher in `src/commands.rs` matches on the command variant and routes to a handler function. Most commands load a `CommandContext` from `.specify/project.yaml` (via `CommandContext::load`); a few unscoped commands (like `Init` and `Adapter Resolve`) run without project context.

Each handler function returns an `Exit` that maps to an exit code.

## JSON Envelope Contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** -- `app-name`, `project-dir`, `envelope-version` (never `app_name` or `projectDir`); the `envelope-version` JSON envelope key is intentionally kept as the wire-protocol version stamp and is unrelated to the Specify adapter noun
- **`envelope-version`** -- auto-injected on every object response by the binary's `emit_response` helper. The current value is `ENVELOPE_VERSION` in `specify-cli/src/output.rs`.
- **Kebab-case error variants** -- `missing-prerequisites`, `invalid-project`, `io` (never `missing_prerequisites`)

The `--format` flag is global on `Cli` and controls output:

| Format | Success | Error |
|--------|---------|-------|
| `text` | Humanised summary | `error: <message>` on stderr |
| `json` | `{ "envelope-version": N, ...payload }` | `{ "envelope-version": N, "error": "<variant>", "message": "...", "exit-code": N }` |

Key helpers in the binary:

- `emit_response(value)` -- injects `envelope-version` into an object and prints to stdout
- `emit(format, value)` / `emit_err(format, value)` -- route typed `Render` bodies to text or JSON
- `emit_error` / `emit_json_error` -- maps `specify_error::Error` variants to kebab-case error envelopes

## Exit codes

The exit-code contract is documented in `src/main.rs` and is part of the public interface for skill authors:

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `Success` | Operation completed successfully |
| `1` | `GenericFailure` | I/O error, parse error, or any unclassified failure |
| `2` | `ValidationFailed` / `ArgumentError` | Validation failure, argument-shape failure discovered after clap parsing, or a declared tool resolver / permission failure |
| `3` | `VersionTooOld` | Binary version is below the `specify_version` floor in `.specify/project.yaml`, or below an adapter's declared `specify` compatibility floor |

The mapping from error variants to exit codes:

- `Error::CliTooOld` maps to exit `3` and serializes as `specify-version-too-old`
- `Error::AdapterCliTooOld` (an adapter's declared `specify` compatibility floor) maps to exit `3` and serializes as `adapter-cli-too-old`
- `Error::Validation` maps to exit `2`
- `Error::Argument`, declared-tool denials, and structural plan errors map to exit `2`
- All other `Error` variants map to exit `1`

## Error handling

Most commands use `specify_error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, declared-tool resolver failures, permission failures, runtime failures, and more. Adapter tool diagnostics written by a WASI guest pass through `specify tool run` on stdout/stderr when the guest starts successfully.

The pattern for a command handler:

1. Call into a library crate function that returns `Result<T, specify_error::Error>`
2. On success, format the result as text or JSON depending on `--format`
3. On error, emit the error envelope and return the appropriate `Exit`

## Public Rust API

The root `specify` package is a binary-only crate. It does not expose `src/lib.rs` or re-export workspace types. Code that needs Rust APIs imports the member crates directly, for example `specify_workflow::Plan`, `specify_workflow::ProjectConfig`, or `specify_error::Error`.
