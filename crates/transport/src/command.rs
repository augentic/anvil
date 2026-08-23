//! CLI grammar, conversions, and output projection.

use clap::Args;
use emery_engine::handler::Render;
use omnia_guest::api::command::{CommandResponse, Outcome, Projector};
use serde::Serialize;

use self::output::{ErrorBody, Exit, emit, write_error_text};
pub use self::output::{Format, render_failure, render_success};
pub use self::routes::router;

mod output;
mod routes;

/// Global command arguments.
#[derive(Clone, Copy, Debug, Args)]
pub struct Globals {
    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text")]
    pub format: Format,
}

/// Projects Emery outcomes into command responses.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmeryProjector;

impl<T> Projector<T, emery_engine::handler::Error, emery_error::Error, Globals> for EmeryProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = emery_error::Error;

    fn project(
        &self, outcome: Outcome<T, emery_engine::handler::Error, emery_error::Error>,
        globals: &Globals,
    ) -> Result<CommandResponse, Self::Error> {
        match outcome {
            Outcome::Output(output) => {
                Ok(CommandResponse::success(encode(globals.format, &output, |w, v| v.render(w))?))
            }
            Outcome::Operation(operation) => error_response(globals.format, &operation),
            Outcome::Decode(error) => Ok(error_response(globals.format, &error)?),
        }
    }

    fn project_failure(&self, error: Self::Error, globals: &Globals) -> CommandResponse {
        failure_response(globals.format, &error)
    }
}

fn encode<T: Serialize>(
    format: Format, value: &T,
    text: impl FnOnce(&mut dyn std::io::Write, &T) -> std::io::Result<()>,
) -> Result<Vec<u8>, emery_error::Error> {
    let mut bytes = Vec::new();
    emit(&mut bytes, format, value, text)?;
    Ok(bytes)
}

fn error_response(
    format: Format, error: &emery_error::Error,
) -> Result<CommandResponse, emery_error::Error> {
    let body = ErrorBody::from(error);
    let stderr = encode(format, &body, write_error_text)?;
    Ok(CommandResponse::failure(stderr, Exit::from(error).code()))
}

fn failure_response(format: Format, error: &emery_error::Error) -> CommandResponse {
    let (stderr, code) = render_failure(format, error);
    CommandResponse::failure(stderr, code)
}
