# Operation shape

The contract every command operation obeys: how a command becomes an `omnia_guest::api::operation::Operation<P>` in the engine crates (`project`, `slice`, `change`), how `Ctx` is constructed from the provider's `Anchor`, how typed outputs implement `Render + Serialize`, and how shared command and HTTP projectors map terminal results.

## Shared operation plumbing (`project::handler`)

Every command is implemented by one stateless type implementing `omnia_guest::api::operation::Operation<P>`:

- **`Input`** is a flat, transport-neutral serde DTO (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields). HTTP deserializes it from path/query/body; command routing reaches it through an exhaustive `TryFrom<Args>`.
- **`call(input, context)`** loads `Ctx` from `context.provider`, delegates to the deterministic kernel, and returns the typed body.
- **`type Error = project::handler::Error`** — the workspace taxonomy plus the report-carrying `Error::Report` shape (below).

Deterministic operations bind `P: Anchor` only unless their kernel resolves adapters, in which case they additionally bind `Resolver`. The orchestration operations (`orchestrate::handlers`) bind the capabilities they drive: `P: Anchor + Model + Resolver + Source + Target` (or the subset they need), so the same impl serves the wasm guest, the native dev shim, and tests against scripted adapters.

```rust
// GOOD — default shape
impl<P: Anchor> Operation<P> for Frob {
    type Error = crate::handler::Error;
    type Input = FrobInput;
    type Output = FrobBody;

    async fn call(input: Self::Input, context: CallContext<'_, P>) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let outcome = some_crate::do_work(cx.layout(), &input)?;
        Ok(FrobBody::from(&outcome))
    }
}
```

Operations live in each domain module's `handlers` submodule beside its kernels.

## Ctx construction and the Anchor

Operations construct `project::handler::Ctx` inside `call` via `Ctx::load(context.provider)`. The project location comes from the provider's `Anchor`; operations never read the process CWD themselves.

`emery init` is the one operation that runs before a project exists: it anchors at the raw `Anchor::project_root` instead of loading `Ctx`.

## Output: `Render + Serialize`

Operations never write to stdout. Each returns a typed body implementing `Serialize` for JSON and `Render` for command text output. The HTTP projector always serializes JSON.

### Gate operations ride the error

Check surfaces return `ReportBody` on success and `Error::Report { body, source }` on failure. The command projector renders the report on stdout and the payload-free source error on stderr; the HTTP projector embeds the report under `report`.

## Errors and their projections

`project::handler::Error` wraps the workspace `error::Error` taxonomy (`Error::Core`) and adds `Error::Report`. The HTTP `EmeryProjector` in `crates/transport/src/http.rs` owns the single taxonomy → status projection (validation/argument → 422, version floor → 426, everything else → 500) and builds the JSON error body from the underlying taxonomy. `Exit` stays in `crates/transport` — there is no second exit table.

## Exit codes

The four-slot CLI exit-code table is fixed:

| Code | Name | When |
|---|---|---|
| 0 | `EXIT_SUCCESS` | Command succeeded |
| 1 | `EXIT_GENERIC_FAILURE` | Default `Error` → exit 1 |
| 2 | `EXIT_VALIDATION_FAILED` | `Error::Validation`, undeclared/over-permissioned tool, `Error::Argument` |
| 3 | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` (`emery-version-too-old` in JSON) |

`Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](../../crates/transport/src/command/output.rs) is the single source of truth. `EmeryProjector` uses it for every terminal operation or conversion error. Do not invent new exit codes.

## The transport crate (`crates/transport`)

`crates/transport` is a pure transport library: per-leaf clap `Args`, the `Globals` type, exhaustive `TryFrom<Args>` operation-input conversions, the reusable `omnia_guest::api::command` route assembly, the shared HTTP route assembly, the Emery command/HTTP projectors, and the fixed exit contract.

`crates/transport/src/command/*.rs` declares the clap derive surface. Each leaf route names a concrete `*Args` type; explicit `TryFrom<Args> for Input` implementations form the command transport boundary. Field parsers (`SourceArg`, closed enums, repeatable flags) live on `Args`. Global flags (`--format`) stay in `Globals`, not operation `Input`.

## The HTTP route table (`http.rs`)

`crates/transport/src/http.rs` owns one `omnia_guest::api::http::Router` assembly using typed `get_with` / `post_with` routes and `EmeryProjector`. The WASI shim serves it directly; native converts it to Axum, layers the process-wide write lock, and merges MCP shelves.

## Dispatch contract (`command.rs`)

The reusable command route table lives in `crates/transport/src/command.rs`. Both WASI and native shims construct an `Invoker`, assemble the router, execute it, and adapt the buffered response to their process boundary. The shared HTTP table lives in `crates/transport/src/http.rs`; native adds its write lock and MCP merge after `into_axum()`.

On wasm, the guest (`src/lib.rs`) exports `wasi:cli/run` explicitly, reads argv from the WASI environment, and writes the returned channels itself. Native writes the buffered response to the process streams. Both paths run the router through `transport::command::execute` — the shared wrapper that emits the `emery.command` span (bounded verb label plus exit code) — with the same assembly and the same `EmeryProjector`.

Target discipline per leaf arm:

1. Parse global flags and the selected leaf's concrete `Args`.
2. Convert `Args` through its explicit `TryFrom` implementation and invoke the typed operation.
3. Project success, operation failure, or conversion failure through `EmeryProjector`; provisioning routes return the standard argument refusal and completions remain synthetic router behavior.

Never put domain logic in `transport` or a shim's route match. Manual `Input { … }` construction in a `command.rs` arm is a shape defect. For the crate dependency direction this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Operation-shape notes

`source resolve <name>` and `target resolve <value>` never load a `Ctx`, because adapter resolution is read-only and runs before any project mutation; they invoke the provider's `Resolver` directly and anchor the default project dir on its `Anchor`. The two axes share one input shape; the axis is the request type. The target axis peels an opaque `@version` suffix (per the workflow contract §CLI surface).

`plan amend` extends the canonical `with_state::<Plan, _, _>(...)` operation shape with the three `--sources` flag families: `--sources <binding>...` (wholesale replace), `--add-source <binding>` (repeatable), `--remove-source <key>` (repeatable). The operation applies `--add-source` / `--remove-source` *after* the wholesale `Plan::amend(name, patch)` call so wholesale replacement plus targeted edits compose cleanly in a single invocation. The `--divergence` flag accepts only `likely | accepted | rejected` from the wire and emits a `plan.amend.divergence` journal event when (and only when) the field flips.

The Gate 1 stamp lives in `project::plan::stamp_approved`, invoked by the `plan execute` orchestration on a `pending` plan (idempotent — an already-approved plan is a no-op) and emitting a `plan.transition.approved` journal event; `plan transition <entry> --undo` is the one-rung reverse walk, and a call without `--undo` is an `Error::Argument` (exit 2). In both cases the journal append runs *after* `with_state` returns so the plan write and the journal append cannot interleave on failure.

## Gotcha — `emery init` and the version floor

`emery init` bypasses the `emery` version floor check (the file doesn't exist yet); every other project-aware command inherits it for free via `ProjectConfig::load`. Don't reimplement the floor check at a route or operation site.
