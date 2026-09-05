//! The emery engine guest.

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
