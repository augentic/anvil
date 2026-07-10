//! The CLI transport bridge: drive one command [`Handler`] and render
//! its `Reply` (or failure) onto the process streams.
//!
//! [`run`] is the single per-command body behind every routing arm:
//! serde-round-trip the parsed mirror `*Args` onto the handler's
//! `Input` (the one argv-side conversion in the codebase — see
//! [`extract`]), build the request via `Handler::from_input`, handle
//! it against the shim's provider, then render the typed body — JSON
//! or [`Render`] text on stdout for success, the failure envelope on
//! stderr (report-carrying validation failures render their findings
//! on stdout first, preserving the two-channel contract).

use error::Error;
use omnia_guest::api::{Handler, Provider};
use serde::Serialize;
use serde::de::DeserializeOwned;
use workflow::handler::{Out, Render};

use crate::output::{Exit, Format, emit, report};

/// The one generic argv → `Input` bridge.
///
/// Renders the mirror `*Args` to its kebab-case wire map and
/// deserializes the handler `Input` from it — exactly the extraction
/// the HTTP transport performs on a merged request.
///
/// # Errors
///
/// A failure is mirror drift between the `*Args` struct and its
/// `Input` — a programming error, not operator error — surfaced on
/// the standard failure envelope; the per-command extraction tests
/// exist to make it unreachable.
pub fn extract<I: DeserializeOwned>(args: impl Serialize) -> Result<I, Error> {
    serde_json::to_value(args).and_then(serde_json::from_value).map_err(|err| Error::Diag {
        code: "argv-bridge-drift",
        detail: format!("argv mirror does not serialize onto the handler input: {err}"),
    })
}

/// Drive one command handler against `provider` and render the outcome.
///
/// `args` is the routed leaf's mirror `*Args` struct, converted onto
/// `R::Input` through [`extract`]. The success body rides stdout
/// (`--format json` serialises it verbatim; text goes through the
/// body's [`Render`] impl); failures ride stderr through [`report`]
/// and map onto the exit-code contract via `Exit::from(&error::Error)`.
pub async fn run<R, P, B>(format: Format, provider: &P, args: impl Serialize + Send) -> Exit
where
    R: Handler<P, Output = Out<B>, Error = workflow::handler::Error>,
    R::Input: DeserializeOwned,
    P: Provider,
    B: Render + Send + Sync,
{
    let input: R::Input = match extract(args) {
        Ok(input) => input,
        Err(err) => return report(format, &err),
    };
    let handled = match R::handler(input) {
        Ok(handler) => handler.owner("specify").provider(provider).handle().await,
        Err(err) => Err(err),
    };
    match handled {
        Ok(reply) => {
            let written =
                emit(&mut std::io::stdout().lock(), format, &reply.body, |w, out| out.0.render(w));
            match written {
                Ok(()) => Exit::Success,
                Err(err) => report(format, &err),
            }
        }
        Err(err) => fail(format, err),
    }
}

/// Render one handler failure: report-carrying errors put their findings
/// on stdout (the success channel) before the payload-free failure
/// envelope lands on stderr; plain failures go straight to stderr.
pub fn fail(format: Format, err: workflow::handler::Error) -> Exit {
    match err {
        workflow::handler::Error::Report { body, source } => {
            let written = emit(&mut std::io::stdout().lock(), format, &body, |w, b| b.render(w));
            if let Err(write_err) = written {
                return report(format, &write_err);
            }
            report(format, &source)
        }
        workflow::handler::Error::Core(core) => report(format, &core),
    }
}
