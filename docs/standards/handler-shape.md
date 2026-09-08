# Handler shape

The contract every operation obeys: how an operation becomes an `omnia_guest::api::Handler<P, I>` fn in `crates/engine`, how paths anchor at the deployed preopen layout, how typed outputs stay `Serialize`-only DTOs, and how the command grammar (`crates/cli`) decodes them for omnia's command façade to dispatch and project.

## Operations (`emery_engine::specify`, `emery_engine::show`)

Every operation is one `pub async fn <verb>(input: I, context: Context<P>) -> Result<Body, omnia_guest::Error>`, a `Handler<P, I>` through omnia's blanket impl over every fn of that shape (there is no proc-macro; a mis-shaped fn is reported by rustc at the route or `Client::call` site). The fn is bound at the call site — `client.call(specify, input, &metadata)`, `http::post(specify)` — never named by a type parameter:

- **`I`** is a flat, transport-neutral serde DTO (`Serialize`/`Deserialize`, `#[serde(rename_all = "kebab-case")]`): `Specify { bindings: Vec<SourceBinding> }`, `Show { document: Document }`. It carries no clap derives, no flag names, and no carrier knowledge — the same type deserializes from an HTTP body (`omnia_guest::api::http::post(specify)`) as is built by the CLI façade.
- **The fn body** validates its input against the rules every transport must get (the private `validate` in `emery_engine::specify`: a non-empty list, kebab-case unique keys, `digest`/`registry` gating, preopen-relative roots; the `adapter` field is the typed `AdapterRef`, so a malformed selector refuses at the DTO boundary), anchors at the deployed layout, delegates to the deterministic kernel over `context.provider()`, and returns the typed body.
- **`Result<_, omnia_guest::Error>`** — handlers return Omnia's protocol error; do not introduce a house error type.

`Context<P>` is owned by the call: `owner()` and `provider()` are accessors, `metadata` is the public transport-neutral field, and `Context::new(owner, provider, metadata)` builds one without a `Client` when a handler is exercised directly.

Deterministic handlers bind only the capabilities they use unless their kernel issues model judgments, in which case they additionally bind `Model`. Paths and adapter dispatch are not provider capabilities: paths are fixed constants relative to named preopens, and adapter operations ride the `emery:adapter/source` WIT imports directly.

```rust
// GOOD — deterministic kernel behind the handler fn.
pub async fn frob<P>(input: FrobInput, _context: Context<P>) -> Result<FrobBody, omnia_guest::Error> {
    kernel(input)
}

fn kernel(input: FrobInput) -> Result<FrobBody, omnia_guest::Error> {
    let outcome = some_crate::do_work(&input)?;
    Ok(FrobBody::from(&outcome))
}
```

Handler fns live beside their domain kernels, in the module named for the verb (`specify::specify`, `show::show`).

## The deployed layout (C5)

Handlers anchor at the `.` preopen inside the handler fn: paths are constants relative to the project-root mount (the invocation directory natively; `emery_engine::preopen_path` normalizes operator paths inside it and speaks paths, never flag names), and engine storage is named by fixed key/container formulas over the provider's storage capabilities. There is no project record and no project-level version requirement — a run's inputs arrive on the invocation, and there is nothing to be "inside". Handlers never derive paths any other way — no environment reads, no ancestor walks, no CWD dependence; native tests script the storage capabilities in memory instead of chdir-ing into a tempdir.

## Output: `Serialize`-only bodies

Handlers never write to stdout. Each returns a typed body (`SpecifyBody`, `ShowBody`) implementing `Serialize` and nothing presentational: no `Display`, no terminal style. Text-mode rendering is the CLI's concern — one render fn per body in `crates/cli/src/text.rs` (`Fn(&Body, &mut dyn fmt::Write) -> fmt::Result`), handed to omnia's `Command::call` as the verb's text form; `omnia_guest::api::Format::encode` encodes either mode into memory infallibly (text through the render fn, JSON through `Serialize`). The style those render fns follow is [CLI output shapes](../reference/cli-output-shapes.md).

## Errors and their projections

Handlers return `omnia_guest::Error` with transport-neutral descriptions: name the path, the adapter, or the rule — never a flag, a verb, or "the CLI". Omnia's command façade (`omnia_guest::api::command`) owns the 1:1 variant → exit projection (`Error::exit_code`) and builds the `Failure` envelope from `code()` / `description()` (the same `api::ErrorBody` HTTP emits); the envelope is encoded in the selected format on stderr, so success and failure share one rendering path. Its text form is `error[<code>]: <message>` plus an optional `hint:` line, so the `error` discriminant is grep-stable in both formats and descriptions never repeat it. Flag-vocabulary recovery text is the `hint` table in `crates/cli/src/lib.rs`, keyed by the discriminant and attached through `Command::hints`. There is no exit table in this repository — `omnia_guest::Error::exit_code` is the only one. Do not introduce a house error type or a report-carrying failure wrapper until a gate verb needs one.

## Exit codes

The Omnia 1:1 exit map is fixed:

