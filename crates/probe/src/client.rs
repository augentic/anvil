//! The shared live composition client (`feature = "client"`).
//!
//! One argv dispatch shaped like the `emery` CLI plus one extra verb:
//! emery verbs pass through natively, `eval` or a case id runs a case.

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

/// One classified composition invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// The case runner, with normalized `eval …` args (`args[0]` is
    /// the `eval` shim name the argv parser runs under).
    Runner(Vec<String>),
    /// Native command passthrough over `rest` (argv without the
    /// program name); `case` names the leading case id when one bound
    /// the invocation to its retained sandbox.
    Passthrough {
        /// Argv tokens without the program name.
        rest: Vec<String>,
        /// The bound case id, when a leading case id was consumed.
        case: Option<String>,
    },
}

/// Top-level verbs the native emery router owns. A case id colliding
/// with one of these would be unreachable by bare name; the catalogs
/// avoid such ids.
const VERBS: &[&str] = &[
    "adapter", "archive", "debt", "init", "journal", "plan", "slice", "source", "system", "target",
];

fn is_verb(token: &str) -> bool {
    VERBS.contains(&token)
}

/// Classify one invocation's tokens (argv without the program name,
/// after the host flags and the leading `--project-dir` are peeled).
///
/// Empty tokens and an explicit `eval` route to the case runner, as
/// does a leading known case id followed by nothing but runner flags
/// (`--restart`, `--until …`). A leading emery verb passes through; a
/// case id followed by a verb binds that case's sandbox and passes
/// the rest through. Anything else passes through so the native
/// router renders the rejection (or help).
#[must_use]
pub fn classify(rest: &[String], case_ids: &[String]) -> Invocation {
    let Some(first) = rest.first() else {
        return Invocation::Runner(vec!["eval".to_string()]);
    };
    if first == "eval" {
        return Invocation::Runner(rest.to_vec());
    }
    if is_verb(first) {
        return Invocation::Passthrough {
            rest: rest.to_vec(),
            case: None,
        };
    }
    if case_ids.iter().any(|id| id == first) {
        if rest.get(1).is_some_and(|second| is_verb(second)) {
            return Invocation::Passthrough {
                rest: rest[1..].to_vec(),
                case: Some(first.clone()),
            };
        }
        let mut args = vec!["eval".to_string()];
        args.extend(rest.iter().cloned());
        return Invocation::Runner(args);
    }
    Invocation::Passthrough {
        rest: rest.to_vec(),
        case: None,
    }
}

/// The base tracing filter one invocation runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// `--quiet`: tracing off.
    Off,
    /// The debug preset (`info,omnia_cursor=debug,omnia_wasi_http=debug`).
    Debug,
    /// The ambient `RUST_LOG` filter.
    Ambient,
    /// The flagless default.
    Info,
}

/// Select the base filter for one invocation.
///
/// An explicit host flag (`--quiet` is [`Filter::Off`], `--debug` is
/// [`Filter::Debug`]) wins, then the ambient `RUST_LOG`, then `info` —
/// backend and seam visibility is opt-in via `--debug`, matching the
/// shipped binary.
#[must_use]
pub const fn filter(flag: Option<Filter>, ambient: bool) -> Filter {
    match (flag, ambient) {
        (Some(explicit), _) => explicit,
        (None, true) => Filter::Ambient,
        (None, false) => Filter::Info,
    }
}

/// Dispatch one composition-binary invocation over `catalog`.
///
/// [`classify`] routes the tokens: the case runner through
/// [`crate::run`] (with the composition's `cases` and `sandbox`
/// roots, when carried), everything else through the native command
/// API under a `emery.command` span — bound to a case's retained
/// sandbox locations when the invocation names one.
///
/// # Errors
///
/// Returns tracing init failures, `--project-dir` resolution
/// failures, sandbox-binding failures, and every [`crate::run`]
/// failure.
pub async fn run(
    mut argv: Vec<String>, catalog: Catalog, cases: Option<&Path>, sandbox: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // `cargo make eval -- ARGS` forwards the literal `--` separator.
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
    let invocation = classify(&argv[1..], &case_ids(&root, cases));
    let flag = match (quiet, debug) {
        (true, _) => Some(Filter::Off),
        (false, true) => Some(Filter::Debug),
        (false, false) => None,
    };
    let choice = filter(flag, std::env::var_os("RUST_LOG").is_some());
    init_tracing(log_destination(&invocation, &root, sandbox), choice)?;
    // Same EventKind growth as `plan execute`: clippy large_futures at 16KiB.
    Box::pin(dispatch(root, invocation, catalog, cases, sandbox)).await
}

// Known case ids for classification; a composition without a `cases`
// root (or an unreadable one) classifies nothing as a case id.
fn case_ids(root: &Path, cases: Option<&Path>) -> Vec<String> {
    cases
        .map(|dir| crate::run::anchored(root, dir))
        .and_then(|dir| crate::case::ids(&dir).ok())
        .unwrap_or_default()
}

