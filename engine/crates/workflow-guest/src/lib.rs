//! Workflow-skeleton guest component for the RFC-61 migration.
//!
//! The deployment's only `wasi:cli/run` exporter. Imports the `augentic:specify`
//! `workflow` world's `source` / `target` interfaces, satisfied at runtime by
//! Omnia's host-mediated link dispatch (each call routes to the exporting guest
//! by its `adapter-id` first argument). `run` drives one `survey` call against
//! the echo source adapter and prints each lead to stdout, then proves the
//! `[[mount]]` preopen seam by finding the `"."` entry in the preopen table.
//! Deliberately model-free: the component exists to exercise the runtime
//! seams, not Specify logic.
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
        path: "../../../wit",
        // The seam operations are `async func`s (judgment legs await the
        // async `omnia:model` import mid-call), so the imports async-lower.
        async: true,
    });
}

/// The manifest id the deployment registers the echo source adapter under;
/// host-mediated dispatch routes the `survey` call to it by this first argument.
const ECHO_ADAPTER_ID: &str = "source:echo";

struct CliGuest;
wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        let leads =
            match bindings::augentic::specify::source::survey(ECHO_ADAPTER_ID.to_string()).await {
                Ok(leads) => leads,
                Err(error) => {
                    eprintln!("survey failed: {error:?}");
                    return Err(());
                }
            };
        for lead in leads {
            println!("lead: {} — {}", lead.lead, lead.synopsis);
        }

        // The mount-preopen seam: the host resolves the manifest's `[[mount]]`
        // into this guest's preopen table; finding the `"."` entry proves the
        // seam end to end. Part of the skeleton's contract, so its absence is
        // a hard failure.
        if wasip3::filesystem::preopens::get_directories().iter().any(|(_, name)| name == ".") {
            println!("mount: . ok");
            Ok(())
        } else {
            eprintln!("mount `.` missing from the preopen table");
            Err(())
        }
    }
}
