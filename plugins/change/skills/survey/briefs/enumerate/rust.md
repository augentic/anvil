---
id: enumerate
description: Per-language enumeration prompt for /change:survey driving Rust legacy-code surface discovery.
language: rust
---

# Rust enumeration for `/change:survey`

This brief drives the LLM step of `/change:survey` for `legacy-code` sources whose detected `language` is `rust`. The LLM produces a candidate `surfaces.json` matching the schema below; `specify change survey` validates, canonicalizes, and writes it.

Rust source uses macro-heavy idioms — Rocket attribute macros, Actix attribute macros, `clap` derive macros. The brief treats the macro invocation line as the declaration site (e.g. `#[get("/users")]` on its own line, the `Router::route("…", get(handler))` chain, the `web::resource("…").route(…)` chain).

## Scope

Frameworks and crates covered in v1:

- **Axum** — `axum::Router::route("<path>", method(handler))` chains and the `routing::{get,post,put,delete,patch}` family count as one `http-route` surface per `(method, path)`.
- **Actix-web** — `actix_web::web::resource("<path>").route(web::<method>().to(handler))` and the `#[get("/...")]` / `#[post("/...")]` attribute macros count as one `http-route` surface per registration.
- **Rocket** — `#[get("/...")]`, `#[post("/...")]`, `#[put("/...")]`, `#[delete("/...")]`, `#[patch("/...")]` attribute macros each count as one `http-route` surface.
- **lapin (RabbitMQ / AMQP)** — `Channel::basic_publish(...)` is `message-pub`; `Channel::basic_consume(...)` (typically followed by `Consumer::next(...)`) is `message-sub`.
- **rdkafka (Kafka)** — `FutureProducer::send(...)` / `BaseProducer::send(...)` call sites are `message-pub`; `StreamConsumer::recv(...)` / `BaseConsumer::poll(...)` loops are `message-sub`.
- **async-nats (NATS)** — `Client::publish(...)` is `message-pub`; `Client::subscribe(...)` is `message-sub`.
- **tokio scheduling** — `tokio::time::interval(...)` driving a periodic loop, `tokio_cron_scheduler::JobScheduler::add(...)`, and similar job-runner crates are `scheduled-job`.
- **clap** — each `#[derive(Subcommand)]` variant (or `#[command(...)]` leaf) is one `cli-command` surface.
- **reqwest / typed SDK clients** — call sites that issue an outbound HTTP / RPC request to a non-local host (`reqwest::Client::{get,post,put,delete,patch}`, typed-SDK methods such as `aws_sdk_s3::Client::get_object`) are `external-call-out`.

Stacks outside this set fail closed in `/change:survey` per the brief-resolution policy.

## Schema

Every surface emitted MUST match the closed schema in `specify-cli/schemas/surfaces.schema.json`. Repeated verbatim:

**Closed `kind` enum** (unknown values fail validation):

- `http-route`
- `message-pub`
- `message-sub`
- `ws-handler`
- `scheduled-job`
- `cli-command`
- `ui-route`
- `external-call-out`

**`Surface` field set** (all required, `additionalProperties: false`):

| Field         | Type            | Notes                                                                                |
| ------------- | --------------- | ------------------------------------------------------------------------------------ |
| `id`          | string          | Stable, kebab-case, unique within this `surfaces.json`.                              |
| `kind`        | string          | One of the closed enum above.                                                        |
| `identifier`  | string          | Legacy spelling of the surface (route, topic, command, etc.).                        |
| `handler`     | string          | `<file>:<function>` reference to the implementation entry.                           |
| `touches`     | array of string | Source files reached from the handler, alphabetical, relative to source root.        |
| `declared-at` | array of string | Non-empty list of `<file>` or `<file>:<line>` registration sites, alphabetical.      |

**Path-under-source-root rule.** Every entry in `touches[]` and `declared-at[]` MUST:

- Be a relative path (no leading `/`, no Windows drive letter).
- Contain no `..` segments.
- Resolve to a file under the source root.
- Live outside `target/`, `vendor/`, `.cargo/`, and any generated `OUT_DIR` outputs (compile-time `include!(concat!(env!("OUT_DIR"), …))` sources are not real files in the source tree).

