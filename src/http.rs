//! WASI HTTP export over the shared typed Specify router.

use omnia_guest::api::http;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::wasip3;

use crate::provider::Provider;

struct Http;
wasip3::http::service::export!(Http);

impl wasip3::exports::http::handler::Guest for Http {
    async fn handle(
        request: wasip3::http::types::Request,
    ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
        let router = argv::http::router(Invoker::new("specify", Provider));
        http::serve(router, request).await
    }
}
