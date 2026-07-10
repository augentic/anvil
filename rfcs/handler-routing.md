# Handler Routing

One routing mechanism — the `omnia_guest::api::Handler` trait — under two transports (CLI and HTTP) and two shims (the wasm guest at `src/`, the native `specify-dev` at `harness/native/`). This document is the design of record for how a `specify` command is implemented, where its handler lives, and how each transport reaches it. It supersedes RFC-61, which is implemented and removed.

## Vocabulary

Specify overloads several nearby terms; this RFC uses one sense per layer:


| Layer            | Term              | Example                                                                                      |
| ---------------- | ----------------- | -------------------------------------------------------------------------------------------- |
| Operator surface | **command**       | `specify slice build my-slice`                                                               |
| Grammar leaf     | **action**        | `build`, `emit`, `transition` (the clap subcommand enums: `SliceAction`, `JournalAction`, …) |
| Implementation   | **handler**       | one `Handler<P>` impl with a flat `Input` DTO, `from_input`, and `handle`                    |
| Resource prefix  | **command group** | slice commands, plan commands, journal commands                                              |
| Wire shape       | `**Input**`       | the flat serde DTO shared by every transport for one command                                 |
| Argv mirror      | `**Args**`        | the clap struct in `crates/cli` that parses the leaf and serializes onto `Input`             |


A **command** is one operator-facing invocation. Each command is implemented by exactly one **handler** with exactly one `**Input`** and, on the argv side, exactly one mirror `**Args**` struct. **Actions** are the leaves in the clap grammar; **command groups** are the resource prefixes (`slice`, `plan`, `journal`) that namespace them.

Reserve **verb** for unrelated grammar elsewhere (skill-description imperatives, breakout slash skills) — not for handlers or commands.

## Transport model

Every command crosses four layers. HTTP already keeps layers 3–4 generic; CLI must do the same.

```text
Operator surface     specify slice build my-slice --format json
        ↓
Grammar / registry   which command? (path + read/write kind)
        ↓
Extraction           argv or HTTP request → Handler::Input
        ↓
Execution            Handler::from_input → handle → render
```


| Layer      | HTTP today                                                     | CLI target                                                     |
| ---------- | -------------------------------------------------------------- | -------------------------------------------------------------- |
| Registry   | `.route("/slice/{name}/build", route::post::<Build, P>())`     | `SliceAction::Build(args) => cli::post::<Build, _>(…, args)`   |
| Extraction | `route::post` merges path + JSON body → `BuildInput` via serde | clap parses leaf args into `BuildArgs`; `front::run` serde-round-trips into `BuildInput` |
| Execution  | `Handler::from_input` → `handle` → JSON body                   | `cli::front::run` → stdout envelope / text                     |


**The invariant:** the leaf parser serializes to the wire map. `Input` is the only command payload; transports extract, they do not translate. HTTP deserializes `Input` directly from the merged request; argv parses into the leaf's mirror `*Args` struct, which serde-round-trips onto `Input` through one generic bridge in `cli::front::run`. If a shim arm constructs `FooInput { field: x, … }` by hand, the shape is wrong — the arm passes the parsed `*Args` whole.

### Global transport context (not in `Input`)

Mirror HTTP's split between request metadata and body:


| HTTP                        | CLI                                                |
| --------------------------- | -------------------------------------------------- |
| `Client` (owner + provider) | `&Provider` in the shim                            |
| `HeaderMap`                 | (none today)                                       |
| —                           | `cli.format` (`--format` / `SPECIFY_FORMAT`)       |
| —                           | `cli.plan_dir` (`--plan-dir` / `SPECIFY_PLAN_DIR`) |


`cli::front::run` (alias `cli::post` / `cli::get` for naming symmetry with `route::post` / `route::get`) takes `format`, `provider`, and the parsed `*Args` separately. Global flags stay on `Cli`, not on handler `Input`.

### Command registry (one row per command)

HTTP and CLI are two projections of the same registry. Paths are isomorphic; handler and `Input` are identical on both sides.


