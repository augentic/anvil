# CLI Architecture

The `specify` CLI lives in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repository. It is a Rust workspace producing a single binary that skills invoke as a subprocess for all deterministic operations.

## Crate dependency graph

The workspace contains 10 library crates plus the root binary crate:

```text
specify (binary)
├── specify-change      Change lifecycle, plan CRUD, locks, journals
├── specify-drift       Code-vs-baseline drift detection
├── specify-error       Shared error types
├── specify-platform   Multi-repo peer configuration
├── specify-merge       Delta merge engine, conflict detection
├── specify-schema      Schema resolution, caching, brief pipelines
├── specify-spec        Spec parsing, delta operations, requirement IDs
├── specify-task        Task file parsing, checkbox tracking
├── specify-validate    Artifact validation (structural + semantic)
└── specify-vectis      Crux project bootstrap and verification
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
platform: specify-platform
merge: specify-merge
schema: specify-schema
spec: specify-spec
task: specify-task
validate: specify-validate
vectis: "specify-vectis (isolated)"

specify -> change
specify -> drift
specify -> platform
specify -> merge
specify -> schema
specify -> spec
specify -> task
specify -> validate
specify -> vectis

change -> error
change -> schema
drift -> error
drift -> spec
platform -> error
platform -> change
merge -> error
merge -> spec
merge -> schema
merge -> change
validate -> error
validate -> schema
validate -> spec
validate -> task
validate -> change
spec -> error
task -> error
schema -> error
```

`specify-vectis` is intentionally isolated -- it depends only on external crates (`clap`, `ureq`, `syn`, `toml`) and has no internal `specify-*` dependencies. This keeps the Crux bootstrap tooling decoupled from the rest of the system.

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs  →  Cli::parse()  →  commands::run(cli)  →  ExitCode
```

The CLI definition lives in `src/cli.rs`:

- **`Cli`** -- top-level struct with a global `--format text|json` flag and a `Commands` subcommand
- **`Commands`** -- enum with one variant per top-level subcommand (`Init`, `Validate`, `Merge`, `Status`, `Task`, `Schema`, `Change`, `Spec`, `Plan`, `Initiative`, `Workspace`, `Vectis`, `Completions`)
- **Nested enums** -- subcommands with their own variants (e.g. `PlanAction`, `ChangeAction`, `VectisAction`)

The dispatcher in `src/commands/mod.rs` matches on the command variant and routes to a handler function. Most commands load a `CommandContext` from `.specify/project.yaml` (via `CommandContext::require`); a few "bare" commands (like `Init` and `Schema Resolve`) run without project context.

Each handler function returns a `CliResult` that maps to an exit code.

## JSON v2 contract

All JSON output follows the v2 contract:

- **Kebab-case keys** -- `app-name`, `project-dir`, `schema-version` (never `app_name` or `projectDir`)
- **`schema-version: 2`** -- auto-injected on every object response by the `emit_json` helper
- **Kebab-case error variants** -- `missing-prerequisites`, `invalid-project`, `io` (never `missing_prerequisites`)

The `--format` flag is global on `Cli` and controls output:

| Format | Success | Error |
|--------|---------|-------|
| `text` | Humanised summary | `error: <message>` on stderr |
| `json` | `{ "schema-version": 2, ...payload }` | `{ "error": "<variant>", "message": "...", "exit-code": N, "schema-version": 2 }` |

Key helpers in the binary:

- `emit_json(value)` -- injects `schema-version: 2` into an object and prints to stdout
- `emit_error` / `emit_json_error` -- maps `specify_error::Error` variants to kebab-case error envelopes
- `emit_vectis_error` -- parallel error emitter for the `specify-vectis` error type

## Exit codes

The exit-code contract is documented in `src/main.rs` and is part of the public interface for skill authors:

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `Success` | Operation completed successfully |
| `1` | `GenericFailure` | I/O error, parse error, or any unclassified failure |
| `2` | `ValidationFailed` | `specify validate` failed, or `specify vectis` missing prerequisites |
| `3` | `VersionTooOld` | Binary version is below the `specify_version` floor in `.specify/project.yaml` |

The mapping from error variants to exit codes:

- `Error::SpecifyVersionTooOld` maps to exit `3`
- `Error::Validation` maps to exit `2`
- All other `Error` variants map to exit `1`
- A successful `validate` command where `report.passed == false` also returns exit `2`

## Error handling

Most commands use `specify_error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, and more. The `specify-vectis` crate has its own `VectisError` type because it is an isolated subtree.

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
pub use specify_schema::{Schema, Pipeline, Brief, ...};
// ... and so on for each domain crate
```

Internal modules (`config`, `init`, `workspace`) provide the project-level orchestration that ties the domain crates together.
