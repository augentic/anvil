//! The emery guest (wasm32) — the deployment's only `wasi:cli/run`
//! exporter. Native deployment policy lives inline in `src/main.rs`;
//! this library carries nothing on native targets.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::io::{self, Write as _};

        use emery_transport::{command, http};
        use wasip3::cli::environment;

        // Bare provider over the WASI capability defaults
        struct Provider;
        impl omnia_guest::Model for Provider {}
        impl emery_adapter::SourceDispatch for Provider {}

        struct Cli;
        wasip3::cli::command::export!(Cli);

        impl wasip3::exports::cli::run::Guest for Cli {
            async fn run() -> Result<(), ()> {
                let telemetry = omnia_wasi_otel::init();
                let resp = command::execute(Provider, environment::get_arguments()).await;

                if io::stdout().write_all(&resp.stdout).is_err()
                    || io::stderr().write_all(&resp.stderr).is_err()
                {
                    return Err(());
                }

                if resp.exit != 0 {
                    drop(telemetry);
                    wasip3::cli::exit::exit_with_code(resp.exit);
                }

                Ok(())
            }
        }

        struct Http;
        wasip3::http::service::export!(Http);

        impl wasip3::exports::http::handler::Guest for Http {
            async fn handle(
                request: wasip3::http::types::Request,
            ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                omnia_wasi_http::serve(http::refusal(), request).await
            }
        }
    }
}
