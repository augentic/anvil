# Operation shape

The contract every command operation obeys: how a command becomes an `omnia_guest::api::operation::Operation<P>` in `crates/engine`, how paths anchor at the deployed preopen layout, how typed outputs implement `Render + Serialize`, and how the command projector maps terminal results.

## Shared operation plumbing (`emery_engine::handler`)

Every command is implemented by one stateless type implementing `omnia_guest::api::operation::Operation<P>`:

- **`Input`** is a flat serde DTO that doubles as the verb's clap surface: it derives `clap::Args` alongside `Serialize`/`Deserialize` (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields; parsers and defaults ride `#[arg]` attributes). The router registers the operation directly over its input, so route decoding is infallible by construction.
- **`call(input, context)`** anchors at the deployed layout, delegates to the deterministic kernel, and returns the typed body.
- **`type Error = omnia_guest::Error`** — operations return Omnia's protocol error; do not introduce a house error type.

Deterministic operations bind `P: Provider` only unless their kernel issues model judgments, in which case they additionally bind `Model` — the one capability the provider still carries. Paths and adapter dispatch are not provider capabilities: paths are fixed constants relative to named preopens, and adapter operations ride the `emery:adapter/source` WIT imports directly.

```rust
// GOOD — deterministic kernel. Model-using operations stay `async fn call`.
impl<P: Provider> Operation<P> for Frob {
    type Error = omnia_guest::Error;
    type Input = FrobInput;
    type Output = FrobBody;

    fn call(
        input: Self::Input, _context: CallContext<'_, P>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> {
        std::future::ready(frob(input))
    }
}

fn frob(input: FrobInput) -> Result<FrobBody, omnia_guest::Error> {
    let outcome = some_crate::do_work(&input)?;
    Ok(FrobBody::from(&outcome))
}
```

Operations live in each domain module's `handlers` submodule beside its kernels.

## The deployed layout (C5)

Operations anchor at the `.` preopen inside `call`: paths are constants relative to the project-root mount (the invocation directory natively; `emery_engine::handler::preopen_path` normalizes operator paths inside it), and engine storage is named by fixed key/container formulas over the provider's storage capabilities. There is no project record and no project floor — a run's inputs arrive on the invocation, and there is nothing to be "inside". Operations never derive paths any other way — no environment reads, no ancestor walks, no CWD dependence; native tests script the storage capabilities in memory instead of chdir-ing into a tempdir.

## Output: `Render + Serialize`

Operations never write to stdout. Each returns a typed body implementing `Serialize` for JSON and `Render` for command text output.

## Errors and their projections

Operations return `omnia_guest::Error`. The command projector in `crates/engine/src/cli.rs` owns the 1:1 variant → exit projection and builds the JSON error body from `code()` / `description()`. `exit_code` stays in `emery_engine::cli` — there is no second exit table. Do not introduce a house error type or a report-carrying failure wrapper until a gate verb needs one.

## Exit codes

The Omnia 1:1 exit map is fixed:

| Code | Name            | When                                                                                                                                                          |
| ---- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`  | Command succeeded                                                                                                                                             |
| 1    | `BadRequest`    | Operator or input refusal. The `error` field is `specify-source-required`, `adapter-cli-too-old`, or the Omnia default `bad_request`.                          |
| 2    | `NotFound`      | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`. Clap usage and unknown-verb also exit 2 (framework).             |
| 3    | `ServerError`   | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error`.                                              |
| 4    | `BadGateway`    | Upstream or model failure. The `error` field is the Omnia default `bad_gateway`.                                                                               |

Omnia default codes are snake_case (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The three recovery discriminants stay kebab-case so skills can branch on them.

`exit_code` in [`crates/engine/src/cli.rs`](../../crates/engine/src/cli.rs) maps `omnia_guest::Error` variants and is the single source of truth. The command projector uses it for every terminal operation failure. Do not invent new exit codes.

## The CLI module (`emery_engine::cli`)

`crates/engine/src/cli.rs` is the whole CLI surface: the `Globals` type, the reusable `omnia_guest::api::command` route assembly, the Emery command projector, and the fixed exit contract. There is no HTTP surface: the engine binds no listener, so C3 (no unauthenticated HTTP ingress) is satisfied by absence rather than a refusal router.

There is no separate `*Args` layer: each operation `Input` derives `clap::Args` and registers directly (`run::<Input, Operation>()`), so grammar/input drift cannot exist and route decoding is infallible. Field parsers (closed `ValueEnum`s, repeatable flags, defaults) live on the input's `#[arg]` attributes. Global flags (`--format`) stay in `Globals`, not operation `Input`. A module-level layering rule replaces the old crate boundary: `cli` imports operation types and `omnia_guest::Error` only, never domain kernels.

## Dispatch contract (`cli.rs`)

The reusable command route table lives in `crates/engine/src/cli.rs`. `cli::router` binds a provider-carrying `Invoker` into the static grammar and returns the executable `Router`; `Router::execute` runs one argv under the framework's `command` span (selected route path plus exit code) and returns the buffered response. Wire-contract suites and the native journey rung call the same `cli::router` and assert on the buffered channels.

On wasm, the guest (`src/lib.rs`) exports `wasi:cli/run` explicitly, assembles that router over its provider, and hands it to `omnia_guest::api::command::execute_wasi` — the WASI last mile that reads argv, initializes and flushes guest telemetry, writes both channels, and exits with the exact status. Every path runs the same grammar and command `EmeryProjector`.

Target discipline per leaf arm:

1. Parse global flags and the selected leaf's `Input` — the input is its own clap surface.
2. Invoke the typed operation directly over the parsed input.
3. Project success or operation failure through the command projector; completions remain synthetic router behavior.

Never put domain logic in `cli` or a shim's route match. Manual `Input { … }` construction in a route arm is a shape defect. For the layering this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Gotcha — the only version floor is per adapter

There is no project-level `emery` version floor: the adapter compatibility floor (`requires-emery` from `metadata`, enforced during resolve as `adapter-cli-too-old`) is a `BadRequest`. Don't reintroduce a floor check at a route or operation site.
