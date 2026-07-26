//! The shared live composition client (`feature = "client"`).
//!
//! One argv dispatch serving both composition examples — the `eval`
//! example in this repository over the mock catalog and the one in
//! `augentic/specify-adapters` over the first-party catalog: native
//! command passthrough by default, the live case runner under the
//! `eval` subcommand. The client owns the lazily connected cursor
//! backend ([`DevModel`]), process telemetry init via
//! [`omnia::Telemetry`] (with an explicit [`omnia::telemetry::flush`]
//! before exit so batched spans survive fast command exits), and the
//! `--project-dir` convenience; the composition root keeps what the
//! client refuses to own — the Tokio runtime, `std::env::args`
//! collection, and the catalog and cases declarations.
//!
//! Driver-side tracing knobs (same as the Omnia runtime binary):
//! - `RUST_LOG` — `tracing` filter (e.g. `info,opentelemetry_sdk=off`)
//! - `OTEL_GRPC_URL` — optional OTLP gRPC endpoint; unset uses
//!   OpenTelemetry defaults (`http://localhost:4317`)

mod model;
mod native;

use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use ::native::{Catalog, DynModel, ExecutionPaths};
use anyhow::Context as _;
pub use model::DevModel;
use tracing::Instrument as _;

use crate::{ModelFactory, ModelInstance, case};

/// Dispatch one composition-binary invocation over `catalog`.
///
/// The `eval` subcommand routes through [`crate::run`] (with `cases`
/// as the case-directory root, when the composition carries one);
/// anything else runs through the native command API under a
/// `specify.command` span.
///
/// # Errors
///
/// Returns telemetry init failures, `--project-dir` resolution
/// failures, and every [`crate::run`] failure.
pub async fn run(
    argv: Vec<String>, catalog: Catalog, cases: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // Mirrors Omnia `init_env`: install once, then flush before exit so
    // batched spans survive even a fast passthrough command.
    init_telemetry()?;
    let code = dispatch(argv, catalog, cases).await;
    omnia::telemetry::flush();
    code
}

async fn dispatch(
    mut argv: Vec<String>, catalog: Catalog, cases: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // `cargo make specify -- ARGS` forwards the literal `--` separator.
    if argv.get(1).is_some_and(|arg| arg == "--") {
        argv.remove(1);
    }
    let root = project_root(&mut argv)?;

    if argv.get(1).is_some_and(|arg| arg == "eval") {
        return crate::run(root, catalog, cursor_factory(), &argv[1..], cases).await;
    }

    let span = tracing::info_span!(
        "specify.command",
        command = %case::command_label(argv.get(1..).unwrap_or_default()),
        exit = tracing::field::Empty,
    );
    let paths = ExecutionPaths::operator(root.clone());
    let model = DynModel::new(DevModel::new(&root));
    let code = async {
        match ::native::command::execute(paths, model, catalog, argv).await {
            Ok(response) => {
                tracing::Span::current().record("exit", response.exit);
                response
                    .write_to(&mut io::stdout().lock(), &mut io::stderr().lock())
                    .unwrap_or(ExitCode::FAILURE)
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        }
    }
    .instrument(span)
    .await;
    Ok(code)
}

/// Process tracing / OTLP init (mirrors Omnia `init_env`). Idempotence
/// lives in `omnia::Telemetry` — later builds in the same process
/// share the first initialization's providers.
fn init_telemetry() -> anyhow::Result<()> {
    let mut builder = omnia::Telemetry::new("specify-eval");
    if let Ok(endpoint) = std::env::var("OTEL_GRPC_URL") {
        builder = builder.endpoint(endpoint);
    } else {
        tracing::debug!("OTEL_GRPC_URL unset; using OpenTelemetry defaults");
    }
    builder.build().context("initializing telemetry")
}

/// A lazily connected cursor-agent backend per case root, carrying
/// the `EVAL_MODEL` default read once at composition.
#[must_use]
pub fn cursor_factory() -> ModelFactory {
    let default = std::env::var("EVAL_MODEL").ok().filter(|id| !id.trim().is_empty());
    Arc::new(move |root| {
        Ok(ModelInstance {
            model: DynModel::new(DevModel::new(root)),
            default_model: default.clone(),
        })
    })
}

/// Resolve the lab's canonical anchor: the `--project-dir` option when
/// placed before the subcommand, else the current directory.
fn project_root(argv: &mut Vec<String>) -> anyhow::Result<PathBuf> {
    let dir = take_project_dir(argv).map_err(|message| anyhow::anyhow!(message))?;
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    dir.canonicalize().map_err(|error| anyhow::anyhow!("--project-dir {}: {error}", dir.display()))
}

// Only the option before the subcommand is the lab's; later `--project-dir` passes through.
fn take_project_dir(argv: &mut Vec<String>) -> Result<Option<PathBuf>, String> {
    let Some(first) = argv.get(1).cloned() else {
        return Ok(None);
    };
    if first == "--project-dir" {
        let Some(path) = argv.get(2).cloned() else {
            return Err("--project-dir requires a path".to_string());
        };
        argv.drain(1..=2);
        return Ok(Some(PathBuf::from(path)));
    }
    if let Some(path) = first.strip_prefix("--project-dir=") {
        let path = PathBuf::from(path);
        argv.remove(1);
        return Ok(Some(path));
    }
    Ok(None)
}
