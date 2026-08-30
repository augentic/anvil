//! Wasm32 engine guest exporting the deployment's CLI entry point.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use emery_engine::cli;
        use omnia_guest::api::command;

        omnia_guest::provider! {
            struct Provider: Model + StateStore + BlobStore + Plugins;
        }
        impl emery_adapter::Source for Provider {}

        struct Cli;
        wasip3::cli::command::export!(Cli);

        impl wasip3::exports::cli::run::Guest for Cli {
            async fn run() -> Result<(), ()> {
                command::execute_wasi(dispatch()).await
            }
        }

        async fn dispatch() -> Result<(), u8> {
            let response = cli::router(Provider)
                .execute(wasip3::cli::environment::get_arguments())
                .await;
            if response.write_to(&mut std::io::stdout(), &mut std::io::stderr()).is_err() {
                return Err(3);
            }
            if response.exit == 0 { Ok(()) } else { Err(response.exit) }
        }
    }
}
