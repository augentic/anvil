//! Typed command grammar, conversions, and Emery projection policy.

use clap::Args;
use engine::handler::{Anchor, Render};
use engine::resolve::Resolver;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{CommandResponse, Outcome, Projector, Router};
use serde::Serialize;
use tracing::Instrument as _;

use self::output::{ErrorBody, Exit, emit, write_error_text};
pub use self::output::{Format, render_failure, render_success};
pub use self::routes::router;

mod output;
mod routes;

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

impl<T> Projector<T, engine::handler::Error, error::Error, Globals> for EmeryProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = error::Error;

    fn project(
        &self, outcome: Outcome<T, engine::handler::Error, error::Error>, globals: &Globals,
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
    format: Format, error: engine::handler::Error,
) -> Result<CommandResponse, error::Error> {
    match error {
        engine::handler::Error::Core(source) => error_response(format, &source),
        engine::handler::Error::Report { body, source } => {
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
/// code — never the full argv, which may embed operator prose. Both
/// deployments route through here: the native host's command entry and
/// the engine guest's `wasi:cli/run` exporter.
pub async fn execute<P>(router: &Router<P, Globals>, argv: Vec<String>) -> CommandResponse
where
    P: Provider + Anchor + Resolver,
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
