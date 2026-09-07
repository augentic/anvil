//! The engine guest
//!
//! The wasm component the shipped runtime embeds and runs. It binds
//! the host's model, storage, and plugin capabilities into one provider and
//! hands the process arguments to the command façade.
//!
//! Running the engine as a guest is what gives Emery its sandbox: the
//! project is mounted read-only, and every effect the engine has goes through
//! a capability the runtime deliberately granted.

#![cfg(target_arch = "wasm32")]

use emery_cli::Response;
use wasip3::cli::environment;

omnia_guest::provider! {
    struct Provider: Model + StateStore + BlobStore + Plugins;
}
impl emery_source::Source for Provider {}

omnia_guest::command!(dispatch);

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments()).await
}