| CLI path       | HTTP path                  | Kind | Handler              | `Input`       |
| -------------- | -------------------------- | ---- | -------------------- | ------------- |
| `journal emit` | `POST /journal`            | POST | `journal::Emit`      | `EmitInput`   |
| `journal show` | `GET /journal`             | GET  | `journal::Show`      | `ShowInput`   |
| `slice build`  | `POST /slice/{name}/build` | POST | `orchestrate::Build` | `BuildInput`  |
| `plan status`  | `GET /plan/status`         | GET  | `plan::Status`       | `StatusInput` |


The table is the design artifact. It may stay hand-maintained in this RFC and in the shim sources; codegen is an optimization only after the shapes are correct.

## Principles

1. **Start simple, iterate.** Ship the smallest symmetric shim first — explicit WASI exports, hand-written route tables, one line per command — then tighten wire shapes and add tests. Do not front-load macros, codegen, or shared registry machinery before the mirror-serialization invariant holds.
2. `**Input` is the wire contract.** One flat serde DTO per command (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields). It is the shape HTTP deserializes into and the shape the CLI's mirror `*Args` serializes onto. Validation of input *shape* lives in `from_input` — one deserialize path for both transports; project-dependent checks wait for `handle`.
3. **The mirror is dumb and the bridge is generic.** Each routed leaf carries a `*Args` struct in `crates/cli` — field-identical to `Input`, `#[derive(clap::Args, Serialize)]`, kebab-case serialization — and `cli::front::run` converts with one serde round-trip. No per-command conversion code, and `workflow` never links clap: the flag surface and operator help prose stay in `crates/cli`. Mirror parity is guarded by the per-command extraction test, not the compiler; unifying the two shapes (deriving clap on `Input` itself) is the deferred strong-typing iteration.
4. **The `Handler` impl is the command handler.** `from_input`, `handle`, typed `Out<Body>` the transports render (JSON verbatim, text via `Render`).
5. **Handlers are co-located with the code that implements them.** Each domain module in `crates/workflow` owns its family's handlers in a `handlers` submodule beside its kernels. A kernel whose only consumer is one handler goes private in the domain module; shared kernels stay `pub` where they are. There is no separate command-layer crate.
6. **Routing lives in the shims, in the open.** Each shim carries a symmetric `argv.rs` / `http.rs` pair. Both files export a WASI `Guest` impl on wasm (or a process entry on native) and a shared `route` / `router` function holding the match table. Arms name only the handler type `R` and pass the parsed `*Args` — no field mapping. Duplication across shims is deliberate: the compiler checks CLI coverage; HTTP drift is a review catch.
7. **No routing machinery (yet).** No route-table macro, no route/refusal parity data, no parity framework test. A new command is wired by hand into each shim. Macros, codegen, and the strong-typing unification come only after the mirror shapes are stable.

## Where handlers live


| Commands                                                                                | Handlers                           | Beside                                                |
| --------------------------------------------------------------------------------------- | ---------------------------------- | ----------------------------------------------------- |
| `journal emit` / `show`                                                                 | `workflow::journal::handlers`      | the append kernel and the (private) `show` projection |
| `slice create/transition/drop/validate/…` and `archive prune`                           | `workflow::slice::handlers`        | `slice::actions`, `slice::validate`, `slice::model`   |
| `plan create/add/amend/remove/validate/…`                                               | `workflow::change::plan::handlers` | the `plan.yaml` state machine (`plan::core`)          |
| `source survey/extract`, `slice refine/build`, `slice merge run`, `plan author/execute` | `workflow::orchestrate::handlers`  | the orchestrators they drive                          |
| `registry validate/add/remove`                                                          | `workflow::registry::handlers`     | the `registry.yaml` catalog types                     |
| `source resolve` / `target resolve`                                                     | `workflow::adapter::handlers`      | the axis resolvers                                    |
| `init --scaffold-only`                                                                  | `workflow::init::handlers`         | the scaffold kernel                                   |


Shared plumbing every handler uses lives once, in `workflow::handler`: the `Anchor` provider capability (project root + plan-dir override), `Ctx` (config + layout + clock, loaded at the top of each `handle`), `Out` / `Render` / `ReportBody` (output currency), and the handler-layer `Error` with the single taxonomy → HTTP status projection.

`crates/cli` owns the clap grammar (operator UX, `--help`, completions) — including every mirror `*Args` struct and its help prose — plus `try_parse` exit-code passthrough and `front::run`, the generic execution bridge carrying the argv-side serde round-trip from `*Args` onto `Input`. It is a transport front-end library, not the home of any handler; `workflow` never links clap.

