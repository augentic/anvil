//! The guest shim's HTTP transport: the `wasi:http/incoming-handler`
//! export plus this shim's hand-written route table, bridged through
//! `omnia_wasi_http::serve` — making the specify guest a served
//! component. Structurally symmetric with `argv.rs`: a transport
//! entry (`struct Http` + `export!` + `Guest::handle`) calling a
//! route-table function.

use omnia_guest::api::{Client, route};
use omnia_guest::{axum, omnia_wasi_http, wasip3};
use workflow::change::plan;
use workflow::{adapter, init, journal, orchestrate, registry, slice};

use crate::provider::Provider;

/// The `wasi:http/incoming-handler` export struct.
struct Http;
wasip3::http::service::export!(Http);

// One line per routed command — GET for pure reads (path + query
// args), POST for writes and judgment (JSON bodies), the noun in
// the path. Grammar leaves with no route here are the provisioning
// commands (native-only; the argv route refuses them too) and
// `completions` (argv-transport sugar). Built inside `handle()` —
// Omnia instantiates a fresh guest per HTTP trigger, so there is
// no cross-request state to amortize with a `static`.
fn router(client: Client<Provider>) -> axum::Router {
    axum::Router::new()
        .route("/init/scaffold", route::post::<init::handlers::Scaffold, Provider>())
        .route("/source/resolve", route::get::<adapter::handlers::SourceResolve, Provider>())
        .route("/source/{source}/survey", route::post::<orchestrate::handlers::Survey, Provider>())
        .route(
            "/source/{source}/extract",
            route::post::<orchestrate::handlers::Extract, Provider>(),
        )
        .route("/target/resolve", route::get::<adapter::handlers::TargetResolve, Provider>())
        .route("/slice/{name}/create", route::post::<slice::handlers::Create, Provider>())
        .route("/slice/{name}/validate", route::get::<slice::handlers::Validate, Provider>())
        .route("/slice/{name}/provenance", route::get::<slice::handlers::Provenance, Provider>())
        .route("/slice/{name}/model", route::get::<slice::handlers::ModelShow, Provider>())
        .route("/slice/{name}/refine", route::post::<orchestrate::handlers::Refine, Provider>())
        .route("/slice/{name}/build", route::post::<orchestrate::handlers::Build, Provider>())
        .route("/slice/{name}/merge", route::post::<orchestrate::handlers::MergeRun, Provider>())
        .route("/slice/{name}/merge/preview", route::get::<slice::handlers::Preview, Provider>())
        .route(
            "/slice/{name}/merge/conflict-check",
            route::get::<slice::handlers::ConflictCheck, Provider>(),
        )
        .route("/slice/{name}/tasks", route::get::<slice::handlers::TaskProgress, Provider>())
        .route(
            "/slice/{name}/tasks/{task-number}",
            route::post::<slice::handlers::TaskMark, Provider>(),
        )
        .route("/slice/{name}/transition", route::post::<slice::handlers::Transition, Provider>())
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
        adapter::metadata::register(crate::provider::metadata);
        let client = Client::new("specify").provider(Provider);
        omnia_wasi_http::serve(router(client), request).await
    }
}
