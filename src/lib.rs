//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! One routing mechanism under two transports, each owned by a
//! symmetric transport module (see `rfcs/handler-routing.md`):
//!
//! - `argv` — the explicit `wasi:cli/run` export. Argv arrives through
//!   wasip3 and parses through the shared `cli` grammar — the exact
//!   clap tree the native shim parses, so every shared command is
//!   argv- and envelope-compatible across shims. The route match
//!   converts each parsed action into the matching handler `Input`
//!   DTO and drives the transport-neutral `Handler` against
//!   `provider::Provider` — the WIT-backed `Anchor + Model +
//!   SourceSeam + TargetSeam` implementation over this world's
//!   `source` / `target` imports (satisfied at runtime by Omnia's
//!   host-mediated dispatch, routed to the exporting adapter guest by
//!   each call's `adapter-id` first argument).
//! - `http` — the explicit `wasi:http/incoming-handler` export: the
//!   shim's own hand-written HTTP route table served through
//!   `omnia_wasi_http::serve` against the same `provider::Provider`,
//!   so every routed command is reachable over both transports with
//!   one handler implementation.
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `workflow::handler::Ctx::load` finds
//! `.specify/project.yaml` exactly as a native run from the project
//! root would. Exit codes pass through verbatim — the argv entry maps
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

mod argv;
mod http;
mod provider;
