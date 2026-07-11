//! The native shim's HTTP transport: `specify-dev serve` — the shared
//! typed HTTP router merged with the `/mcp/<name>` shelves on one
//! `TcpListener`, the native counterpart of the guest's
//! `wasi:http/incoming-handler` export. Mutating invocation is
//! serialized behind a process-wide write lock; GETs stay concurrent.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use clap::Parser;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::http::Method;
use specify_dev::mcp;
use specify_dev::model::DevModel;
use specify_dev::provider::Provider;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

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
pub async fn serve(argv: &[String]) -> Result<ExitCode> {
    let opts = ServeArgs::parse_from(argv);
    let project_dir =
        opts.project_dir.canonicalize().context("resolving the served project root")?;

    let listener = TcpListener::bind(("127.0.0.1", opts.port))
        .await
        .with_context(|| format!("binding 127.0.0.1:{}", opts.port))?;
    let base = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    println!("specify-dev serving {} at {base}", project_dir.display());

    let model = DevModel::from_env(&project_dir)?;
    let provider = Provider::new(project_dir, model).mcp_base(base);
    let router = transport::http::router(Invoker::new("specify", provider))
        .into_axum()
        .layer(middleware::from_fn(serialize_writes))
        .merge(mcp::router());

    axum::serve(listener, router).await.context("serving")?;
    Ok(ExitCode::SUCCESS)
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
