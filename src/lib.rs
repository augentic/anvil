//! The emery guest (wasm32) — the deployment's only `wasi:cli/run`
//! exporter — or, on native, the `launcher` deployment-policy module
//! the binary and the journey host share (ADR-0011).

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        pub mod launcher;
    } else {
        mod bindings {
            #![allow(missing_docs)]

            wit_bindgen::generate!({
                world: "engine",
                path: "wit",
                inline: r#"
                    package emery:engine;
                    world engine {
                        import emery:adapter/source@0.1.0;
                    }
                "#,
                generate_all,
            });
        }

        mod provider;

        struct CliGuest;
        wasip3::cli::command::export!(CliGuest);

        impl wasip3::exports::cli::run::Guest for CliGuest {
            async fn run() -> Result<(), ()> {
                use std::io::Write as _;

                // Guest OpenTelemetry over the host's wasi-otel provider;
                // the guard exports buffered telemetry on drop.
                let telemetry = omnia_wasi_otel::init();
                let invoker =
                    omnia_guest::api::invoke::Invoker::new("emery", provider::Provider);
                let router = emery_transport::command::router(invoker).map_err(|_e| ())?;
                let argv = wasip3::cli::environment::get_arguments();
                let response = emery_transport::command::execute(&router, argv).await;
                if std::io::stdout().write_all(&response.stdout).is_err()
                    || std::io::stderr().write_all(&response.stderr).is_err()
                {
                    return Err(());
                }
                if response.exit != 0 {
                    // `exit-with-code` does not return, so export telemetry first.
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
                // C3: every path answers the typed refusal until an
                // authenticated operator ingress is designed.
                omnia_wasi_http::serve(emery_transport::http::refusal(), request).await
            }
        }
    }
}
