//! The emery engine guest, as a library: the `workflow`-world WIT
//! bindings, the WIT-backed `Provider`, and the `export!` macro.
//! `wasm32`-only — native builds see an empty crate.
#![cfg(target_arch = "wasm32")]

mod bindings {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "workflow",
        path: "wit",
        generate_all,
    });
}

mod provider;

pub use provider::Provider;
/// Re-exported for the [`export!`] macro expansion.
pub use {omnia_guest, omnia_wasi_http, omnia_wasi_otel, transport, wasip3};

/// Export the engine guest from the invoking cdylib.
///
/// Expands the deployment's only `wasi:cli/run` exporter plus the
/// `wasi:http/incoming-handler` service, both routing through the
/// [`Provider`]. WASI resolves relative paths against the `"."`
/// project-root preopen, so config loads behave exactly like a native
/// run from the project root; exit codes pass through verbatim via
/// `wasi:cli/exit#exit-with-code`.
#[macro_export]
macro_rules! export {
    () => {
        struct CliGuest;
        $crate::wasip3::cli::command::export!(CliGuest);

        impl $crate::wasip3::exports::cli::run::Guest for CliGuest {
            async fn run() -> Result<(), ()> {
                use std::io::Write as _;

                // Guest OpenTelemetry over the host's wasi-otel provider;
                // the guard exports buffered telemetry on drop.
                let telemetry = $crate::omnia_wasi_otel::init();
                let invoker =
                    $crate::omnia_guest::api::invoke::Invoker::new("emery", $crate::Provider);
                let router = $crate::transport::command::router(invoker).map_err(|_e| ())?;
                let argv = $crate::wasip3::cli::environment::get_arguments();
                let response = $crate::transport::command::execute(&router, argv).await;
                if std::io::stdout().write_all(&response.stdout).is_err()
                    || std::io::stderr().write_all(&response.stderr).is_err()
                {
                    return Err(());
                }
                if response.exit != 0 {
                    // `exit-with-code` does not return, so export
                    // telemetry before signalling the failure.
                    drop(telemetry);
                    $crate::wasip3::cli::exit::exit_with_code(response.exit);
                }
                Ok(())
            }
        }

        struct Http;
        $crate::wasip3::http::service::export!(Http);

        impl $crate::wasip3::exports::http::handler::Guest for Http {
            async fn handle(
                request: $crate::wasip3::http::types::Request,
            ) -> Result<
                $crate::wasip3::http::types::Response,
                $crate::wasip3::http::types::ErrorCode,
            > {
                // C3: every path answers the typed refusal until an
                // authenticated operator ingress is designed
                // (target-architecture §7).
                $crate::omnia_wasi_http::serve(
                    $crate::transport::http::refusal(),
                    request,
                )
                .await
            }
        }
    };
}
