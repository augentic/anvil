//! The emery engine guest, as a library.
//!
//! One `wit_bindgen::generate!` over the `workflow` world (the
//! `source` / `target` imports Omnia satisfies by host-mediated
//! dispatch), the WIT-backed `Provider` those imports feed, and the
//! `export!` macro that wires the shared typed transport routers
//! onto `wasi:cli/run` + `wasi:http/incoming-handler`.
//!
//! The crate is `wasm32`-only: native builds see an empty crate.
//!
//! A deployment's guest crate is one invocation: `guest::export!();`.
//! The engine's root `emery` cdylib is the sole caller — the wasm
//! examples here and in `augentic/emery-adapters` drive that shipped
//! binary rather than building their own guest.
#![cfg(target_arch = "wasm32")]

mod bindings {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "workflow",
        path: "../../wit",
        generate_all,
    });
}

mod provider;

pub use provider::Provider;
/// Re-exported for the [`export!`] macro expansion.
pub use {omnia_guest, omnia_wasi_otel, transport, wasip3};

/// Export the engine guest from the invoking cdylib.
///
/// Expands the deployment's only `wasi:cli/run` exporter plus the
/// `wasi:http/incoming-handler` service, both routing through the
/// [`Provider`]. The project root is the `"."` mount preopen: WASI
/// resolves relative paths against it, so `project::handler::Ctx::load`
/// finds `.emery/project.yaml` exactly as a native run from the
/// project root would. Exit codes pass through verbatim — the command
/// entry maps the route's numeric code onto
/// `wasi:cli/exit#exit-with-code`, preserving the closed exit-code
/// contract.
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
                let invoker =
                    $crate::omnia_guest::api::invoke::Invoker::new("emery", $crate::Provider);
                let router = $crate::transport::http::router(invoker);
                $crate::omnia_guest::api::http::serve(router, request).await
            }
        }
    };
}
