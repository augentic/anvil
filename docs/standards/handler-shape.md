# Operation shape

The contract every command operation obeys: how a command becomes an `omnia_guest::api::operation::Operation<P>` in `crates/engine`, how `RequestContext` is assembled from the provider's `Anchor`, how typed outputs implement `Render + Serialize`, and how shared command and HTTP projectors map terminal results.

## Shared operation plumbing (`emery_engine::handler`)

Every command is implemented by one stateless type implementing `omnia_guest::api::operation::Operation<P>`:

- **`Input`** is a flat, transport-neutral serde DTO (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields). HTTP deserializes it from path/query/body; command routing reaches it through an exhaustive `TryFrom<Args>`.
- **`call(input, context)`** assembles `RequestContext` from `context.provider`, delegates to the deterministic kernel, and returns the typed body.
- **`type Error = emery_engine::handler::Error`** — the workspace taxonomy plus the report-carrying `Error::Report` shape (below).

Deterministic operations bind `P: Anchor` only unless their kernel resolves adapters, in which case they additionally bind `Resolver`, so the same impl serves the wasm guest and tests against scripted providers.

```rust
// GOOD — default shape
impl<P: Anchor> Operation<P> for Frob {
    type Error = crate::handler::Error;
    type Input = FrobInput;
    type Output = FrobBody;

    async fn call(input: Self::Input, context: CallContext<'_, P>) -> Result<Self::Output, Self::Error> {
        let request = RequestContext::load(context.provider)?;
        let outcome = some_crate::do_work(request.paths(), request.project(), &input)?;
        Ok(FrobBody::from(&outcome))
    }
}
```

Operations live in each domain module's `handlers` submodule beside its kernels.

## RequestContext and the Anchor (C5)

Project-scoped operations assemble the one `emery_engine::handler::RequestContext` inside `call` via `RequestContext::load(context.provider)`: the provider's `Anchor` supplies the paths, and the project loads fail-closed (version floor included) exactly once. Operations never read the process CWD themselves.

`emery init` is the one operation that runs before a project exists: it anchors at the raw `Anchor::project_root` instead of loading `RequestContext`.

## Output: `Render + Serialize`

Operations never write to stdout. Each returns a typed body implementing `Serialize` for JSON and `Render` for command text output. The HTTP projector always serializes JSON.

### Gate operations ride the error

Check surfaces return `ReportBody` on success and `Error::Report { body, source }` on failure. The command projector renders the report on stdout and the payload-free source error on stderr; the HTTP projector embeds the report under `report`.

## Errors and their projections

`emery_engine::handler::Error` wraps the workspace `emery_error::Error` taxonomy (`Error::Core`) and adds `Error::Report`. The command `EmeryProjector` in `crates/transport/src/command.rs` owns the taxonomy → exit projection and builds the JSON error body from the underlying taxonomy. `Exit` stays in `crates/transport` — there is no second exit table.

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

`crates/transport` is a pure transport library: per-leaf clap `Args`, the `Globals` type, exhaustive `TryFrom<Args>` operation-input conversions, the reusable `omnia_guest::api::command` route assembly, the HTTP refusal surface, the Emery command projector, and the fixed exit contract.

`crates/transport/src/command/*.rs` declares the clap derive surface. Each leaf route names a concrete `*Args` type; explicit `TryFrom<Args> for Input` implementations form the command transport boundary. Field parsers (`SourceArg`, closed enums, repeatable flags) live on `Args`. Global flags (`--format`) stay in `Globals`, not operation `Input`.

## The HTTP surface (`http.rs`)

`crates/transport/src/http.rs` owns the guest's non-MCP HTTP surface: one typed refusal router (C3). The unauthenticated pre-bound listener serves only the deployment-routed adapter MCP shelves; every other path and method answers a typed 404. There is no HTTP operation route table until an authenticated operator ingress is designed (target-architecture §7); `crates/transport/tests/router.rs::adr_0002_http_refusal` holds the refusal.

## Dispatch contract (`command.rs`)

The reusable command route table lives in `crates/transport/src/command.rs`. The WASI shim (and any test harness) constructs an `Invoker`, assembles the router, executes it, and adapts the buffered response to its process boundary.

On wasm, the guest (`src/lib.rs`) exports `wasi:cli/run` explicitly, reads argv from the WASI environment, and writes the returned channels itself. Native writes the buffered response to the process streams. Both paths run the router through `emery_transport::command::execute` — the shared wrapper that emits the `emery.command` span (bounded verb label plus exit code) — with the same assembly and the same command `EmeryProjector`.

Target discipline per leaf arm:

1. Parse global flags and the selected leaf's concrete `Args`.
2. Convert `Args` through its explicit `TryFrom` implementation and invoke the typed operation.
3. Project success, operation failure, or conversion failure through `EmeryProjector`; provisioning routes return the standard argument refusal and completions remain synthetic router behavior.

Never put domain logic in `transport` or a shim's route match. Manual `Input { … }` construction in a `command.rs` arm is a shape defect. For the crate dependency direction this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Gotcha — `emery init` and the version floor

`emery init` bypasses the `emery` version floor check (the file doesn't exist yet); every other project-aware command inherits it for free via `RequestContext::load` (over `emery_engine::project::Project::load`). Don't reimplement the floor check at a route or operation site.
