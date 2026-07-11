//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! One routing mechanism under two transports, each owned by a
//! symmetric transport module (see `rfcs/handler-routing.md`):
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `workflow::handler::Ctx::load` finds
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

mod command;
mod http;
mod provider;