`specify change survey` enforces these mechanically. A violation exits `surfaces-touches-out-of-tree`.

## Worked examples

Each example shows a Rust input snippet, the framework signature that fired, and the resulting `Surface` JSON block.

### `http-route` — Axum `Router::route`

Input snippet (`src/api.rs`):

```rust
use axum::{Router, routing::get};

pub async fn list_users() -> &'static str {
    "users"
}

pub fn build() -> Router {
    Router::new().route("/users", get(list_users))
}
```

Signature: `Router::route("/users", get(list_users))` on line 9.

```json
{
  "id": "http-get-users",
  "kind": "http-route",
  "identifier": "GET /users",
  "handler": "src/api.rs:list_users",
  "touches": ["src/api.rs"],
  "declared-at": ["src/api.rs:9"]
}
```

### `http-route` — Actix-web `web::resource`

Input snippet (`src/users/routes.rs`):

```rust
use actix_web::{web, HttpResponse};

pub async fn register(form: web::Json<RegisterForm>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users").route(web::post().to(register)),
    );
}
```

Signature: `web::resource("/users").route(web::post().to(register))` on line 9.

```json
{
  "id": "http-post-users",
  "kind": "http-route",
  "identifier": "POST /users",
  "handler": "src/users/routes.rs:register",
  "touches": ["src/users/routes.rs"],
  "declared-at": ["src/users/routes.rs:9"]
}
```

### `http-route` — Rocket attribute macro

Input snippet (`src/routes.rs`):

```rust
use rocket::get;

#[get("/users")]
pub fn list_users() -> &'static str {
    "users"
}
```

Signature: `#[get("/users")]` on line 3.

```json
{
  "id": "http-get-users",
  "kind": "http-route",
  "identifier": "GET /users",
  "handler": "src/routes.rs:list_users",
  "touches": ["src/routes.rs"],
  "declared-at": ["src/routes.rs:3"]
}
```

### `message-pub` — lapin `basic_publish`

Input snippet (`src/events.rs`):

```rust
use lapin::{Channel, BasicProperties, options::BasicPublishOptions};

pub async fn publish_user_created(ch: &Channel, payload: &[u8]) -> lapin::Result<()> {
    ch.basic_publish(
        "events",
        "user.created",
        BasicPublishOptions::default(),
        payload,
        BasicProperties::default(),
    )
    .await?
    .await?;
    Ok(())
}
```

Signature: `Channel::basic_publish(exchange="events", routing_key="user.created", …)` starting on line 4.

```json
{
  "id": "message-pub-user-created",
  "kind": "message-pub",
  "identifier": "events:user.created",
  "handler": "src/events.rs:publish_user_created",
  "touches": ["src/events.rs"],
  "declared-at": ["src/events.rs:4"]
}
```

### `message-sub` — lapin `basic_consume`

Input snippet (`src/consumers/user_created.rs`):

```rust
use lapin::{Channel, options::BasicConsumeOptions, types::FieldTable};
use futures_lite::StreamExt;

pub async fn consume_user_created(ch: &Channel) -> lapin::Result<()> {
    let mut consumer = ch.basic_consume(
        "user-created",
        "user-service",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await?;
    while let Some(_delivery) = consumer.next().await {
        // handle
    }
    Ok(())
}
```

Signature: `Channel::basic_consume(queue="user-created", consumer_tag="user-service", …)` starting on line 5, followed by a `Consumer::next()` loop.

```json
{
  "id": "message-sub-user-created",
  "kind": "message-sub",
  "identifier": "user-created",
  "handler": "src/consumers/user_created.rs:consume_user_created",
  "touches": ["src/consumers/user_created.rs"],
  "declared-at": ["src/consumers/user_created.rs:5"]
}
```

### `message-sub` — async-nats `Client::subscribe`

Input snippet (`src/nats/orders.rs`):

```rust
use async_nats::Client;
use futures::StreamExt;

pub async fn subscribe_orders(client: Client) -> Result<(), async_nats::Error> {
    let mut sub = client.subscribe("orders.created").await?;
    while let Some(_msg) = sub.next().await {
        // handle
    }
    Ok(())
}
```