## Shim layout

Each shim (wasm guest at `src/`, native `specify-dev` at `harness/native/`) exposes the same two transports through a **symmetric file pair**. `lib.rs` / `main.rs` stay thin — module wiring and mode switch only; no macro-owned exports.

```text
src/                          # wasm guest shim
  lib.rs                      # mod argv; mod http; mod provider;
  provider.rs                 # WIT-backed Provider
  argv.rs                     # struct Cli + wasi:cli/run export + route(cli)
  http.rs                     # struct Http + wasi:http export + router(client)

harness/native/src/           # native shim
  main.rs                     # argv mode vs `serve` mode switch
  provider.rs                 # NativeProvider
  argv.rs                     # run(argv) → same route(cli) as the guest
  http.rs                     # serve() → same router(client) as the guest
```

`**argv.rs**` — not `cli.rs` — avoids a name collision with the workspace `cli` crate (`use cli::parse`, `cli::front::run`, …). Reserve **dispatch** for adapter/host dispatch (metadata runner, Omnia dispatch-by-id); shim filenames use **argv** / **http** for transport routing.

### Symmetric transport files

Both transport files follow the same shape: a **route table function** plus a **transport entry** that calls it. On wasm the entry is an explicit `Guest` impl and `export!` macro — not a hidden macro module.


|              | `argv.rs`                                         | `http.rs`                                                |
| ------------ | ------------------------------------------------- | -------------------------------------------------------- |
| Route table  | `async fn route(cli: cli::Cli) -> Exit`           | `fn router(client: Client<P>) -> Router`                 |
| Wasm export  | `struct Cli; wasip3::cli::command::export!(Cli);` | `struct Http; wasip3::http::service::export!(Http);`     |
| Wasm entry   | `impl Guest for Cli { async fn run() { … } }`     | `impl Guest for Http { async fn handle(request) { … } }` |
| Native entry | `pub async fn run(argv: Vec<String>) -> u8`       | `pub async fn serve(listener, provider)`                 |


Do **not** wire CLI through `omnia_guest::guest!({ command: … })`. That macro hides the `Cli` struct and `Guest::run` impl in a generated `mod command`, which breaks structural symmetry with `http.rs`. Specify already hand-writes HTTP routes; both transports should be equally visible in source.

The WASI export struct is `Cli` (like the Omnia `examples/cli/guest.rs` pattern); the file stays `argv.rs` to avoid colliding with the workspace `cli` crate name. The clap parser root is always `cli::Cli` — qualified, never imported at the same scope as the export struct.

### `lib.rs` (wasm)

```rust
mod argv;
mod http;
mod provider;
```

No `guest!` block. The two `export!` macros in `argv.rs` and `http.rs` are the only WASI surface exports.

## CLI routing

### WASI seam (instance-per-call)

The guest exports `wasi:cli/run` explicitly from `src/argv.rs` — the same structural pattern as `http.rs` and the Omnia CLI example (`examples/cli/guest.rs`). One trigger, one instance, no cross-call state.

```rust
// src/argv.rs (guest shim) — transport entry (symmetric with http.rs)
struct Cli;
wasip3::cli::command::export!(Cli);

impl wasip3::exports::cli::run::Guest for Cli {
    async fn run() -> Result<(), ()> {
        let argv = wasip3::cli::environment::get_arguments();
        let cli = match cli::parse(argv) {
            Ok(cli) => cli,
            Err(exit) => {
                wasip3::cli::exit::exit_with_code(exit.code());
                unreachable!("exit_with_code does not return");
            }
        };
        let exit = route(cli).await;
        if exit.code() == 0 {
            Ok(())
        } else {
            wasip3::cli::exit::exit_with_code(exit.code());
            unreachable!("exit_with_code does not return");
        }
    }
}
```

