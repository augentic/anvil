//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! One routing mechanism under two transports, each owned by its
//! `transport` module (`transport::command` / `transport::http`).
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `project::handler::Ctx::load` finds
//! `.specify/project.yaml` exactly as a native run from the project
//! root would. Exit codes pass through verbatim — the command entry maps
//! the route's numeric code onto `wasi:cli/exit#exit-with-code`,
//! preserving the closed exit-code contract.
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

use omnia_guest::api::http;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::wasip3;

use crate::provider::Provider;

struct CliGuest;
wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        let invoker = Invoker::new("specify", Provider);
        let router = transport::command::router(invoker).map_err(|_e| ())?;
        omnia_guest::api::command::execute_wasi(&router).await
    }
}

struct Http;
wasip3::http::service::export!(Http);

impl wasip3::exports::http::handler::Guest for Http {
    async fn handle(
        request: wasip3::http::types::Request,
    ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
        let invoker = Invoker::new("specify", Provider);
        let router = transport::http::router(invoker);
        http::serve(router, request).await
    }
}
