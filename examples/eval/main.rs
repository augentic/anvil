//! Emery's live composition example: native command passthrough
//! over the mock catalog by default, the live eval case runner under
//! the `eval` subcommand. (wasm32 builds compile an empty stub so
//! `--examples` passes.)
//!
//! The composition root owns what the shared client (`probe::client`)
//! refuses to: the Tokio runtime, `std::env::args` collection, and
//! the catalog, cases, and sandbox declarations. It is a development
//! tool, never an install or release artifact. Driven by `cargo make
//! lab` and `cargo make eval`.

#[cfg(target_arch = "wasm32")]
fn main() {}

/// The engine's eval cases, anchored at the crate manifest so the
/// runner is independent of the process working directory.
#[cfg(not(target_arch = "wasm32"))]
const CASES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/eval/cases");

/// Retained per-case sandboxes at the repository root, beside the
/// wasm example's `sandbox/wasm/` tree.
#[cfg(not(target_arch = "wasm32"))]
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/sandbox");

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cases = std::path::Path::new(CASES);
    let sandbox = std::path::Path::new(SANDBOX);
    match probe::client::run(
        std::env::args().collect(),
        mock::catalog(),
        Some(cases),
        Some(sandbox),
    )
    .await
    {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