| Code | Name            | When                                                                                                                                                          |
| ---- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`  | Command succeeded                                                                                                                                             |
| 1    | `BadRequest`    | Operator or input refusal. The `error` field is `specify-source-required`, `unsupported-version`, or the Omnia default `bad_request`.                          |
| 2    | `NotFound`      | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`.                                                                  |
| 3    | `ServerError`   | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error`.                                              |
| 4    | `BadGateway`    | Upstream or model failure. The `error` field is the Omnia default `bad_gateway`.                                                                               |
| 64   | `USAGE_EXIT`    | Clap usage error (unknown verb or flag, missing argument): clap's own text on stderr, no envelope. `EX_USAGE`, so exit 2 always means a `NotFound` envelope.   |

Omnia default codes are snake_case (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The three recovery discriminants stay kebab-case so skills can branch on them.

`omnia_guest::Error::exit_code` maps the variants and is the single source of truth; omnia's `Command` projector applies it to every terminal operation failure, and `omnia_guest::api::command::USAGE_EXIT` is the usage status. Do not invent new exit codes.

## The command grammar (`emery-cli`)

`crates/cli` is what is Emery's about the CLI surface: the clap `App` type and per-verb `*Args` types, the binding carriers, the per-body render fns, and the hint table. The projection itself is omnia's command façade — `omnia_guest::api::command::{parse, Command, Response, Failure, completions}` — which is a transport over the engine in exactly the sense omnia's `api::http` overlay is: decode → `Client::call(handler, input, &Metadata)` → encode. There is no HTTP surface shipped: the engine binds no listener, so C3 (no unauthenticated HTTP ingress) is satisfied by absence rather than a refusal router — but the engine permits the overlay unchanged, and that is the litmus for the boundary.

The grammar lives on façade-side `SpecifyArgs` / `ShowArgs` (`clap::Args`, `#[arg]` parsers, `--help` prose) and the closed `DocumentArg` (`clap::ValueEnum`). Each decodes into its engine input by **exhaustive struct literal** (`Specify { bindings }`, `Show { document }`) and an exhaustive `From<DocumentArg> for Document`, so a new engine field or variant is a façade compile error — the same drift guarantee the old fused design had, with one direction of dependency. Global flags (`--format`) stay on `App`. Layering rule: `cli` imports engine inputs, bodies, the binding DTO, `AdapterRef`, `preopen_path`, and `omnia_guest::Error` — never domain kernels.

Decoders (`crates/cli/src/bindings.rs`: argv positionals + `--description`, the `--config` `emery.toml` carrier, project-root discovery) return `omnia_guest::Error`, not clap errors, so their refusals ride the same envelope and exit map as handler failures (`--config` mixed with argv bindings is `bad_request` → 1; an unreadable explicit `--config` is `server_error` → 3). Do not express those rules as clap `conflicts_with` / `value_parser` — that would move them to the usage exit (64) and out of the envelope.

## Dispatch contract (`emery_cli::run`)

`emery_cli::run(provider, argv)` is the whole entry: `parse::<App>(argv)` classifies clap's outcomes (`Parsed::Display` — `--help`, `--version` — is stdout at exit 0; `Parsed::Usage` is `Response::usage` at `USAGE_EXIT`), then each verb decodes into its engine input and runs through `Command::new(&Client::new(NAME, provider), &Metadata::from_env("EMERY"), format).hints(hint).call(handler, decode, render)`; `completions` short-circuits to `command::completions::<App>`. The buffered `Response` comes back. Wire-contract suites call the same `run` and assert on the buffered channels.

On wasm, the guest (`src/lib.rs`) exports `wasi:cli/run` through `omnia_guest::command!(dispatch)`; `dispatch` runs `emery_cli::run` over its provider and returns the `Response` itself; omnia's `Response` implements `IntoExit`, so the macro writes both channels (a `BrokenPipe` keeps the response's own exit, any other refused channel exits 3) and hands the exit status to `execute_wasi` — the WASI last mile that initializes and flushes guest telemetry and exits with the exact status. Every path runs the same grammar and projector.

Target discipline per verb arm:

1. Decode the verb's `*Args` into its engine input (`SpecifyArgs::decode`, `ShowArgs::decode`) as the `decode` closure of `Command::call`, so a decoder refusal rides the envelope.
2. Name the verb's handler fn and its render fn (`command.call(specify, || grammar.decode(), text::specify)`, `command.call(show, || Ok(grammar.decode()), text::show)`); the projector runs `Client::call` and encodes success or failure. Completions remain synthetic grammar behaviour and never reach a handler.

Never put domain logic in `cli`. Binding rules that every transport must enforce (key grammar and uniqueness, pin gating, preopen roots, the empty-list refusal) live in `emery_engine::specify` (its private `validate`, run by the `specify` fn); only the carriers' own grammar (the `<adapter>=<text>` split, the TOML schema and its reserved keys, the exclusivity rule) lives in the façade. For the layering this enforces see [architecture.md §"Workspace layout"](./architecture.md#workspace-layout).

## Gotcha — the only version requirement is per adapter

There is no project-level `emery` version requirement: the adapter's minimum `emery-version` (from `metadata`, enforced during resolve as `unsupported-version`) is a `BadRequest`. Don't reintroduce a version check at a route or handler site.
