//! The emery engine guest.

#![cfg(target_arch = "wasm32")]

use std::io::{stderr, stdout};

use emery_engine::cli::Cli;
use wasip3::cli::environment;

omnia_guest::provider! {
    struct Provider: Model + StateStore + BlobStore + Plugins;
}
impl emery_source::Source for Provider {}

omnia_guest::command!(dispatch);

async fn dispatch() -> Result<(), u8> {
    let response = Cli::new(Provider).run(environment::get_arguments()).await;
    if response.write_to(&mut stdout(), &mut stderr()).is_err() {
        return Err(3);
    }
    if response.exit == 0 { Ok(()) } else { Err(response.exit) }
}
