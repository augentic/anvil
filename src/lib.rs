//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! Argv arrives through wasip3 and parses through the shared `cli`
//! grammar — the exact clap tree the native shim parses, so every
//! shared verb is argv- and envelope-compatible across shims. The
//! `dispatch` module owns the exhaustive match over `Commands`,
//! converting each parsed action into the matching verb input DTO
//! and driving the transport-neutral `Handler` against
//! `provider::Provider` — the WIT-backed `Anchor + Model +
//! SourceSeam + TargetSeam` implementation over this world's `source`
//! / `target` imports (satisfied at runtime by Omnia's host-mediated
//! dispatch, routed to the exporting adapter guest by each call's
//! `adapter-id` first argument).
//!
//! The project root is the `"."` mount preopen: WASI resolves relative
//! paths against it, so `workflow::verb::Ctx::load` finds
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
//! every routed verb is reachable over both transports with one
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
            .route("/init/scaffold", route::post::<init::verbs::Scaffold, Provider>())
            .route("/source/resolve", route::get::<adapter::verbs::SourceResolve, Provider>())
            .route("/source/{source}/survey", route::post::<orchestrate::verbs::Survey, Provider>())
            .route(
                "/source/{source}/extract",
                route::post::<orchestrate::verbs::Extract, Provider>(),
            )
            .route("/target/resolve", route::get::<adapter::verbs::TargetResolve, Provider>())
            .route("/slice/{name}/create", route::post::<slice::verbs::Create, Provider>())
            .route("/slice/{name}/validate", route::get::<slice::verbs::Validate, Provider>())
            .route("/slice/{name}/provenance", route::get::<slice::verbs::Provenance, Provider>())
            .route("/slice/{name}/model", route::get::<slice::verbs::ModelShow, Provider>())
            .route("/slice/{name}/refine", route::post::<orchestrate::verbs::Refine, Provider>())
            .route("/slice/{name}/build", route::post::<orchestrate::verbs::Build, Provider>())
            .route("/slice/{name}/merge", route::post::<orchestrate::verbs::MergeRun, Provider>())
            .route("/slice/{name}/merge/preview", route::get::<slice::verbs::Preview, Provider>())
            .route(
                "/slice/{name}/merge/conflict-check",
                route::get::<slice::verbs::ConflictCheck, Provider>(),
            )
            .route("/slice/{name}/tasks", route::get::<slice::verbs::TaskProgress, Provider>())
            .route(
                "/slice/{name}/tasks/{task-number}",
                route::post::<slice::verbs::TaskMark, Provider>(),
            )
            .route("/slice/{name}/transition", route::post::<slice::verbs::Transition, Provider>())
            .route(
                "/slice/{name}/touched-specs",
                route::post::<slice::verbs::TouchedSpecs, Provider>(),
            )
            .route("/slice/{name}/overlap", route::get::<slice::verbs::Overlap, Provider>())
            .route("/slice/{name}/drop", route::post::<slice::verbs::Drop, Provider>())
            .route("/archive/prune", route::post::<slice::verbs::Prune, Provider>())
            .route("/plan/{name}/create", route::post::<plan::verbs::Create, Provider>())
            .route("/plan/validate", route::get::<plan::verbs::Validate, Provider>())
            .route("/plan/next", route::post::<plan::verbs::Next, Provider>())
            .route("/plan/status", route::get::<plan::verbs::Status, Provider>())
            .route("/plan/{name}/add", route::post::<plan::verbs::Add, Provider>())
            .route("/plan/{name}/amend", route::post::<plan::verbs::Amend, Provider>())
            .route("/plan/{name}/remove", route::post::<plan::verbs::Remove, Provider>())
            .route("/plan/{name}/transition", route::post::<plan::verbs::Transition, Provider>())
            .route("/plan/{name}/author", route::post::<orchestrate::verbs::Author, Provider>())
            .route("/plan/execute", route::post::<orchestrate::verbs::Execute, Provider>())
            .route("/plan/archive", route::post::<plan::verbs::Archive, Provider>())
            .route("/journal", route::post::<journal::verbs::Emit, Provider>())
            .route("/journal", route::get::<journal::verbs::Show, Provider>())
            .route("/registry/validate", route::get::<registry::verbs::Validate, Provider>())
            .route("/registry", route::post::<registry::verbs::Add, Provider>())
            .route("/registry/{name}/remove", route::post::<registry::verbs::Remove, Provider>())
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
