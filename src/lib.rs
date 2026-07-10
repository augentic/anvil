//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! Argv arrives through wasip3 and parses through the shared `cli`
//! grammar — the exact clap tree the native shim parses, so every
//! shared command is argv- and envelope-compatible across shims. The
//! `dispatch` module owns the exhaustive match over `Commands`,
//! converting each parsed action into the matching handler `Input` DTO
//! and driving the transport-neutral `Handler` against
//! `provider::Provider` — the WIT-backed `Anchor + Model +
//! SourceSeam + TargetSeam` implementation over this world's `source`
//! / `target` imports (satisfied at runtime by Omnia's host-mediated
//! dispatch, routed to the exporting adapter guest by each call's
//! `adapter-id` first argument).
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `workflow::handler::Ctx::load` finds
//! `.specify/project.yaml` exactly as a native run from the project
//! root would. Exit codes pass through verbatim — the `command:`
//! trigger maps the dispatch's numeric code onto
//! `wasi:cli/exit#exit-with-code`, preserving the closed exit-code
//! contract.
//!
//! Alongside `wasi:cli/run`, the guest exports
//! `wasi:http/incoming-handler` (the `http` module): the shim's own
//! hand-written HTTP route table served through
//! `omnia_wasi_http::serve` against the same `provider::Provider`, so
//! every routed command is reachable over both transports with one
//! handler implementation.
#![cfg(target_arch = "wasm32")]

mod bindings {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "workflow",
        path: "wit",
        generate_all,
    });
}

mod dispatch;
mod provider;

omnia_guest::guest!({
    owner: "specify",
    provider: Provider,
    command: crate::dispatch::main,
});

/// The `wasi:http/incoming-handler` export: this shim's hand-written
/// HTTP route table bridged through `omnia_wasi_http::serve`, making
/// the specify guest a served component.
mod http {
    use omnia_guest::api::{Client, route};
    use omnia_guest::{axum, omnia_wasi_http, wasip3};
    use workflow::change::plan;
    use workflow::{adapter, init, journal, orchestrate, registry, slice};

    use crate::provider::Provider;

    struct Http;
    wasip3::http::service::export!(Http);

    // One line per routed command — GET for pure reads (path + query
    // args), POST for writes and judgment (JSON bodies), the noun in
    // the path. Grammar leaves with no route here are the provisioning
    // commands (native-only; the argv dispatch refuses them too) and
    // `completions` (argv-transport sugar). Built inside `handle()` —
    // Omnia instantiates a fresh guest per HTTP trigger, so there is
    // no cross-request state to amortize with a `static`.
    fn router(client: Client<Provider>) -> axum::Router {
        axum::Router::new()
            .route("/init/scaffold", route::post::<init::handlers::Scaffold, Provider>())
            .route("/source/resolve", route::get::<adapter::handlers::SourceResolve, Provider>())
            .route(
                "/source/{source}/survey",
                route::post::<orchestrate::handlers::Survey, Provider>(),
            )
            .route(
                "/source/{source}/extract",
                route::post::<orchestrate::handlers::Extract, Provider>(),
            )
            .route("/target/resolve", route::get::<adapter::handlers::TargetResolve, Provider>())
            .route("/slice/{name}/create", route::post::<slice::handlers::Create, Provider>())
            .route("/slice/{name}/validate", route::get::<slice::handlers::Validate, Provider>())
            .route(
                "/slice/{name}/provenance",
                route::get::<slice::handlers::Provenance, Provider>(),
            )
            .route("/slice/{name}/model", route::get::<slice::handlers::ModelShow, Provider>())
            .route("/slice/{name}/refine", route::post::<orchestrate::handlers::Refine, Provider>())
            .route("/slice/{name}/build", route::post::<orchestrate::handlers::Build, Provider>())
            .route(
                "/slice/{name}/merge",
                route::post::<orchestrate::handlers::MergeRun, Provider>(),
            )
            .route(
                "/slice/{name}/merge/preview",
                route::get::<slice::handlers::Preview, Provider>(),
            )
            .route(
                "/slice/{name}/merge/conflict-check",
                route::get::<slice::handlers::ConflictCheck, Provider>(),
            )
            .route("/slice/{name}/tasks", route::get::<slice::handlers::TaskProgress, Provider>())
            .route(
                "/slice/{name}/tasks/{task-number}",
                route::post::<slice::handlers::TaskMark, Provider>(),
            )
            .route(
                "/slice/{name}/transition",
                route::post::<slice::handlers::Transition, Provider>(),
            )
            .route(
                "/slice/{name}/touched-specs",
                route::post::<slice::handlers::TouchedSpecs, Provider>(),
            )
            .route("/slice/{name}/overlap", route::get::<slice::handlers::Overlap, Provider>())
            .route("/slice/{name}/drop", route::post::<slice::handlers::Drop, Provider>())
            .route("/archive/prune", route::post::<slice::handlers::Prune, Provider>())
            .route("/plan/{name}/create", route::post::<plan::handlers::Create, Provider>())
            .route("/plan/validate", route::get::<plan::handlers::Validate, Provider>())
            .route("/plan/next", route::post::<plan::handlers::Next, Provider>())
            .route("/plan/status", route::get::<plan::handlers::Status, Provider>())
            .route("/plan/{name}/add", route::post::<plan::handlers::Add, Provider>())
            .route("/plan/{name}/amend", route::post::<plan::handlers::Amend, Provider>())
            .route("/plan/{name}/remove", route::post::<plan::handlers::Remove, Provider>())
            .route("/plan/{name}/transition", route::post::<plan::handlers::Transition, Provider>())
            .route("/plan/{name}/author", route::post::<orchestrate::handlers::Author, Provider>())
            .route("/plan/execute", route::post::<orchestrate::handlers::Execute, Provider>())
            .route("/plan/archive", route::post::<plan::handlers::Archive, Provider>())
            .route("/journal", route::post::<journal::handlers::Emit, Provider>())
            .route("/journal", route::get::<journal::handlers::Show, Provider>())
            .route("/registry/validate", route::get::<registry::handlers::Validate, Provider>())
            .route("/registry", route::post::<registry::handlers::Add, Provider>())
            .route("/registry/{name}/remove", route::post::<registry::handlers::Remove, Provider>())
            .with_state(client)
    }

    impl wasip3::exports::http::handler::Guest for Http {
        async fn handle(
            request: wasip3::http::types::Request,
        ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
            adapter::describe::register_describe_runner(crate::provider::describe_runner);
            let client = Client::new("specify").provider(Provider);
            omnia_wasi_http::serve(router(client), request).await
        }
    }
}
