# Operation shape

The contract every command operation obeys: how a command becomes an `omnia_guest::api::operation::Operation<P>` in `crates/engine`, how paths anchor at the deployed preopen layout, how typed outputs implement `Render + Serialize`, and how shared command and HTTP projectors map terminal results.

## Shared operation plumbing (`emery_engine::handler`)

Every command is implemented by one stateless type implementing `omnia_guest::api::operation::Operation<P>`:

- **`Input`** is a flat, transport-neutral serde DTO (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields). HTTP deserializes it from path/query/body; command routing reaches it through an exhaustive `TryFrom<Args>`.
- **`call(input, context)`** anchors at the deployed layout, delegates to the deterministic kernel, and returns the typed body.
- **`type Error = emery_engine::handler::Error`** — an alias of the workspace taxonomy (`emery_error::Error`).

Deterministic operations bind `P: Provider` only unless their kernel issues model judgments, in which case they additionally bind `Model` — the one capability the provider still carries. Paths and adapter dispatch are not provider capabilities: paths are fixed constants relative to named preopens, and adapter operations ride the `emery:adapter/source` WIT imports directly.

```rust
// GOOD — deterministic kernel. Model-using operations stay `async fn call`.
impl<P: Provider> Operation<P> for Frob {
    type Error = crate::handler::Error;
    type Input = FrobInput;
    type Output = FrobBody;

    fn call(
        input: Self::Input, _context: CallContext<'_, P>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> {
        std::future::ready(frob(input))
    }
}

fn frob(input: FrobInput) -> Result<FrobBody, emery_error::Error> {
    let outcome = some_crate::do_work(&input)?;
    Ok(FrobBody::from(&outcome))
}
```

Operations live in each domain module's `handlers` submodule beside its kernels.

## The deployed layout (C5)

Operations anchor at `ExecutionPaths::deployed()` inside `call`: paths are constants relative to the `.` preopen (the project-root mount — the invocation directory natively), and engine storage is named by the `Locations` key/container formulas over the provider's storage capabilities. There is no project record and no project floor — a run's inputs arrive on the invocation, and there is nothing to be "inside". Operations never derive paths any other way — no environment reads, no ancestor walks, no CWD dependence; native tests script the storage capabilities in memory instead of chdir-ing into a tempdir.

## Output: `Render + Serialize`

Operations never write to stdout. Each returns a typed body implementing `Serialize` for JSON and `Render` for command text output. The HTTP projector always serializes JSON.

## Errors and their projections

`emery_engine::handler::Error` is the workspace `emery_error::Error` taxonomy. The command `EmeryProjector` in `crates/transport/src/command.rs` owns the taxonomy → exit projection and builds the JSON error body from it. `Exit` stays in `crates/transport` — there is no second exit table. Do not reintroduce a report-carrying failure wrapper until a gate verb needs one.

## Exit codes

The four-slot CLI exit-code table is fixed:

| Code | Name                     | When                                                                      |
| ---- | ------------------------ | ------------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`           | Command succeeded                                                         |
| 1    | `EXIT_GENERIC_FAILURE`   | Default `Error` → exit 1                                                  |
| 2    | `EXIT_VALIDATION_FAILED` | `Error::Validation`, undeclared/over-permissioned tool, `Error::Argument` |
| 3    | `EXIT_VERSION_TOO_OLD`   | `Error::AdapterCliTooOld` (`adapter-cli-too-old` in JSON)                 |

`Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](../../crates/transport/src/command/output.rs) is the single source of truth. `EmeryProjector` uses it for every terminal operation or conversion error. Do not invent new exit codes.

## The transport crate (`crates/transport`)

`crates/transport` is a pure transport library: per-leaf clap `Args`, the `Globals` type, exhaustive `TryFrom<Args>` operation-input conversions, the reusable `omnia_guest::api::command` route assembly, the guest HTTP surface (the read-only MCP spec shelf plus the refusal), the Emery command projector, and the fixed exit contract.

`crates/transport/src/command/*.rs` declares the clap derive surface. Each leaf route names a concrete `*Args` type; explicit `TryFrom<Args> for Input` implementations form the command transport boundary. Field parsers (`SourceArg`, closed enums, repeatable flags) live on `Args`. Global flags (`--format`) stay in `Globals`, not operation `Input`.

## The HTTP surface (`http.rs`)

`crates/transport/src/http.rs` owns the guest's HTTP surface: the read-only MCP spec shelf plus one typed refusal router (C3). `http::listener` serves the current generation and its id at `/mcp/emery/spec` — a stateless `McpServer` over a per-request storage snapshot (the same `Home::current_set` read `show` uses), exposing `spec://spec.md`, `spec://design.md`, and `spec://generation` as resources with mirroring read tools. Beside the deployment-routed adapter MCP shelves, every other path and method answers a typed 404 — reads are served, mutation is refused. There is no HTTP operation route table until an authenticated operator ingress is designed (target-architecture §7); the root scenario `tests/shelf.rs::every_route_refuses` holds the refusal.

## Dispatch contract (`command.rs`)

The reusable command route table lives in `crates/transport/src/command.rs`. `command::router` binds a provider-carrying `Invoker` into the static grammar and returns the executable `Router`; `Router::execute` runs one argv under the framework's `command` span (selected route path plus exit code) and returns the buffered response. Wire-contract suites and the native journey rung call the same `command::router` and assert on the buffered channels.

On wasm, the guest (`src/lib.rs`) exports `wasi:cli/run` explicitly, assembles that router over its provider, and hands it to `omnia_guest::api::command::execute_wasi` — the WASI last mile that reads argv, initializes and flushes guest telemetry, writes both channels, and exits with the exact status. Every path runs the same grammar and command `EmeryProjector`.

Target discipline per leaf arm:

1. Parse global flags and the selected leaf's concrete `Args`.
2. Convert `Args` through its explicit `TryFrom` implementation and invoke the typed operation.
3. Project success, operation failure, or conversion failure through `EmeryProjector`; provisioning routes return the standard argument refusal and completions remain synthetic router behavior.

Never put domain logic in `transport` or a shim's route match. Manual `Input { … }` construction in a `command.rs` arm is a shape defect. For the crate dependency direction this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Gotcha — the only version floor is per adapter

There is no project-level `emery` version floor: the adapter compatibility floor (`requires-emery` from `metadata`, enforced during resolve as `adapter-cli-too-old`) is the whole exit-3 surface. Don't reintroduce a floor check at a route or operation site.