async fn dispatch(
    root: PathBuf, invocation: Invocation, catalog: Catalog, cases: Option<&Path>,
    sandbox: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    match invocation {
        Invocation::Runner(args) => {
            Box::pin(crate::run(root, catalog, cursor_factory(), &args, cases, sandbox))
                .await
                .map(|()| ExitCode::SUCCESS)
        }
        Invocation::Passthrough { rest, case } => {
            let (paths, model_root) = passthrough_paths(&root, sandbox, case.as_deref(), &rest)?;
            let model = DynModel::new(DevModel::new(&model_root));
            let mut argv = vec!["emery".to_string()];
            argv.extend(rest);
            Ok(Box::pin(::native::command::run(paths, model, catalog, argv)).await)
        }
    }
}

/// The passthrough anchoring for one native command invocation.
///
/// An invocation bound to a retained case sandbox — a leading case
/// id, a `--change-dir` naming one, or a peeled `--project-dir` that
/// *is* one — runs over that sandbox's self-contained locations (the
/// adapter store, cache, snapshot, and workspace roots the case
/// runner used), so continuing a parked run is the same run. Anything
/// else keeps operator locations.
///
/// Returns the execution paths plus the model workspace root.
///
/// # Errors
///
/// Returns a case id bound against a composition without a sandbox
/// root, and a bound case whose sandbox does not exist yet.
pub fn passthrough_paths(
    root: &Path, sandbox: Option<&Path>, case: Option<&str>, rest: &[String],
) -> anyhow::Result<(ExecutionPaths, PathBuf)> {
    let sandbox_root = sandbox.map(|dir| {
        let dir = crate::run::anchored(root, dir);
        dir.canonicalize().unwrap_or(dir)
    });
    if let Some(id) = case {
        let base = sandbox_root
            .context("this composition carries no sandbox root to bind a case id against")?;
        let dir = base.join(id);
        anyhow::ensure!(
            dir.is_dir(),
            "no retained sandbox at {}; start the case with `cargo make eval {id}`",
            dir.display()
        );
        return Ok((crate::case::paths(&dir), dir));
    }
    if let Some(dir) = change_dir_of(rest) {
        let dir = resolve(root, &dir);
        if in_sandbox(&dir, sandbox_root.as_deref()) {
            // Keep `.` at the invocation root so the command surface
            // re-anchors the (possibly relative) `--change-dir` as
            // before; only the value locations move into the sandbox.
            return Ok((ExecutionPaths::new(root, crate::case::locations(&dir)), dir));
        }
    } else if in_sandbox(root, sandbox_root.as_deref()) {
        return Ok((crate::case::paths(root), root.to_path_buf()));
    }
    Ok((ExecutionPaths::operator(root.to_path_buf()), root.to_path_buf()))
}

// The `--change-dir` value argv carries, when any (both the
// `--change-dir X` and `--change-dir=X` spellings).
fn change_dir_of(rest: &[String]) -> Option<PathBuf> {
    let mut tokens = rest.iter();
    while let Some(token) = tokens.next() {
        if token == "--change-dir" {
            return tokens.next().map(PathBuf::from);
        }
        if let Some(value) = token.strip_prefix("--change-dir=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}

fn resolve(root: &Path, dir: &Path) -> PathBuf {
    let dir = if dir.is_absolute() { dir.to_path_buf() } else { root.join(dir) };
    dir.canonicalize().unwrap_or(dir)
}

// A directory is a retained case sandbox exactly when its parent is
// the composition's sandbox root.
fn in_sandbox(dir: &Path, sandbox_root: Option<&Path>) -> bool {
    sandbox_root.is_some_and(|base| dir.parent() == Some(base))
}

/// The log-file destination: an explicit `EVAL_LOG` always wins; a
/// case-runner invocation naming a case defaults to a per-run
/// timestamped file under the composition's sandbox root; anything
/// else logs to console only.
fn log_destination(
    invocation: &Invocation, root: &Path, sandbox: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("EVAL_LOG") {
        return Some(PathBuf::from(path));
    }
    let Invocation::Runner(args) = invocation else {
        return None;
    };
    let case = crate::run::case_of(args)?;
    let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S");
    let sandbox = crate::run::anchored(root, sandbox?);
    // Collapse `examples/eval/../../sandbox` once the root exists.
    let sandbox = sandbox.canonicalize().unwrap_or(sandbox);
    Some(sandbox.join("logs").join(case).join(format!("eval-{stamp}.log")))
}

/// Process tracing init: a stderr console layer plus, when `log`
/// names a file, an ANSI-free copy of the same filtered output.
fn init_tracing(log: Option<PathBuf>, choice: Filter) -> anyhow::Result<()> {
    let filter = match choice {
        Filter::Off => EnvFilter::new("off"),
        Filter::Debug => EnvFilter::new("info,omnia_cursor=debug,omnia_wasi_http=debug"),
        Filter::Ambient => EnvFilter::from_default_env(),
        Filter::Info => EnvFilter::new("info"),
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
    // output, matching the shipped runtime's stream roles. The console
    // renders compact (no span-name chain); the file copy keeps the
    // full span context for grepping.
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact().with_writer(std::io::stderr))
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