Signature: `Client::subscribe("orders.created")` on line 5.

```json
{
  "id": "message-sub-orders-created",
  "kind": "message-sub",
  "identifier": "orders.created",
  "handler": "src/nats/orders.rs:subscribe_orders",
  "touches": ["src/nats/orders.rs"],
  "declared-at": ["src/nats/orders.rs:5"]
}
```

### `scheduled-job` — `tokio::time::interval`

Input snippet (`src/jobs/sweep.rs`):

```rust
use std::time::Duration;
use tokio::time;

pub async fn run_sweep_job() {
    let mut ticker = time::interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        sweep().await;
    }
}

async fn sweep() {
    // periodic work
}
```

Signature: `tokio::time::interval(Duration::from_secs(60))` driving a `loop { ticker.tick().await; … }` on line 5.

```json
{
  "id": "scheduled-job-sweep",
  "kind": "scheduled-job",
  "identifier": "every 60s",
  "handler": "src/jobs/sweep.rs:run_sweep_job",
  "touches": ["src/jobs/sweep.rs"],
  "declared-at": ["src/jobs/sweep.rs:5"]
}
```

### `cli-command` — `clap` derive

Input snippet (`src/main.rs`):

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Migrate {
        #[arg(long)]
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Migrate { target } => run_migrate(target),
    }
}

fn run_migrate(_target: String) {
    // dispatch
}
```

Signature: `Commands::Migrate` variant declared on line 11; dispatched to `run_migrate` on line 20.

```json
{
  "id": "cli-migrate",
  "kind": "cli-command",
  "identifier": "migrate",
  "handler": "src/main.rs:run_migrate",
  "touches": ["src/main.rs"],
  "declared-at": ["src/main.rs:11"]
}
```

### `external-call-out` — `reqwest::Client`

Input snippet (`src/billing/client.rs`):

```rust
use reqwest::Client;

pub async fn fetch_invoice(client: &Client, id: &str) -> reqwest::Result<Invoice> {
    client
        .get(format!("https://billing.example.com/invoices/{id}"))
        .send()
        .await?
        .json::<Invoice>()
        .await
}

pub struct Invoice;
```

Signature: `reqwest::Client::get("https://billing.example.com/invoices/…")` on line 5.

```json
{
  "id": "external-call-billing-invoices",
  "kind": "external-call-out",
  "identifier": "GET https://billing.example.com/invoices/{id}",
  "handler": "src/billing/client.rs:fetch_invoice",
  "touches": ["src/billing/client.rs"],
  "declared-at": ["src/billing/client.rs:5"]
}
```

## `handler` resolution

`handler` is `<file>:<function>` and points at the implementation entry for the surface, NOT the registration site. Pick it as follows:

- **`http-route` (Axum `Router::route`).** The function passed to the method extractor: `route("/users", get(list_users))` → `<file>:list_users` in the file that declares `list_users`. If the route is mounted with an inline closure — `route("/users", get(|| async { … }))` — there is no named function. Use the containing function with a synthetic suffix `<file>:<containing-fn>-handler-<n>` where `<n>` is the 1-based ordinal of the closure-handler within `<containing-fn>` (closure-handlers only; ordinary closures are not counted). The synthetic suffix keeps `handler` stable across reruns because the source position is fixed.
- **`http-route` (Actix `web::resource(…).route(web::post().to(register))`).** The function passed to `.to(...)`. Inline closures use the same `<file>:<containing-fn>-handler-<n>` rule.
- **`http-route` (Rocket `#[get("/…")]` and friends).** The function the macro decorates, in the file that contains the macro line.
- **`message-pub`.** The function whose body contains the publish call (`basic_publish`, `Producer::send`, `Client::publish`). If the publish lives at module scope (rare), use `<file>:<module-path>` with the module path resolved from the file's `mod` chain.
- **`message-sub`.** The function whose body contains the subscribe / consume call AND the message-handling loop. If the loop hands each message to a separate handler function, prefer the loop-owning function — that is the entry point, the per-message handler shows up as a `touches[]` file via the module graph walk.
- **`scheduled-job`.** The function that constructs the interval / scheduler AND drives the tick loop. For `tokio_cron_scheduler::JobScheduler::add(...)` invoked with a closure, fall back to `<file>:<containing-fn>-handler-<n>`.
- **`cli-command`.** The function the dispatcher routes the variant to, e.g. `Commands::Migrate { target } => run_migrate(target)` → `<file>:run_migrate` in the file that declares `run_migrate`. When the dispatcher inlines the body (`Commands::Migrate { target } => { /* … */ }`), use the synthetic-suffix form against the dispatch function.
- **`external-call-out`.** The function whose body issues the request (`reqwest::Client::get(...)`, typed-SDK call). The `declared-at` line is the call site.

