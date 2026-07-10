//! The CLI transport bridge: drive one verb [`Handler`] and render
//! its `Reply` (or failure) onto the process streams.
//!
//! [`run`] is the single per-verb body behind every dispatch-match
//! arm: build the request via `Handler::from_input`, handle it
//! against the shim's provider, then render the typed body — JSON or
//! [`Render`] text on stdout for success, the failure envelope on
//! stderr (report-carrying validation failures render their findings
//! on stdout first, preserving the two-channel contract).

use omnia_guest::api::{Handler, Provider};
use workflow::verb::{Out, Render};

use crate::output::{Exit, Format, emit, report};

/// Drive one verb handler against `provider` and render the outcome.
///
/// The success body rides stdout (`--format json` serialises it
/// verbatim; text goes through the body's [`Render`] impl); failures
/// ride stderr through [`report`] and map onto the exit-code
/// contract via `Exit::from(&error::Error)`.
pub async fn run<R, P, B>(format: Format, provider: &P, input: R::Input) -> Exit
where
    R: Handler<P, Output = Out<B>, Error = workflow::verb::Error>,
    P: Provider,
    B: Render + Send + Sync,
{
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

/// Render one verb failure: report-carrying errors put their findings
/// on stdout (the success channel) before the payload-free failure
/// envelope lands on stderr; plain failures go straight to stderr.
pub fn fail(format: Format, err: workflow::verb::Error) -> Exit {
    match err {
        workflow::verb::Error::Report { body, source } => {
            let written = emit(&mut std::io::stdout().lock(), format, &body, |w, b| b.render(w));
            if let Err(write_err) = written {
                return report(format, &write_err);
            }
            report(format, &source)
        }
        workflow::verb::Error::Core(core) => report(format, &core),
    }
}
