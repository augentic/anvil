# CLI Architecture

The `specify` CLI lives in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repository. It is a Rust workspace producing a single host binary that skills invoke as a subprocess for core deterministic operations. Capability-specific deterministic helpers run as declared WASI tools through `specify tool run`.

## Core crate dependency graph

The core CLI crates stay capability-agnostic:

```text
specify (binary)
├── specify-change      Change orchestration, plan CRUD, locks, journals
├── specify-drift       Code-vs-baseline drift detection
├── specify-error       Shared error types
├── specify-merge       Delta merge engine, conflict detection
├── specify-capability  Capability resolution, caching, brief pipelines
├── specify-spec        Spec parsing, delta operations, requirement IDs
├── specify-task        Task file parsing, checkbox tracking
├── specify-validate    Artifact validation (structural + semantic)
└── specify-tool        Declared WASI tool resolution and execution
```

The crates form a layered dependency graph with `specify-error` at the base:

```d2
direction: down

specify: "specify (binary)" {
  shape: rectangle
}
change: specify-change
drift: specify-drift
error: specify-error
merge: specify-merge
capability: specify-capability
spec: specify-spec
task: specify-task
validate: specify-validate
tool: specify-tool

specify -> change
specify -> drift
specify -> merge
specify -> capability
specify -> spec
specify -> task
specify -> validate
specify -> tool

change -> error
change -> capability
drift -> error
drift -> spec
merge -> error
merge -> spec
merge -> capability
merge -> change
validate -> error
validate -> capability
validate -> spec
validate -> task
validate -> change
tool -> error
tool -> capability
spec -> error
task -> error
capability -> error
```

Vectis no longer links a capability-specific crate into the root `specify` binary. Its deterministic helpers are published as WASI command components declared by `capabilities/vectis/tools.yaml`: `vectis` (`validate`) for UI artifact validation and `vectis` (`scaffold`) for render-only scaffolding. The root CLI remains responsible for resolving, caching, permissioning, and running those tools; platform SDK, Cargo, Xcode, Gradle, and registry behavior remains skill-owned host workflow.

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs  →  Cli::parse()  →  commands::run(cli)  →  ExitCode
```

The CLI definition lives in `src/cli.rs`:

- **`Cli`** -- top-level struct with a global `--format text|json` flag and a `Commands` subcommand
- **`Commands`** -- enum with one variant per top-level subcommand (`Init`, `Status`, `Capability`, `Change`, `Registry`, `Workspace`, `Tool`, `Migrate`, `Completions`, etc.). The pre-v1 standalone `Validate`, `Merge`, `Spec`, and `Task` variants were folded into `Slice` during the v1 cleanup; `Registry` was added by RFC-9 §2A; `Schema` was renamed to `Capability` by RFC-13 §Migration.
- **Nested enums** -- subcommands with their own variants (e.g. `ChangeAction`, `RegistryAction`, `WorkspaceAction`, `ToolAction`, `CapabilityAction`)

The dispatcher in `src/commands/mod.rs` matches on the command variant and routes to a handler function. Most commands load a `CommandContext` from `.specify/project.yaml` (via `CommandContext::require`); a few "bare" commands (like `Init` and `Capability Resolve`) run without project context.

Each handler function returns a `CliResult` that maps to an exit code.

## JSON v2 contract

All JSON output follows the v2 contract:

- **Kebab-case keys** -- `app-name`, `project-dir`, `schema-version` (never `app_name` or `projectDir`); the `schema-version` JSON envelope key is intentionally kept as the wire-protocol version stamp and is unrelated to the Specify capability noun
- **`schema-version: 2`** -- auto-injected on every object response by the `emit_json` helper (envelope version, distinct from any Specify capability version)
- **Kebab-case error variants** -- `missing-prerequisites`, `invalid-project`, `io` (never `missing_prerequisites`)

The `--format` flag is global on `Cli` and controls output:

| Format | Success | Error |
|--------|---------|-------|
| `text` | Humanised summary | `error: <message>` on stderr |
| `json` | `{ "schema-version": 2, ...payload }` | `{ "error": "<variant>", "message": "...", "exit-code": N, "schema-version": 2 }` |

Key helpers in the binary:

- `emit_json(value)` -- injects `schema-version: 2` into an object and prints to stdout
- `emit_error` / `emit_json_error` -- maps `specify_error::Error` variants to kebab-case error envelopes
- `emit_tool_error` / tool-runtime helpers -- map resolver, permission, and runtime failures into the standard error envelope

## Exit codes

The exit-code contract is documented in `src/main.rs` and is part of the public interface for skill authors:

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `Success` | Operation completed successfully |
| `1` | `GenericFailure` | I/O error, parse error, or any unclassified failure |
| `2` | `ValidationFailed` | `specify validate` failed, or a declared tool resolver / permission / runtime validation failed |
| `3` | `VersionTooOld` | Binary version is below the `specify_version` floor in `.specify/project.yaml` |

The mapping from error variants to exit codes:

- `Error::SpecifyVersionTooOld` maps to exit `3`
- `Error::Validation` maps to exit `2`
- All other `Error` variants map to exit `1`
- A successful `validate` command where `report.passed == false` also returns exit `2`

## Error handling

Most commands use `specify_error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, declared-tool resolver failures, permission failures, runtime failures, and more. Capability tool diagnostics written by a WASI guest pass through `specify tool run` on stdout/stderr when the guest starts successfully.

The pattern for a command handler:

1. Call into a library crate function that returns `Result<T, specify_error::Error>`
2. On success, format the result as text or JSON depending on `--format`
3. On error, emit the error envelope and return the appropriate `CliResult`

## The `src/lib.rs` public API

The root `specify` crate re-exports types from all workspace crates as a curated public API. This is used by the binary's command handlers and could be consumed by external tools:

```rust
pub use specify_change::{ChangeMetadata, LifecycleStatus, Plan, ...};
pub use specify_error::{Error, ValidationStatus, ...};
pub use specify_merge::{MergeResult, PreviewResult, ...};
pub use specify_capability::{Capability, Pipeline, Brief, ...};
// ... and so on for each domain crate
```

Internal modules (`config`, `init`, `workspace`) provide the project-level orchestration that ties the domain crates together.
