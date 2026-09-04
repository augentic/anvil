//! The emery engine guest.

#![cfg(target_arch = "wasm32")]

use emery_engine::cli;

omnia_guest::provider! {
    struct Provider: Model + StateStore + BlobStore + Plugins;
}
impl emery_source::Source for Provider {}

omnia_guest::command!(dispatch);

async fn dispatch() -> Result<(), u8> {
    let response = cli::router(Provider).execute(wasip3::cli::environment::get_arguments()).await;
    if response.write_to(&mut std::io::stdout(), &mut std::io::stderr()).is_err() {
        return Err(3);
    }
    if response.exit == 0 { Ok(()) } else { Err(response.exit) }
}
