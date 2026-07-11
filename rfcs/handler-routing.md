# Operation routing

Specify has one transport-neutral execution contract — `omnia_guest::api::operation::Operation<P>` — and two explicit typed transport routers, command and HTTP. Both routers invoke the same operations through `omnia_guest::api::invoke::Invoker<P>`. This document is the design of record for command implementation and routing.

## Vocabulary

| Layer | Term | Example |
|---|---|---|
| Operator surface | **command** | `specify slice build my-slice` |
| Command grammar | **route** | `["slice", "build"]` plus `slice::BuildArgs` |
| Implementation | **operation** | `orchestrate::handlers::Build: Operation<P>` |
| Wire input | **Input** | `BuildInput`, the transport-neutral operation payload |
| Command input | **Args** | `BuildArgs`, the clap parser for one command route |
| Invocation | **Invoker** | owner + provider supplied to either typed router |
| Projection | **projector** | maps typed output/error to command channels or an HTTP response |

A command is implemented by exactly one operation. The `handlers` module name remains the domain-module convention for co-locating operation types; it does not imply the retired Omnia `Handler` trait.

## Design

```text
command argv ──→ typed command Router ──→ TryFrom<Args> ─┐
                                                        ├─→ Invoker<P> ─→ Operation::call
HTTP request ─→ typed HTTP Router ─────→ Input decode ──┘                    │
                                                                             └─→ typed Output / Error
```

The hard-cut invariants are:

1. Every workflow command is a stateless `Operation<P>` with associated `Input`, `Output`, and `Error` types.
2. `Invoker<P>` is the only execution seam used by command and HTTP routing.
3. `crates/transport/src/command.rs` is the complete typed command inventory.
4. `crates/transport/src/http.rs` is the complete typed HTTP inventory.
5. Command conversion is explicit and exhaustive through `TryFrom<Args> for Input`; there is no serde round-trip between command args and operation input.
6. The WASI and native shims construct providers and invokers, call the shared routers, and adapt terminal transport output. They do not own route inventories or domain conversions.
7. Operation outputs are typed values implementing `Serialize`; command-visible outputs also implement `workflow::handler::Render`.

## Operation contract

Operations live beside their domain kernels in `crates/workflow`:

```rust
impl<P: Anchor> Operation<P> for Build {
    type Error = workflow::handler::Error;
    type Input = BuildInput;
    type Output = BuildBody;

    async fn call(
        input: Self::Input,
        context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        // Delegate to the domain kernel and return a typed body.
    }
}
```

`Input` is a flat serde DTO using kebab-case wire names. HTTP decodes it from merged path, query, and body values. Command routing reaches the same type through an explicit `TryFrom<Args>` implementation. Shape validation may therefore occur in either transport decoding/conversion or at the start of `Operation::call`; project-dependent validation belongs in the operation or its domain kernel.

Provider bounds state the capabilities an operation consumes. Deterministic operations usually require `P: Anchor`; orchestration operations add `Model`, `SourceSeam`, or `TargetSeam` as needed.

`workflow::handler` retains its name as shared operation plumbing. It owns `Anchor`, `Ctx`, `Render`, `ReportBody`, and the operation-layer `Error`. It contains no transport parser, stdout writer, or exit table.

## Command router

`crates/transport/src/command.rs` assembles an `omnia_guest::api::command::Router<P, Globals>` from concrete route paths, concrete `Args`, concrete workflow operations, and `SpecifyProjector`.

```rust
route!(
    ["slice", "build"],
    slice::BuildArgs,
    workflow::orchestrate::handlers::Build,
    "Build a slice"
);
```

Each command leaf has a clap-only `Args` type under `crates/transport/src/args/*.rs`. Global flags such as `--format` and `--plan-dir` stay in `Globals`. Each supported leaf has an exhaustive conversion:

```rust
impl TryFrom<BuildArgs> for BuildInput {
    type Error = error::Error;

    fn try_from(args: BuildArgs) -> Result<Self, Self::Error> {
        Ok(Self { name: args.name })
    }
}
```

