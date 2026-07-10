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


A **command** is one operator-facing invocation. Each command is implemented by exactly one **handler** with exactly one `**Input`**. **Actions** are the leaves in the clap grammar; **command groups** are the resource prefixes (`slice`, `plan`, `journal`) that namespace them.

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
| Registry   | `.route("/slice/{name}/build", route::post::<Build, P>())`     | `SliceAction::Build(input) => cli::post::<Build, _>(…, input)` |
| Extraction | `route::post` merges path + JSON body → `BuildInput` via serde | clap parses leaf args directly into `BuildInput`               |
| Execution  | `Handler::from_input` → `handle` → JSON body                   | `cli::front::run` → stdout envelope / text                     |


**The invariant:** `Input` is the only command payload. Transports extract; they do not translate. If a shim arm constructs `FooInput { field: x, … }` by hand, the shape is wrong — the parser should already have produced `FooInput`.

### Global transport context (not in `Input`)

Mirror HTTP's split between request metadata and body:


| HTTP                        | CLI                                                |
| --------------------------- | -------------------------------------------------- |
| `Client` (owner + provider) | `&Provider` in the shim                            |
| `HeaderMap`                 | (none today)                                       |
| —                           | `cli.format` (`--format` / `SPECIFY_FORMAT`)       |
| —                           | `cli.plan_dir` (`--plan-dir` / `SPECIFY_PLAN_DIR`) |


`cli::front::run` (alias `cli::post` / `cli::get` for naming symmetry with `route::post` / `route::get`) takes `format`, `provider`, and `input` separately. Global flags stay on `Cli`, not on handler `Input`.

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

1. **Start simple, iterate.** Ship the smallest symmetric shim first — explicit WASI exports, hand-written route tables, one line per command — then tighten wire shapes and add tests. Do not front-load macros, codegen, or shared registry machinery before the `Input` = leaf-parser invariant holds.
2. `**Input` is the wire contract.** One flat serde DTO per command (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields). It is the shape HTTP deserializes into and CLI must parse into. Validation of input *shape* lives in `from_input`; project-dependent checks wait for `handle`.
3. **The `Handler` impl is the command handler.** `from_input`, `handle`, typed `Out<Body>` the transports render (JSON verbatim, text via `Render`).
4. **Handlers are co-located with the code that implements them.** Each domain module in `crates/workflow` owns its family's handlers in a `handlers` submodule beside its kernels. A kernel whose only consumer is one handler goes private in the domain module; shared kernels stay `pub` where they are. There is no separate command-layer crate.
5. **Routing lives in the shims, in the open.** Each shim carries a symmetric `argv.rs` / `http.rs` pair. Both files export a WASI `Guest` impl on wasm (or a process entry on native) and a shared `route` / `router` function holding the match table. Arms name only the handler type `R` and pass a parsed `Input` — no field mapping. Duplication across shims is deliberate: the compiler checks CLI coverage; HTTP drift is a review catch.
6. **No routing machinery (yet).** No route-table macro, no route/refusal parity data, no parity framework test. A new command is wired by hand into each shim. Macros and codegen come only after `Input` and leaf parsers are unified.

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

`crates/cli` owns the clap grammar (operator UX, `--help`, completions), `try_parse` exit-code passthrough, and `front::run` — the generic execution bridge. It is a transport front-end library, not the home of any handler.

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

