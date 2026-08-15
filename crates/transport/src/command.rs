//! Typed command grammar, conversions, and Emery projection policy.

use clap::Args;
use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{CommandResponse, Outcome, Projector, Router};
use project::adapter::{Inventory, Resolver};
use project::handler::{Anchor, Render};
use project::profile::Profiles;
use project::seam::{Forge, Source, Target, Trees, Workspaces, Worktree};
use serde::Serialize;
use tracing::Instrument as _;

use self::output::{ErrorBody, Exit, emit, write_error_text};
pub use self::output::{Format, render_failure, render_success};
pub use self::routes::router;

mod adapter;
mod archive;
mod change_dir;
mod journal;
mod output;
mod plan;
mod routes;
pub mod selectors;
mod slice;
mod source;
mod system;
mod target;

/// Arguments shared by every command route.
#[derive(Clone, Copy, Debug, Args)]
pub struct Globals {
    /// Output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text")]
    pub format: Format,
}

/// Emery's command output and error projection.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmeryProjector;

impl<T> Projector<T, project::handler::Error, error::Error, Globals> for EmeryProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = error::Error;

    fn project(
        &self, outcome: Outcome<T, project::handler::Error, error::Error>, globals: &Globals,
    ) -> Result<CommandResponse, Self::Error> {
        match outcome {
            Outcome::Output(output) => {
                Ok(CommandResponse::success(encode(globals.format, &output, |w, v| v.render(w))?))
            }
            Outcome::Operation(operation) => operation_response(globals.format, operation),
            Outcome::Decode(error) => Ok(error_response(globals.format, &error)?),
        }
    }

    fn project_failure(&self, error: Self::Error, globals: &Globals) -> CommandResponse {
        failure_response(globals.format, &error)
    }
}

/// Buffer one [`emit`] rendering of `value` for a `CommandResponse`
/// channel.
fn encode<T: Serialize>(
    format: Format, value: &T,
    text: impl FnOnce(&mut dyn std::io::Write, &T) -> std::io::Result<()>,
) -> Result<Vec<u8>, error::Error> {
    let mut bytes = Vec::new();
    emit(&mut bytes, format, value, text)?;
    Ok(bytes)
}

fn error_response(format: Format, error: &error::Error) -> Result<CommandResponse, error::Error> {
    let body = ErrorBody::from(error);
    let stderr = encode(format, &body, write_error_text)?;
    Ok(CommandResponse::failure(stderr, Exit::from(error).code()))
}

/// [`render_failure`] mapped onto a `CommandResponse` — the terminal
/// fallback (a plain exit-1 line) lives in one place.
fn failure_response(format: Format, error: &error::Error) -> CommandResponse {
    let (stderr, code) = render_failure(format, error);
    CommandResponse::failure(stderr, code)
}

fn operation_response(
    format: Format, error: project::handler::Error,
) -> Result<CommandResponse, error::Error> {
    match error {
        project::handler::Error::Core(source) => error_response(format, &source),
        project::handler::Error::Report { body, source } => {
            let stdout = encode(format, &body, |w, v| v.render(w))?;
            let mut response = error_response(format, &source)?;
            response.stdout = stdout;
            Ok(response)
        }
    }
}

/// Run one routed invocation (`argv[0]` is the binary name) under the
/// `emery.command` span.
///
/// The span carries only the bounded verb label and the response exit
/// code — never the full argv, which may embed operator prose (e.g.
/// `plan author --from …`). Both deployments route through here: the
/// native host's command entry and the engine guest's `wasi:cli/run`
/// exporter.
pub async fn execute<P>(router: &Router<P, Globals>, argv: Vec<String>) -> CommandResponse
where
    P: Provider
        + Anchor
        + Model
        + Resolver
        + Inventory
        + Profiles
        + Source
        + Target
        + Workspaces
        + Trees
        + Worktree
        + Forge,
{
    let span = tracing::info_span!(
        "emery.command",
        command = %label(&argv),
        exit = tracing::field::Empty,
    );
    async {
        let response = router.execute(argv).await;
        tracing::Span::current().record("exit", response.exit);
        response
    }
    .instrument(span)
    .await
}

/// The bounded span label: the first two non-flag tokens after the
/// binary name (`plan author`, `slice list`).
fn label(argv: &[String]) -> String {
    let words: Vec<&str> = argv
        .iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .take(2)
        .map(String::as_str)
        .collect();
    words.join(" ")
}
