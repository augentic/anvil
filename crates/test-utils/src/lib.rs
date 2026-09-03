//! Harness for the root component suite (`tests/component.rs`).
//!
//! The build script compiles the mock adapter example to a `wasm32-wasip2`
//! component through `omnia_test::build` and generates its path constant.
//! [`run`] overlays the deployment `src/lib.rs` declares through
//! `omnia::runtime!` — the embedded engine guest, the source seam, the `.`
//! project mount, the model/storage/otel hosts, and the `.` path location
//! — with a scenario's project directory, arguments, and statically declared
//! adapter guests, then drives it through the same wiring the shipped binary
//! runs over scripted backends. A scenario observes exit status and storage
//! handles rather than stdout (which the runtime inherits).

#![cfg(not(target_arch = "wasm32"))]

use anyhow::Result;
use omnia::ExitStatus;
pub use omnia_test::host::{Backends, STATE_BUCKET, Scratch, ScriptedModel, scratch};
pub use omnia_test::{Exchange, Seen};

include!(concat!(env!("OUT_DIR"), "/gen.rs"));

/// The built mock adapter component (`examples/adapter`).
pub use ADAPTER as MOCK_ADAPTER;

/// One command-mode invocation of the shipped deployment.
#[derive(Clone, Copy, Debug)]
pub struct Deployment<'a> {
    /// CLI arguments after the program name (`["specify", "greeting"]`).
    pub argv: &'a [&'a str],
    /// The project directory, mounted read-only as `.` and serving the
    /// path-load location.
    pub project: &'a Scratch,
    /// Statically declared `(id, component path)` guests — bare-name
    /// adapters reachable by dispatch.
    pub guests: &'a [(&'a str, &'a str)],
}

/// Drive `deployment` once over `backends`, returning the guest's exit
/// status.
///
/// # Errors
///
/// Returns an error if the deployment cannot be built or linked, the path
/// loader cannot open the project, or the guest traps without exiting.
pub async fn run(
    deployment: Deployment<'_>, backends: Backends<ScriptedModel>,
) -> Result<ExitStatus> {
    // Mount names dedupe last-wins, so the project replaces the base `.`
    // mount (the invocation directory) rather than sitting beside it.
    let mut overlay = omnia_test::host::Deployment::from(emery::manifest())
        .mount(deployment.project.mount(false))
        .path_root(deployment.project.path())
        .args(deployment.argv.iter().copied());
    for (id, wasm) in deployment.guests {
        overlay = overlay.guest(*id, *wasm);
    }
    overlay.run_with::<emery::Hooks, _>(backends).await
}