Reuse the synthetic-suffix form rather than emitting per-closure file paths so the handler string remains stable and unique across reruns even when closures are reordered.

## `touches[]` resolution

`touches[]` enumerates source files the handler reaches via static, file-level reach analysis. Walk the module graph with these rules:

1. **Start at the handler module.** The file containing `handler`'s defining function is always in `touches[]`.
2. **Walk `mod` declarations.** From the handler module, follow inline `mod foo { … }` and file-backed `mod foo;` (resolving to `foo.rs` or `foo/mod.rs`) to reach types, traits, and helpers the handler uses. Stop at any `#[cfg(test)]`-gated module.
3. **Walk qualified-path imports.** For every `use crate::a::b::C;` or `use super::sibling::*;` referenced from the handler module, resolve the path to a file in this crate and add it to `touches[]`. Macro re-exports (`pub use crate::a::*`) also count when they point at real source files in the same crate.
4. **Include intra-crate `pub use` re-exports.** A `pub use crate::a::B;` line in a `lib.rs` / `mod.rs` typically points at a real source file; treat the re-export's target file as reachable.
5. **Stop at crate boundaries.** External crates are dependencies, not source. `use serde::Deserialize;`, `use tokio::time;`, `use lapin::Channel;` are all stops — do NOT chase them into `~/.cargo/registry/` or `vendor/`. Only the legacy source root (the project's own `crates/*/src/` plus binary `src/`) is in scope.
6. **Exclude generated files.** Files emitted by `build.rs` into `OUT_DIR` and `include!`'d from source are not real source files; skip them.

Sort the resulting list alphabetically, relative to the source root, and emit it as `touches[]`.

## Anti-patterns

The brief MUST NOT emit:

- **Dead handlers** — surfaces whose framework registration line is `#[cfg(test)]`-gated, or whose registration is unreachable behind feature flags the source disables in production. `cargo` features that default-off and are never enabled by `[features]` defaults or `Cargo.lock` activation count as disabled.
- **Test code** — modules under `#[cfg(test)]`, files under `tests/` (Cargo's integration-test root), files under `examples/`, files under `benches/`, and `*_test.rs` companions. Tests are evidence the surface exists, not declarations of it.
- **Build artifacts** — anything under `target/`, generated `OUT_DIR` output, vendored deps under `vendor/`, and `.cargo/`-cached registry sources.
- **Documentation tests** — fenced `rust` blocks inside `///` doc comments. They run under `cargo test --doc`, are never deployed, and never count as a surface.
- **Absolute paths** — `/Users/...`, `C:\...`, or any path that does not resolve under the source root.
- **`..` traversal** — `../../shared/foo.rs` in `touches[]` or `declared-at[]`. The CLI rejects these with `surfaces-touches-out-of-tree`.
- **Hallucinated framework imports** — if the project never imports `axum::`, do NOT emit Axum surfaces inferred from path-string heuristics. Likewise for `actix_web::`, `rocket::`, `lapin::`, `rdkafka::`, `async_nats::`, `reqwest::`. Surfaces require a real registration line in real code.
- **Generic / monomorphized handler instantiations as separate surfaces** — when a generic handler `async fn handle<T: Repo>(…)` is registered once in the router but instantiated with two different `T`, emit ONE surface keyed on the source declaration. The surface IS the source declaration; monomorphizations are an implementation detail of `cargo build`.
