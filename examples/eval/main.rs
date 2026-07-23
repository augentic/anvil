//! Specify's live composition example: native command passthrough
//! over the mock catalog by default, the live eval client under the
//! `eval` subcommand. (wasm32 builds compile an empty stub so
//! `--examples` passes.)
//!
//! The composition root owns what the shared client (`probe::client`)
//! refuses to: the Tokio runtime, `std::env::args` collection, and
//! the catalog declaration. It is a development tool, never an
//! install or release artifact. Driven by `cargo make specify` and
//! `cargo make eval`.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match probe::client::run(std::env::args().collect(), mock::catalog(), None).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