The conversion is the intentional transport boundary. It handles command-specific desugaring and can reject invalid flag combinations with `error::Error`. Compiler errors expose field drift; router tests cover paths, conversion failures, projections, help, and completions.

`SpecifyProjector` maps:

- operation output to JSON or `Render` text on stdout with exit 0;
- `workflow::handler::Error` to the fixed error/exit contract;
- `Args → Input` conversion failure to the same error/exit contract.

Unsupported provisioning commands remain typed routes to a private `Unsupported` operation so they participate in normal grammar and projection. Completions are router-owned synthetic behavior.

## HTTP router

`crates/transport/src/http.rs` assembles one `omnia_guest::api::http::Router<P>` with typed `get_with` and `post_with` routes:

```rust
Router::new(invoker)
    .route(
        "/slice/{name}/build",
        post_with::<workflow::orchestrate::handlers::Build, P, SpecifyProjector>(
            SpecifyProjector,
        ),
    )
```

The HTTP router merges route parameters, query values, and request-body fields into `Operation::Input`, invokes the operation through the same `Invoker<P>`, and projects `Output` or `workflow::handler::Error` to JSON. The HTTP `SpecifyProjector` is the single taxonomy-to-status table.

GET is used for reads; POST is used for mutation and judgment. HTTP route paths are explicit and auditable. Native converts the typed router with `into_axum()`, adds the process-wide mutation lock, and merges MCP shelves.

## Shim responsibilities

The WASI guest and native harness share router assemblies rather than duplicating route tables.

```text
src/command.rs              construct Invoker → transport::command::router → command::execute_wasi
src/http.rs                 construct Invoker → transport::http::router → http::serve
harness/native/command.rs   construct Invoker → shared command Router::execute → write channels
harness/native/http.rs      construct Invoker → shared HTTP Router::into_axum → lock + MCP merge
```

`src/command.rs` and `src/http.rs` explicitly export `wasi:cli/run` and `wasi:http/incoming-handler`. `src/lib.rs` is module wiring only. Adapter resolution is a provider capability: the WASI provider owns component metadata dispatch and the native provider owns its linked-crate catalog. The transport shims carry neither resolver registration nor per-command match arms.

## Where operations live

| Commands | Operations | Beside |
|---|---|---|
| `journal emit/show` | `workflow::journal::handlers` | journal append and projection kernels |
| `slice create/transition/drop/validate/...`, `archive prune` | `workflow::slice::handlers` | slice actions, validation, model, merge kernels |
| `plan create/add/amend/remove/validate/...` | `workflow::change::plan::handlers` | the plan state machine |
| `source survey/extract`, `slice refine/build/merge run`, `plan author/execute` | `workflow::orchestrate::handlers` | orchestration kernels |
| `registry validate/add/remove` | `workflow::registry::handlers` | registry types and mutation kernels |
| `source resolve`, `target resolve` | `workflow::adapter::handlers` | axis-specific resolvers |
| `init --scaffold-only` | `workflow::init::handlers` | scaffold kernel |

## Adding a command

1. Define the operation `Input`, typed output body, and stateless operation type in the owning workflow domain's `handlers` module. Implement `Operation<P>` and `Render` for command-visible output.
2. Define the concrete clap `Args` under `crates/transport/src/args/*.rs`.
3. Add an explicit `TryFrom<Args> for Input`.
4. Register the command in `crates/transport/src/command.rs` with its path, args, operation, and help.
5. If HTTP-exposed, register the same operation in `crates/transport/src/http.rs` with an explicit path and method.
6. Test domain behavior through the operation public surface and transport behavior through `crates/transport/tests/router.rs` or the native full-loop tests.

## Hard-cut exclusions

The following shapes are retired and must not return:

- `omnia_guest::api::Handler`, `Handler::from_input`, or `Handler::handle`;
- `Reply<T>` and transport-shaped `Out<T>` as operation output currency;
- `crates/transport/src/front.rs`, `transport::front::run`, or a command-args serde round-trip;
- command route tables duplicated in WASI and native shims;
- per-command `Input` construction in shim match arms;
- a hidden `guest!` command export.

This is a hard cut, not a compatibility layer. Old traits, bridges, fixtures, and explanatory prose are removed rather than aliased.
