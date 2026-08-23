//! Wasm32 engine guest exporting the deployment's CLI and HTTP entry points.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use emery_transport::{command, http};
        use omnia_guest::api::invoke::Invoker;

        // The bare provider uses imported model and source capabilities.
        // Host-bound key-value and blobstore capabilities provide durable
        // engine storage through the default trait bodies.
        #[derive(Clone)]
        struct Provider;
        impl omnia_guest::Model for Provider {}
        impl emery_adapter::Source for Provider {}
        impl omnia_guest::StateStore for Provider {}
        impl omnia_guest::BlobStore for Provider {}

        struct Cli;
        wasip3::cli::command::export!(Cli);

        impl wasip3::exports::cli::run::Guest for Cli {
            async fn run() -> Result<(), ()> {
                let router = command::router(Invoker::new("emery", Provider)).map_err(drop)?;
                omnia_guest::api::command::execute_wasi(&router).await
            }
        }

        struct Http;
        wasip3::http::service::export!(Http);

        impl wasip3::exports::http::handler::Guest for Http {
            async fn handle(
                request: wasip3::http::types::Request,
            ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                omnia_wasi_http::serve(http::listener(Provider), request).await
            }
        }
    }
}