Use `cli::parse` (`try_parse` under the hood — not `parse()` — so clap's usage-error exit `2` survives the p2/p3 exit seam). Parse errors print usage and call `exit_with_code` before routing.

### Native entry

Native has no WASI `Guest` trait; `harness/native/src/argv.rs` exposes a process entry that calls the same `route(cli)`:

```rust
// harness/native/src/argv.rs — native transport entry
pub async fn run(argv: Vec<String>) -> u8 {
    let cli = match cli::parse(argv) {
        Ok(cli) => cli,
        Err(exit) => return exit.code(),
    };
    route(cli).await.code()
}
```

`main.rs` calls `argv::run(std::env::args().collect()).await` and `process::exit(code)`.

### Target shape: mirror `*Args` structs, one generic bridge

Each routed leaf in the clap grammar carries a mirror `*Args` struct as its payload — not anonymous fields that the shim copies later. The mirror is field-identical to the handler's `Input`, derives `clap::Args` + `Serialize`, and serializes kebab-case so its wire rendering is exactly the map `Input` deserializes from.

```rust
// crates/cli — leaf variants carry the mirror Args struct
#[derive(Debug, Subcommand)]
enum SliceAction {
    Build(BuildArgs),
    Refine(RefineArgs),
    Merge(MergeAction),
    // …
}

#[derive(Debug, clap::Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildArgs {
    /// Slice name.
    pub name: String,
}
```

`Input` stays a serde-only DTO in the owning `workflow` handlers module. The compiler checks each struct internally, but nothing relates `BuildArgs` to `BuildInput` — the per-command argv → `Input` extraction test is the parity guard, which is why it is required per routed command, not optional.

Field-level parsers (`SourceArg`, `FromStr` for closed enums, `value_parser` for `if-exists`) live on the `*Args` struct. A field type with custom argv grammar keeps its `FromStr` for clap and derives `Serialize`; whatever it serializes to *is* the wire form, and `Input`'s `Deserialize` is the single shape validator for both transports. Closed enums ride typed on both sides — a `ValueEnum` on the mirror, the matching serde enum on `Input`, agreeing on kebab-case wire values — never `.to_string()` bridges.

Cross-field desugaring (`plan create --intent`, today's `source_map` / `bindings` / `assigns` helpers in the shim) moves handler-side: the mirror stays a dumb derived `Serialize`, `Input` carries the raw fields (`sources`, `intent`), and `from_input` builds the desugared form. The HTTP body accepts the same raw fields, so the desugaring runs identically on both transports. Complex commands (`plan amend` and its `--sources` / `--add-source` families) put every flag on `AmendArgs`, mirrored one-for-one by `AmendInput`.

### Target shape: one-line routing arms

The shared `route(cli)` function is an exhaustive match that only names `R` and passes the parsed `*Args`. No `FooInput { x: action.x, … }` construction. Wasm and native duplicate this function deliberately.

```rust
// src/argv.rs (and harness/native/src/argv.rs) — route table
async fn route(cli: cli::Cli) -> Exit {
    if let Err(err) = preflight(&cli) {
        return report(cli.format, &err);
    }
    let format = cli.format;
    let provider = &Provider;

    match cli.command {
        Commands::Journal { action } => match action {
            JournalAction::Emit(args) => cli::post::<journal::handlers::Emit, _>(format, provider, args).await,
            JournalAction::Show(args) => cli::get::<journal::handlers::Show, _>(format, provider, args).await,
        },
        Commands::Slice { action } => match action {
            SliceAction::Build(args) => cli::post::<orchestrate::handlers::Build, _>(format, provider, args).await,
            // … one line per leaf
        },
        Commands::Upgrade(_) => refuse("upgrade"),
        Commands::Completions { shell } => completions(shell),
    }
}
```

`cli::post` / `cli::get` are thin aliases over `cli::front::run` documenting read/write intent; they mirror `route::post` / `route::get`. The bridge inside `front::run` is the only argv-side conversion in the codebase:

```rust
// crates/cli/src/front.rs — the one generic argv → Input bridge
pub async fn run<R, P, B>(format: Format, provider: &P, args: impl Serialize) -> Exit
where
    R: Handler<P, Output = Out<B>, Error = workflow::handler::Error>,
    R::Input: DeserializeOwned,
    P: Provider,
    B: Render + Send + Sync,
{
    let input: R::Input = match serde_json::to_value(&args).and_then(serde_json::from_value) {
        Ok(input) => input,
        Err(err) => return report(format, &bridge_error(&err)),
    };
    // … existing body: R::handler(input) → handle → emit / report
}
```

A bridge failure is mirror drift — a programming error, not operator error — surfaced on the standard failure envelope; the per-command extraction tests exist to make it unreachable.

Nesting in the grammar is for **namespacing only** (`Commands` → `SliceAction` → `MergeAction`). The leaf variant always carries its mirror: `MergeRun(MergeRunArgs)`, not `MergeRun { name, … }`.

### Shim policy (not handlers)

Stay at the edges — not mixed into per-command routing:

- **`preflight`** — `adapter::metadata::register` (idempotent `OnceLock`), `check_plan_dir`
- `**refuse**` — provisioning commands with no guest impl (`init` without `--scaffold-only`, `adapters`, `workspace`, `upgrade`, `plugins`)
- `**completions**` — argv-transport sugar; not a `Handler`

### Extraction tests

Beside HTTP's parameter-merge tests in `omnia-guest::api::route`, add one argv → `Input` test per routed command: sample argv in, parse through the grammar, run the bridge, assert the resulting `Input`. Factor the round-trip as a small pub fn in `cli::front` (`fn extract<I: DeserializeOwned>(args: impl Serialize) -> Result<I, Error>`) so tests exercise exactly the conversion `run` performs without a provider. These tests are the mirror-parity guard — the compiler does not relate `*Args` to `Input`, so they are required for every routed command, not just non-trivial parsers. Cover type coercion on GET-side fields too (query strings arrive stringly; `limit=5` → `usize`). Handler tests in `crates/workflow/tests` stay transport-free: `R::handler(input)?.owner("specify").provider(&anchor).handle().await`.

## HTTP routing

`http.rs` is structurally symmetric with `argv.rs`: same registry rows, same handler types, different extractor (`route::get` / `route::post` instead of clap). Both files own an explicit `struct` + `export!` + `Guest` impl on wasm. Routing splits by shim lifetime. Omnia instantiates a fresh wasm instance per HTTP trigger ([architecture.md §"Guest instantiation"](architecture.md#guest-instantiation)), so the guest builds its table inside `handle()` — no `static`, no `LazyLock`. The native `specify-dev serve` process is long-lived and builds once at startup.

### Wasm guest — router per request

The route table lives in `src/http.rs`. `handle()` builds a plain axum `Router` from omnia's generic route constructors (`route::get::<R, P>()` / `route::post::<R, P>()`). Path parameters, query pairs (GET), and JSON body fields (POST) merge into one flat map and deserialize into `R::Input` via serde — the same `Input` struct the argv bridge serializes onto. GET for pure reads, POST for writes and judgment, the noun in the path.

```rust
// src/http.rs (guest shim) — transport entry (symmetric with argv.rs)
struct Http;
wasip3::http::service::export!(Http);

impl wasip3::exports::http::handler::Guest for Http {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        adapter::metadata::register(crate::provider::metadata);
        let client = Client::new("specify").provider(Provider);
        omnia_wasi_http::serve(router(client), request).await
    }
}

fn router(client: Client<Provider>) -> axum::Router {
    Router::new()
        .route("/journal", route::post::<journal::handlers::Emit, Provider>())
        .route("/journal", route::get::<journal::handlers::Show, Provider>())
        .route("/slice/{name}/build", route::post::<orchestrate::handlers::Build, Provider>())
        .route("/plan/status", route::get::<plan::handlers::Status, Provider>())
        // … one line per routed command (same registry rows as argv.rs)
        .with_state(client)
}
```

### Native shim — router at `serve` boot

`harness/native/src/http.rs` owns `serve()` and the route table. `main.rs` delegates `specify-dev serve` here. The listener binds once; the router is built at startup (merged with the `/mcp/<name>` reference shelves), not a `static`.

```rust
// harness/native/src/http.rs — native HTTP transport
pub async fn serve(listener: TcpListener, provider: NativeProvider) -> Result<()> {
    let client = Client::new("specify").provider(provider);
    let router = router(client).merge(mcp::router());
    axum::serve(listener, router).await
}

fn router(client: Client<NativeProvider>) -> axum::Router {
    Router::new()
        .route("/journal", route::post::<journal::handlers::Emit, NativeProvider>())
        // … same paths as src/http.rs
        .with_state(client)
}
```

Host-level prefix → guest routing for adapter MCP shelves lives in the deployment manifest (`omnia.toml` `[[route.http]]`), not in guest `static`s.

## Wire contract (unchanged)

The JSON envelope, kebab-case error discriminants, exit codes, and the taxonomy → status projection (validation/argument → 422, version floor → 426, else → 500) are unchanged; see [cli-architecture.md](../docs/contributing/cli-architecture.md). `Exit` stays in `crates/cli`; `workflow::handler::Error::status` is the only HTTP status table.

One body-shape carve-out: commands whose shim arms desugared cross-field flags (`plan create` / `plan author` and the `source_map` merge) now carry the raw fields (`sources`, `intent`) on the wire, with `from_input` desugaring identically on both transports.

## Adding a command

1. **Define `Input`** in the owning domain module's `handlers` submodule: `#[derive(Serialize, Deserialize)]`, flat kebab-case fields, no clap. Implement `Handler`, `Body`, and `Render`; shape validation and any cross-field desugaring live in `from_input`. Call the domain kernel beside you; if the kernel is new and single-consumer, keep it private.
2. **Mirror the grammar** in `crates/cli`: a `*Args` struct field-identical to `Input` — `#[derive(clap::Args, Serialize)]`, kebab-case serialization, operator help prose on the doc comments, field parsers for any non-scalar argv grammar — and a leaf variant carrying it directly (`Build(BuildArgs)`), not anonymous fields.
3. **Register in both shims**: one argv arm in `argv.rs` — `cli::post::<R, _>(format, provider, args)` (or `cli::get`); one HTTP line in `http.rs` — `.route(…, route::post::<R, P>())` (or `route::get`). Add a registry row to this RFC's table (or keep the shim sources as the living table). Refuse or omit where a shim cannot implement the command.
4. **Test**: handler directly in `crates/workflow/tests`; the argv → `Input` extraction test in `crates/cli/tests` (required — it is the mirror-parity guard).

## Migration posture

The migration is complete: both shims carry the symmetric `argv.rs` / `http.rs` layout with explicit `Cli` + `Http` exports, every leaf variant carries its mirror `*Args` struct, the routing arms are one-liners through the generic bridge, and the per-command extraction tests in `crates/cli/tests/extract.rs` guard mirror parity. Any reintroduction of the transitional shapes (a `guest!` macro, anonymous leaf fields, manual `Input` construction in a shim arm) is debt against this RFC.

### Order of work (simple first, iterate)

1. **Symmetric shim skeleton.** Rename `dispatch.rs` → `argv.rs`, extract `http.rs`, drop `guest!`. Wire explicit `Cli` + `Http` `Guest` impls. Keep the existing match table as-is — mechanical rename only.
2. **One command group at a time.** Give each leaf a mirror `*Args` (`Build(BuildArgs)`); collapse the corresponding argv arms to one-liners through the bridge; delete manual `Input { … }` construction and the per-command conversion helpers (`source_map`, `bindings`, `assigns`) for that group, moving any cross-field desugaring into `from_input`.
3. **Tests per group.** Add one argv → `Input` extraction test per command as its group lands (the mirror-parity guard); handler tests stay transport-free.
4. **Registry hygiene.** Keep the shim route tables and this RFC's registry table aligned as groups land.
5. *Then* iterate to strong typing if mirror drift demonstrably hurts: derive clap on `Input` itself so the mirror disappears and field parity becomes structural. That step moves clap — and the operator help prose — into `workflow`; take it deliberately, only after steps 1–4 are stable.

Do not add new manual mapping arms during migration.

### Do not


| Temptation                                     | Why                                                                 |
| ---------------------------------------------- | ------------------------------------------------------------------- |
| `guest!({ command: … })` for CLI               | Hides the `Cli` export; breaks symmetry with `http.rs`              |
| Runtime command registry with dynamic dispatch | Loses monomorphization, typed errors, and compile-time CLI coverage |
| Permanent `commands/*/convert.rs` shims        | End state is one generic serde bridge; per-command converters are migration-only |
| Derive clap on `workflow` `Input` DTOs (yet)   | That is the deferred strong-typing iteration — it moves clap and help prose into `workflow`; take it deliberately after the mirrors stabilize |
| Macro-generated match before the mirrors are stable | Optimizes boilerplate before the wire shape is correct         |
| Flatten CLI to `specify POST /slice/foo/build` | Breaks the operator CLI contract                                    |


