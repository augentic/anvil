//! The workflow guest: the deployment's only `wasi:cli/run` exporter
//! (RFC-61 Step 4, Milestone D).
//!
//! Argv arrives through wasip3 and parses through the shared
//! `specify-dispatch` grammar — the exact clap tree the native binary
//! parses, so every shared verb is argv- and envelope-compatible with
//! native. `specify_dispatch::guest::route` runs pure workflow verbs
//! in-process; the four collapsed orchestrator verbs come back as a
//! `specify_dispatch::guest::Orchestration` and `verbs::drive` runs
//! them against `provider::Provider` — the WIT-backed
//! `Model + SourceSeam + TargetSeam` implementation over this world's
//! `source` / `target` imports (satisfied at runtime by Omnia's
//! host-mediated dispatch, routed to the exporting adapter guest by
//! each call's `adapter-id` first argument).
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `Ctx::load`'s CWD walk finds
//! `.specify/project.yaml` exactly as a native run from the project
//! root would. Exit codes pass through verbatim — `Exit::code()` maps
//! onto `wasi:cli/exit#exit-with-code`, preserving the native binary's
//! closed exit-code contract.
#![cfg(target_arch = "wasm32")]

mod bindings {
    //! `wit_bindgen::generate!` output for the `workflow` world. The world only
    //! imports (`source` / `target`), so there is no `export!` shim here; the
    //! `wasi:cli/run` export is wired by wasip3 in the crate root.
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "workflow",
        path: "../../wit",
        // The seam operations are `async func`s (judgment legs await the
        // async `omnia:model` import mid-call), so the imports async-lower.
        async: true,
    });
}

mod provider;
mod verbs;

use specify_dispatch::guest::{self, Route};
use specify_dispatch::output::Exit;

struct CliGuest;
wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        // argv verbatim as the host provides it, argv[0] included —
        // the shared grammar sees exactly what native clap sees.
        let argv = wasip3::cli::environment::get_arguments();
        let cli = match guest::parse(argv) {
            Ok(cli) => cli,
            Err(exit) => return finish(exit),
        };
        let exit = match guest::route(cli) {
            Route::Handled(exit) => exit,
            Route::Orchestrate(orchestration) => verbs::drive(orchestration).await,
        };
        finish(exit)
    }
}

/// Exit-code passthrough: success returns through `run`'s happy leg;
/// any other code exits through `wasi:cli/exit#exit-with-code`, so the
/// host observes the same numeric contract the native binary's
/// `ExitCode` carries.
fn finish(exit: Exit) -> Result<(), ()> {
    let code = exit.code();
    if code == 0 {
        return Ok(());
    }
    wasip3::cli::exit::exit_with_code(code);
    // exit-with-code does not return; this leg only pacifies the type.
    Err(())
}
