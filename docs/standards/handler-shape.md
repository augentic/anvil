# Handler shape

The contract every command handler obeys: how a command becomes an `omnia_guest::api::Handler<P>` in `crates/workflow`, how `Ctx` is constructed from the provider's `Anchor`, how output flows through the typed `Out<Body>` wrapper and its `Render` impl, which exit code a terminal `Error` maps to, and what the per-shim route tables in `argv.rs` / `http.rs` are allowed to do. Vocabulary and routing layout: [handler-routing.md](../../rfcs/handler-routing.md).

## The handler layer (`workflow::handler`)

Every command is implemented by one request type implementing `omnia_guest::api::Handler<P>`:

- **`Input`** is a flat serde DTO (`#[serde(rename_all = "kebab-case")]`, `#[serde(default)]` on optional fields) — the wire shape shared by every transport. It also derives `Parser` for argv; the HTTP transport deserialises the same fields from path/query/body. See [handler-routing.md §"Transport model"](../../rfcs/handler-routing.md#transport-model).
- **`from_input`** carries input-shape validation only (e.g. `slice transition` refusing `merged`). Everything project-dependent waits for `handle`.
- **`handle(self, ctx: Context<'_, P>)`** is the handler body: load `Ctx` from `ctx.provider`, delegate the deterministic work to a workspace kernel, and return `Reply::ok(Out(Body))`.
- **`type Error = workflow::handler::Error`** — the workspace taxonomy plus the report-carrying `Error::Report` shape (below).

Deterministic handlers bound `P: Anchor` only. The orchestration handlers (`orchestrate::handlers`) additionally bound the seams they drive: `P: Anchor + Model + SourceSeam + TargetSeam` (or the subset they need), so the same impl serves the wasm guest, the native dev shim, and tests against scripted seams.

```rust
// GOOD — default shape
impl<P: Anchor> Handler<P> for Frob {
    type Error = crate::handler::Error;
    type Input = FrobInput;
    type Output = Out<FrobBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let outcome = some_crate::do_work(cx.layout(), &self.input)?;
        Ok(Reply::ok(Out(FrobBody::from(&outcome))))
    }
}
```

Handlers live in each domain module's `handlers` submodule beside its kernels (see [handler-routing.md §"Where handlers live"](../../rfcs/handler-routing.md#where-handlers-live)).

## Ctx construction and the Anchor

Handlers construct `workflow::handler::Ctx` inside `handle` via `Ctx::load(ctx.provider)`. The project location comes from the provider's `workflow::handler::Anchor` — the guest provider answers `"."` (the project-root mount preopen); a native provider answers its configured root. Handlers never read the process CWD themselves. `Ctx` exposes the resolved project dir, config, layout, and the single handler-boundary `now()` clock read; everything else flows through workspace crates. `Layout<'a>` lives on `Ctx` rather than at call sites so path helpers stay anchored in `workflow` — see [architecture.md §"Layout boundary"](./architecture.md#layout-boundary).

The scaffold command (`init --scaffold-only`) is the one handler that runs before a project exists: it anchors at the raw `Anchor::project_root` instead of loading `Ctx`.

## Output: `Out<Body>` and `Render`

Handlers never write to stdout. Each handler returns a typed `*Body` wrapped in `workflow::handler::Out` — the local wrapper that gives every body the HTTP transport's `IntoBody` (JSON) encoding without orphan-rule friction. The text rendering lives on the body itself via the `workflow::handler::Render` trait, colocated with the `Serialize` derive so the response shape stays in a single block of code. The CLI front-end calls `Render` for `--format text` and serde for `--format json`; the HTTP transport serialises the same body as JSON and never calls `Render`.

### Gate handlers ride the error

Check surfaces that gate on findings — `slice validate`, `plan validate` — return `Out<ReportBody>` on success and `Error::Report { body, source }` on failure. The transport renders the `ReportBody` (the `DiagnosticReport` wire envelope plus the per-finding text row hook) on **stdout**, then the payload-free `source` error on **stderr** — findings on the success channel, the discriminant and exit 2 on the failure channel, exactly the two-channel contract skills consume. `Error::Validation` stays `{ code, detail }` with no findings payload. The blocking decision uses the uniform predicate (`kind == violation && severity ∈ {critical, important}`); `kind: review` diagnostics surface but never block.

## Errors and their projections

`workflow::handler::Error` wraps the workspace `error::Error` taxonomy (`Error::Core`) and adds `Error::Report`. It carries the **single** taxonomy → HTTP status projection (`Error::status()`: validation/argument → 422, version floor → 426, everything else → 500) and the `From<workflow::handler::Error> for omnia_guest::Error` conversion the HTTP transport consumes. `Exit` stays in `crates/cli` — there is no second exit table.

## Exit codes

The four-slot CLI exit-code table is fixed:

| Code | Name | When |
|---|---|---|
| 0 | `EXIT_SUCCESS` | Command succeeded |
| 1 | `EXIT_GENERIC_FAILURE` | Default `Error` → exit 1 |
| 2 | `EXIT_VALIDATION_FAILED` | `Error::Validation`, undeclared/over-permissioned tool, `Error::Argument` |
| 3 | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` (`specify-version-too-old` in JSON) |

`Exit::from(&Error)` in [`crates/cli/src/output.rs`](../../crates/cli/src/output.rs) is the single source of truth. `cli::front::run` routes every terminal error through `output::report`, which calls `Exit::from`. Do not invent new exit codes. `Exit::Code(u8)` is reserved for the guest leg's exit-code passthrough.

## The CLI front-end (`crates/cli`)

`crates/cli` is a pure front-end library: the clap grammar (`cli::Cli` / `cli::Commands`), the shared `cli::parse` entry point (`try_parse` with exit-code passthrough), the output envelopes (`output::{Format, emit}`), the exit contract (`output::{Exit, report}`), and `front::run` (aliases `cli::post` / `cli::get`) — the generic body that drives a `Handler` against a provider and renders its `Reply` or failure.

`crates/cli/src/cli.rs` declares the clap derive surface. Leaf variants carry handler `Input` directly (`Build(BuildInput)`), not anonymous fields the shim maps later. Field parsers (`SourceArg`, closed enums, repeatable flags) live on `Input`. Global flags (`--format`, `--plan-dir`) stay on `Cli`, not on `Input`. See [handler-routing.md §"CLI routing"](../../rfcs/handler-routing.md#cli-routing).

## The HTTP route table (`http.rs`)

Each shim owns a hand-written axum `Router` in `http.rs`. The wasm guest builds it inside `handle()` (instance-per-call — no `static`); `specify-dev serve` builds once at startup in `harness/native/src/http.rs`. Both use omnia's generic route constructors (`route::get::<R, P>()` / `route::post::<R, P>()`), with the `Client` (owner + provider) as router state. GET for pure reads (path + query args), POST for writes and judgment (JSON bodies), the noun in the path (`POST /slice/{name}/build`). One line per routed command; parity between CLI and HTTP comes from both transports driving the same `Handler` impls, not from shared table code. See [handler-routing.md §"HTTP routing"](../../rfcs/handler-routing.md#http-routing).

## Dispatch contract (`argv.rs`)

Each shim owns an exhaustive argv route table in `argv.rs` — deliberately duplicated per shim (the wasm guest's `src/argv.rs`, the native `specify-dev` binary's `harness/native/src/argv.rs`) so the compiler checks each shim's coverage of the grammar. HTTP routing lives symmetrically in `http.rs`.

On wasm, `argv.rs` exports `wasi:cli/run` explicitly — `struct Cli`, `wasip3::cli::command::export!(Cli)`, `impl Guest for Cli` — matching `http.rs`'s `struct Http` + `Guest::handle` and the Omnia `examples/cli/guest.rs` pattern. Do not hide CLI behind `guest!({ command: … })`. The clap parser root is `cli::Cli` (qualified); native `argv.rs` exposes `run(argv) -> u8` that calls the same `route(cli: cli::Cli)` function.

Target discipline per leaf arm:

1. `preflight` — `adapter::metadata::register`, `check_plan_dir`
2. `cli::parse` parses argv → `Commands` enum (leaf variants already hold `Input`)
3. `cli::post::<R, _, _>` or `cli::get::<R, _, _>(format, provider, input)` — names only `R`, passes parsed `input`
4. Shim policy at the edges: `refuse` for provisioning commands, `completions` for shell scripts

Never put domain logic in `cli` or a shim's route match. Manual `Input { … }` construction in an `argv.rs` arm is a shape defect. For the crate dependency direction this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Handler-shape notes

`source resolve <name>` and `target resolve <value>` never load a `Ctx`, because adapter resolution is read-only and runs before any project mutation; they anchor the default project dir on the provider's `Anchor`. The two axes share one input shape; the axis is the request type. The target axis peels an opaque `@version` suffix (per the workflow contract §CLI surface).

`plan amend` extends the canonical `with_state::<Plan, _, _>(...)` handler shape with the three `--sources` flag families: `--sources <binding>...` (wholesale replace), `--add-source <binding>` (repeatable), `--remove-source <key>` (repeatable). The handler routes `--add-source` / `--remove-source` *after* the wholesale `Plan::amend(name, patch)` call so wholesale replacement plus targeted edits compose cleanly in a single invocation. The `--divergence` flag accepts only `likely | accepted | rejected` from the wire and emits a `plan.amend.divergence` journal event when (and only when) the field flips.

`plan transition <name> <target>` is one handler that dispatches on the operands: `<plan-name> approved` is the Gate 1 stamp and emits a `plan.transition.approved` journal event; `<entry-name> done` is the per-entry close (`/spec:merge` is the canonical caller). Anything else is an `Error::Argument` (exit 2). The journal append runs *after* `with_state` returns so the plan write and the journal append cannot interleave on failure.

## Gotcha — `specify init` and the version floor

`specify init` bypasses the `specify` version floor check (the file doesn't exist yet); every other project-aware command inherits it for free via `ProjectConfig::load`. Don't reimplement the floor check at a subcommand site.
