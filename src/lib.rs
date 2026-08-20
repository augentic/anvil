//! The emery guest (wasm32) — the deployment's only `wasi:cli/run`
//! exporter. Native deployment policy lives inline in `src/main.rs`;
//! this library carries nothing on native targets.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use omnia_guest::api::invoke::Invoker;
        use emery_transport::{command, http};
        use wasip3::cli::environment;

        // Bare provider over the WASI capability defaults: paths are
        // preopen-relative constants and adapter dispatch rides the
        // WIT imports, so no domain capabilities are needed.
        #[derive(Clone, Copy, Debug)]
        struct Engine;

        impl omnia_guest::Model for Engine {}

        struct CliGuest;
        wasip3::cli::command::export!(CliGuest);

        impl wasip3::exports::cli::run::Guest for CliGuest {
            async fn run() -> Result<(), ()> {
                use std::io::Write as _;

                let telemetry = omnia_wasi_otel::init();

                let invoker = Invoker::new("emery", Engine);
                let router = command::router(invoker).map_err(|_e| ())?;
                let argv = environment::get_arguments();
                let response = command::execute(&router, argv).await;

                if std::io::stdout().write_all(&response.stdout).is_err()
                    || std::io::stderr().write_all(&response.stderr).is_err()
                {
                    return Err(());
                }

                if response.exit != 0 {
                    drop(telemetry);
                    wasip3::cli::exit::exit_with_code(response.exit);
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
