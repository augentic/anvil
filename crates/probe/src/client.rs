//! The shared live composition client (`feature = "client"`).
//!
//! One argv dispatch serving both composition examples: native command
//! passthrough by default, the live case runner under `eval`.

mod model;
mod native;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use ::native::{Catalog, DynModel, ExecutionPaths};
use anyhow::Context as _;
pub use model::DevModel;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{DefaultFields, FormatFields, Writer};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

use crate::ModelFactory;

/// Distinct [`FormatFields`] type so the file layer owns its own
/// span-field cache. Sharing [`DefaultFields`] with the console layer
/// would reuse that layer's ANSI-formatted entries (and double-append
/// on every [`tracing::Span::record`]).
#[derive(Debug, Default)]
struct PlainFields(DefaultFields);

impl<'writer> FormatFields<'writer> for PlainFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self, writer: Writer<'writer>, fields: R,
    ) -> std::fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// Dispatch one composition-binary invocation over `catalog`.
///
/// The `eval` subcommand routes through [`crate::run`] (with the
/// composition's `cases` and `sandbox` roots, when carried); anything
/// else runs through the native command API under a `emery.command`
/// span.
///
/// # Errors
///
/// Returns tracing init failures, `--project-dir` resolution
/// failures, and every [`crate::run`] failure.
pub async fn run(
    mut argv: Vec<String>, catalog: Catalog, cases: Option<&Path>, sandbox: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // `cargo make lab -- ARGS` forwards the literal `--` separator.
    if argv.get(1).is_some_and(|arg| arg == "--") {
        argv.remove(1);
    }
    // The reserved host log flags, peeled before the grammar (and the
    // `--project-dir` probe below) ever see them — the same contract the
    // shipped runtime binary's direct-command entry applies.
    let debug = argv.iter().any(|arg| arg == "--debug");
    let quiet = argv.iter().any(|arg| arg == "--quiet");
    anyhow::ensure!(!(debug && quiet), "`--debug` and `--quiet` are mutually exclusive");
    argv.retain(|arg| arg != "--debug" && arg != "--quiet");
    let root = project_root(&mut argv)?;
    init_tracing(log_destination(&argv, &root, sandbox), debug, quiet)?;
    dispatch(root, argv, catalog, cases, sandbox).await
}

async fn dispatch(
    root: PathBuf, argv: Vec<String>, catalog: Catalog, cases: Option<&Path>,
    sandbox: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    if argv.get(1).is_some_and(|arg| arg == "eval") {
        return crate::run(root, catalog, cursor_factory(), &argv[1..], cases, sandbox)
            .await
            .map(|()| ExitCode::SUCCESS);
    }

    let paths = ExecutionPaths::operator(root.clone());
    let model = DynModel::new(DevModel::new(&root));
    Ok(::native::command::run(paths, model, catalog, argv).await)
}

/// The log-file destination: an explicit `EVAL_LOG` always wins; a
/// named eval case defaults to a per-run timestamped file under the
/// composition's sandbox root; anything else logs to console only.
fn log_destination(argv: &[String], root: &Path, sandbox: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("EVAL_LOG") {
        return Some(PathBuf::from(path));
    }
    if argv.get(1).is_none_or(|arg| arg != "eval") {
        return None;
    }
    let case = crate::run::case_of(&argv[1..])?;
    let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S");
    let sandbox = crate::run::anchored(root, sandbox?);
    // Collapse `examples/eval/../../sandbox` once the root exists.
    let sandbox = sandbox.canonicalize().unwrap_or(sandbox);
    Some(sandbox.join("logs").join(case).join(format!("eval-{stamp}.log")))
}

/// Process tracing init: a stderr console layer plus, when `log`
/// names a file, an ANSI-free copy of the same filtered output.
fn init_tracing(log: Option<PathBuf>, debug: bool, quiet: bool) -> anyhow::Result<()> {
    // The host-flag presets mirror the shipped runtime binary: a flag
    // wins over the ambient `RUST_LOG`; flagless defaults to INFO.
    let filter = match (quiet, debug, std::env::var_os("RUST_LOG").is_some()) {
        (true, ..) => EnvFilter::new("off"),
        (_, true, _) => EnvFilter::new("info,omnia_cursor=debug,omnia_wasi_http=debug"),
        (_, _, true) => EnvFilter::from_default_env(),
        _ => EnvFilter::new("info"),
    };
    // HTTP/gRPC stacks and the OTel GlobalSet chatter are noise under the
    // lab's console subscriber; the shipped runtime owns real export.
    let filter = filter
        .add_directive("hyper=off".parse()?)
        .add_directive("h2=off".parse()?)
        .add_directive("tonic=off".parse()?)
        .add_directive("opentelemetry=off".parse()?)
        .add_directive("opentelemetry_sdk=off".parse()?);
    let (file, log) = match log {
        Some(path) => {
            let (layer, path) = file_layer(&path)?;
            (Some(layer), Some(path))
        }
        None => (None, None),
    };
    // Console tracing goes to stderr: stdout stays the semantic command
    // output, matching the shipped runtime's stream roles.
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(file)
        .try_init()
        .context("installing the tracing subscriber")?;
    if let Some(log) = log {
        tracing::info!("eval log: {}", log.display());
    }
    Ok(())
}

/// An ANSI-free file copy of the console output.
///
/// Returns the canonical path (parents created) for the startup
/// announcement.
fn file_layer<S>(path: &Path) -> anyhow::Result<(impl tracing_subscriber::Layer<S>, PathBuf)>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let file = std::fs::File::create(path)
        .with_context(|| format!("creating the log file {}", path.display()))?;
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .fmt_fields(PlainFields::default())
        .with_writer(Arc::new(file));
    Ok((layer, path))
}

/// A lazily connected cursor-agent backend per case root.
#[must_use]
pub fn cursor_factory() -> ModelFactory {
    Arc::new(|root| Ok(DynModel::new(DevModel::new(root))))
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
