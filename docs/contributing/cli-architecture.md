# CLI Architecture

The `specify` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: launcher + runtime

The shipped binary is a self-assembling deployment launcher (RFC-70 Stage 1) in front of a domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (`src/omnia.rs`). For each invocation, `main` runs the `launcher` crate's prepare pipeline — anchor the project root, derive the component closure (engine guest + every adapter the command could dispatch), hydrate store misses from the configured registry with fail-closed digest verification (one pass per store component), and assemble a typed in-memory deployment — then hands the deployment to the generated `host::run`. There is no `omnia.toml` and no `run --config` surface.

The launcher owns exactly two fast paths: `--version` (printed natively) and pre-run rejection (a grammar failure or verification failure exits with the standard failure envelope before anything starts). Every other command — `--help` included — runs in the specify (core) guest through the shared typed command router; envelopes and exit codes pass through verbatim. Removed provisioning and bootstrap surfaces are not advertised as deferred commands.

The engine guest identity is versioned by the binary (`specify:engine@<binary version>`) and resolves from the global adapter store (`~/.specify/store/engine@<version>.wasm`), hydrated from the registry on first miss. Adapter closure entries resolve through the same probes the in-guest resolver uses: the global store for package pins, the seeded project component cache for bare names. `SPECIFY_HOME` is the single relocation override — store and cache derive together beneath it (the Cargo model) — captured once per invocation into a carried `Locations` value; nothing below the composition root reads the environment.

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [AGENTS.md §"Crate graph"](https://github.com/augentic/specify/blob/main/AGENTS.md#the-rust-workspace-specify-cli) and [docs/standards/architecture.md §"Workspace layout"](../standards/architecture.md#workspace-layout). The headline shape: `error` is the leaf; the three engine crates own the domain and every command operation (`project` the foundation + init, `slice` the refine-build-merge loop, `change` the plan loop; `Operation<P>` impls in each crate's `<domain>::handlers`, shared plumbing in `project::handler`); `transport` owns the typed command/HTTP route inventories, clap args, explicit conversions, projectors, and exit contract; `launcher` owns the native-only pre-run pipeline (closure derivation, hydration + verification, deployment assembly); the root binary nests `omnia::runtime!` in `mod host` and its native `main` composes `launcher::prepare` with the generated `host::run`.

Adapter deterministic helpers sit co-located beside their adapter prose in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) as in-guest library code compiled into each adapter's published component.

Vectis does not link an adapter-specific crate into the root `specify` binary. Its deterministic helpers (UI artifact validation, render-only scaffolding) are in-guest library code inside its published component; platform SDK, Cargo, Xcode, Gradle, and registry behavior lives in the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/merge.md) prompts (which carry the Vectis writer / reviewer / template-updater behavior).

## Dispatch pattern

The binary entry point is thin:

```text
src/omnia.rs  →  launcher::prepare (anchor → closure → hydrate + verify → assemble)
              →  host::run / omnia::runtime! (command mode)  →  specify guest  →  typed command router
```

The launcher sees adapter selectors in argv through `transport::command::selectors::from_argv`, which parses argv against the *same* assembled clap grammar the guest router executes (a grammar-only provider that never dispatches). The selector-bearing routes are pinned by `SELECTOR_ROUTES` and guarded by the grammar-coverage test in `crates/transport/tests/selectors.rs` — a new selector-bearing verb cannot land without being classified there.

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

| Code | Constant | Meaning |
|------|----------|---------|
| `0` | `EXIT_SUCCESS` | Operation completed successfully |
| `1` | `EXIT_GENERIC_FAILURE` | I/O error, parse error, or any unclassified failure |
| `2` | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`, or clap usage errors |
| `3` | `EXIT_VERSION_TOO_OLD` | Binary version is below the `specify` floor in `.specify/project.yaml`, or below an adapter's declared `specify` compatibility floor |

Guest commands inherit the same contract: `omnia_guest::api::command` projects parser, conversion, and operation outcomes into a buffered command response; the WASI seam forwards its exit and the binary passes it through verbatim.

## Error handling

Most commands use `error::Error`, a unified error enum with structured variants covering I/O, YAML parsing, validation, lifecycle violations, permission failures, runtime failures, and more.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, error::Error>`
2. Return a typed body implementing `Serialize + Render`
3. Let the command or HTTP projector render success or apply the shared error contract

## Public Rust API

The root `specify` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `project::plan::Plan`, `project::config::ProjectConfig`, or `error::Error`.