`**argv.rs**` — not `cli.rs` — avoids a name collision with the workspace `cli` crate (`use cli::parse`, `cli::front::run`, …). Reserve **dispatch** for adapter/host dispatch (describe runner, Omnia dispatch-by-id); shim filenames use **argv** / **http** for transport routing.

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
        adapter::describe::register(crate::provider::describe);
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
    adapter::describe::register(crate::provider::describe);
    let cli = match cli::parse(argv) {
        Ok(cli) => cli,
        Err(exit) => return exit.code(),
    };
    route(cli).await.code()
}
```

`main.rs` calls `argv::run(std::env::args().collect()).await` and `process::exit(code)`.

### Target shape: `Input` is the leaf parser

Each routed leaf in the clap grammar carries the handler's `Input` as its payload — not anonymous fields that the shim copies later.

```rust
// crates/cli — leaf variants name Input directly
#[derive(Debug, Subcommand)]
enum SliceAction {
    Build(BuildInput),
    Refine(RefineInput),
    Merge(MergeAction),
    // …
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// Slice name.
    pub name: String,
}
```

`Input` derives both `Parser` (for argv) and `Deserialize` (for HTTP). Field-level parsers (`SourceArg`, `FromStr` for closed enums, `value_parser` for `if-exists`) live on the wire type, not in the shim. Complex commands (`plan amend` and its `--sources` / `--add-source` families) put every flag on `AmendInput`; HTTP sends the same fields in a POST body.

### Target shape: one-line routing arms

The shared `route(cli)` function is an exhaustive match that only names `R` and passes `input`. No `FooInput { x: action.x, … }` construction. Wasm and native duplicate this function deliberately.

```rust
// src/argv.rs (and harness/native/src/argv.rs) — route table
async fn route(cli: cli::Cli) -> Exit {
    preflight(&cli)?;
    let format = cli.format;
    let provider = &Provider;

    match cli.command {
        Commands::Journal { action } => match action {
            JournalAction::Emit(input) => cli::post::<journal::handlers::Emit, _>(format, provider, input).await,
            JournalAction::Show(input) => cli::get::<journal::handlers::Show, _>(format, provider, input).await,
        },
        Commands::Slice { action } => match action {
            SliceAction::Build(input) => cli::post::<orchestrate::handlers::Build, _>(format, provider, input).await,
            // … one line per leaf
        },
        Commands::Upgrade(_) => refuse("upgrade"),
        Commands::Completions { shell } => completions(shell),
    }
}
```

`cli::post` / `cli::get` are thin aliases over `cli::front::run` documenting read/write intent; they mirror `route::post` / `route::get`.

Nesting in the grammar is for **namespacing only** (`Commands` → `SliceAction` → `MergeAction`). The leaf variant always carries `Input`: `MergeRun(MergeRunInput)`, not `MergeRun { name, … }`.

### Shim policy (not handlers)

Stay at the edges — not mixed into per-command routing:

- **`preflight`** — `adapter::describe::register` (idempotent `OnceLock`), `check_plan_dir`
- `**refuse**` — provisioning commands with no guest impl (`init` without `--scaffold-only`, `adapters`, `workspace`, `upgrade`, `plugins`)
- `**completions**` — argv-transport sugar; not a `Handler`

### Extraction tests

Beside HTTP's parameter-merge tests in `omnia-guest::api::route`, add argv → `Input` tests per command: sample argv in, `Input` out. Handler tests in `crates/workflow/tests` stay transport-free: `R::handler(input)?.owner("specify").provider(&anchor).handle().await`.

## HTTP routing

`http.rs` is structurally symmetric with `argv.rs`: same registry rows, same handler types, different extractor (`route::get` / `route::post` instead of clap). Both files own an explicit `struct` + `export!` + `Guest` impl on wasm. Routing splits by shim lifetime. Omnia instantiates a fresh wasm instance per HTTP trigger ([architecture.md §"Guest instantiation"](architecture.md#guest-instantiation)), so the guest builds its table inside `handle()` — no `static`, no `LazyLock`. The native `specify-dev serve` process is long-lived and builds once at startup.

### Wasm guest — router per request

The route table lives in `src/http.rs`. `handle()` builds a plain axum `Router` from omnia's generic route constructors (`route::get::<R, P>()` / `route::post::<R, P>()`). Path parameters, query pairs (GET), and JSON body fields (POST) merge into one flat map and deserialize into `R::Input` via serde — the same `Input` struct argv parses from. GET for pure reads, POST for writes and judgment, the noun in the path.

```rust
// src/http.rs (guest shim) — transport entry (symmetric with argv.rs)
struct Http;
wasip3::http::service::export!(Http);

impl wasip3::exports::http::handler::Guest for Http {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        adapter::describe::register(crate::provider::describe);
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

## Adding a command

1. **Define `Input`** in the owning domain module's `handlers` submodule: `#[derive(Parser, Serialize, Deserialize)]`, flat kebab-case fields, field parsers for any non-scalar argv grammar. Implement `Handler`, `Body`, and `Render`. Call the domain kernel beside you; if the kernel is new and single-consumer, keep it private.
2. **Wire the grammar** in `crates/cli`: add a leaf variant that carries `Input` directly (`Build(BuildInput)`), not anonymous fields.
3. **Register in both shims**: one argv arm in `argv.rs` — `cli::post::<R, _>(format, provider, input)` (or `cli::get`); one HTTP line in `http.rs` — `.route(…, route::post::<R, P>())` (or `route::get`). Add a registry row to this RFC's table (or keep the shim sources as the living table). Refuse or omit where a shim cannot implement the command.
4. **Test**: handler directly in `crates/workflow/tests`; argv → `Input` extraction in `crates/cli/tests` when the leaf parser is non-trivial.

## Migration posture

The codebase may still carry transitional shim code: `guest!({ command: dispatch::main })` in `lib.rs`, clap leaf variants with anonymous fields, manual `Input` construction in `dispatch.rs` (target name: `argv.rs`), HTTP routing inline in `lib.rs` / `main.rs` (target name: `http.rs`), and `handlers` paths still named `verbs`. That is debt against this RFC, not the target.

### Order of work (simple first, iterate)

1. **Symmetric shim skeleton.** Rename `dispatch.rs` → `argv.rs`, extract `http.rs`, drop `guest!`. Wire explicit `Cli` + `Http` `Guest` impls. Keep the existing match table as-is — mechanical rename only.
2. **One command group at a time.** Merge clap leaves with `Input` (`Build(BuildInput)`); collapse the corresponding argv arms to one-liners; delete manual `Input { … }` construction for that group.
3. **Tests per group.** Add argv → `Input` extraction tests when a leaf parser is non-trivial; handler tests stay transport-free.
4. **Registry hygiene.** Keep the shim route tables and this RFC's registry table aligned as groups land.
5. *Then* consider shared registry codegen if duplication still hurts — only after steps 1–4 are stable.

Do not add new manual mapping arms during migration.

### Do not


| Temptation                                     | Why                                                                 |
| ---------------------------------------------- | ------------------------------------------------------------------- |
| `guest!({ command: … })` for CLI               | Hides the `Cli` export; breaks symmetry with `http.rs`              |
| Runtime command registry with dynamic dispatch | Loses monomorphization, typed errors, and compile-time CLI coverage |
| Permanent `commands/*/convert.rs` shims        | End state is `Input` parses itself; converters are migration-only   |
| Macro-generated match before `Input = Args`    | Optimizes boilerplate before the wire shape is correct              |
| Flatten CLI to `specify POST /slice/foo/build` | Breaks the operator CLI contract                                    |


