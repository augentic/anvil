//! `specify-dev` — the Rust-native shim binary.
//!
//! Two modes over the same handler layer the wasm guest serves:
//!
//! - **CLI mode** (default): the shared argv contract — `cli::parse`,
//!   then this shim's own exhaustive dispatch match against a
//!   [`NativeProvider`], with the same provisioning refusals as the
//!   guest (parity-or-less, never parity-plus). An ephemeral MCP
//!   listener serves the adapter reference shelves so judgment legs
//!   carry real grants.
//! - **`serve` mode**: this shim's own hand-written HTTP route table
//!   merged with the `/mcp/<name>` shelves on one `TcpListener` — the
//!   native counterpart of the guest's `wasi:http/incoming-handler`
//!   export. Mutating dispatch is serialized behind a process-wide
//!   write lock; GETs stay concurrent.

mod dispatch;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use clap::Parser;
use omnia_guest::api::{Client, route};
use omnia_guest::http::Method;
use specify_dev::model::DevModel;
use specify_dev::provider::NativeProvider;
use specify_dev::{mcp, provider};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use workflow::change::plan;
use workflow::{adapter, init, journal, orchestrate, registry, slice};

#[tokio::main]
async fn main() -> ExitCode {
    // Adapter describe dispatch calls the linked adapter crates'
    // `describe()` directly, so the resolvers work without a wasm
    // runtime.
    adapter::describe::register(provider::describe);

    let argv: Vec<String> = std::env::args().collect();
    let outcome = if argv.get(1).map(String::as_str) == Some("serve") {
        serve(&argv[1..]).await
    } else {
        return ExitCode::from(dispatch::main(argv).await);
    };
    match outcome {
        Ok(code) => code,
        Err(err) => {
            eprintln!("specify-dev: {err:#}");
            ExitCode::FAILURE
        }
    }
}

/// `specify-dev serve` — the native HTTP transport.
#[derive(Debug, Parser)]
#[command(name = "serve", about = "Serve the command routes and MCP shelves over HTTP")]
struct ServeArgs {
    /// Listen port (0 picks an ephemeral port).
    #[arg(long, default_value_t = 7737)]
    port: u16,
    /// Project root the provider anchors at.
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
}

/// Bind the listener, build the provider, and serve the merged router.
async fn serve(argv: &[String]) -> Result<ExitCode> {
    let opts = ServeArgs::parse_from(argv);
    let project_dir =
        opts.project_dir.canonicalize().context("resolving the served project root")?;

    let listener = TcpListener::bind(("127.0.0.1", opts.port))
        .await
        .with_context(|| format!("binding 127.0.0.1:{}", opts.port))?;
    let base = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    println!("specify-dev serving {} at {base}", project_dir.display());

    let model = DevModel::from_env(&project_dir)?;
    let provider = NativeProvider::new(project_dir, model).mcp_base(base);
    let client = Client::new("specify").provider(provider);
    let router = router(client).layer(middleware::from_fn(serialize_writes)).merge(mcp::router());

    axum::serve(listener, router).await.context("serving")?;
    Ok(ExitCode::SUCCESS)
}

/// The served provider type — the state every route line binds.
type P = NativeProvider<DevModel>;

/// This shim's HTTP route table, one line per routed command — GET for
/// pure reads (path + query args), POST for writes and judgment (JSON
/// bodies), the noun in the path. Deliberately a hand-maintained copy
/// of the guest table in the repo root's `src/lib.rs`: parity between
/// transports comes from both routing to the same `Handler` impls,
/// not from shared table code. Grammar leaves with no route here are
/// the provisioning commands and `completions`.
fn router(client: Client<P>) -> axum::Router {
    axum::Router::new()
        .route("/init/scaffold", route::post::<init::handlers::Scaffold, P>())
        .route("/source/resolve", route::get::<adapter::handlers::SourceResolve, P>())
        .route("/source/{source}/survey", route::post::<orchestrate::handlers::Survey, P>())
        .route("/source/{source}/extract", route::post::<orchestrate::handlers::Extract, P>())
        .route("/target/resolve", route::get::<adapter::handlers::TargetResolve, P>())
        .route("/slice/{name}/create", route::post::<slice::handlers::Create, P>())
        .route("/slice/{name}/validate", route::get::<slice::handlers::Validate, P>())
        .route("/slice/{name}/provenance", route::get::<slice::handlers::Provenance, P>())
        .route("/slice/{name}/model", route::get::<slice::handlers::ModelShow, P>())
        .route("/slice/{name}/refine", route::post::<orchestrate::handlers::Refine, P>())
        .route("/slice/{name}/build", route::post::<orchestrate::handlers::Build, P>())
        .route("/slice/{name}/merge", route::post::<orchestrate::handlers::MergeRun, P>())
        .route("/slice/{name}/merge/preview", route::get::<slice::handlers::Preview, P>())
        .route(
            "/slice/{name}/merge/conflict-check",
            route::get::<slice::handlers::ConflictCheck, P>(),
        )
        .route("/slice/{name}/tasks", route::get::<slice::handlers::TaskProgress, P>())
        .route("/slice/{name}/tasks/{task-number}", route::post::<slice::handlers::TaskMark, P>())
        .route("/slice/{name}/transition", route::post::<slice::handlers::Transition, P>())
        .route("/slice/{name}/touched-specs", route::post::<slice::handlers::TouchedSpecs, P>())
        .route("/slice/{name}/overlap", route::get::<slice::handlers::Overlap, P>())
        .route("/slice/{name}/drop", route::post::<slice::handlers::Drop, P>())
        .route("/archive/prune", route::post::<slice::handlers::Prune, P>())
        .route("/plan/{name}/create", route::post::<plan::handlers::Create, P>())
        .route("/plan/validate", route::get::<plan::handlers::Validate, P>())
        .route("/plan/next", route::post::<plan::handlers::Next, P>())
        .route("/plan/status", route::get::<plan::handlers::Status, P>())
        .route("/plan/{name}/add", route::post::<plan::handlers::Add, P>())
        .route("/plan/{name}/amend", route::post::<plan::handlers::Amend, P>())
        .route("/plan/{name}/remove", route::post::<plan::handlers::Remove, P>())
        .route("/plan/{name}/transition", route::post::<plan::handlers::Transition, P>())
        .route("/plan/{name}/author", route::post::<orchestrate::handlers::Author, P>())
        .route("/plan/execute", route::post::<orchestrate::handlers::Execute, P>())
        .route("/plan/archive", route::post::<plan::handlers::Archive, P>())
        .route("/journal", route::post::<journal::handlers::Emit, P>())
        .route("/journal", route::get::<journal::handlers::Show, P>())
        .route("/registry/validate", route::get::<registry::handlers::Validate, P>())
        .route("/registry", route::post::<registry::handlers::Add, P>())
        .route("/registry/{name}/remove", route::post::<registry::handlers::Remove, P>())
        .with_state(client)
}

/// `.specify/` assumes a single writer: atomic writes protect files,
/// not workflows, so mutating dispatch is serialized process-wide
/// while GETs stay concurrent.
async fn serialize_writes(request: Request, next: Next) -> Response {
    static WRITES: Mutex<()> = Mutex::const_new(());
    let guard = if request.method() == Method::GET { None } else { Some(WRITES.lock().await) };
    let response = next.run(request).await;
    drop(guard);
    response
}
