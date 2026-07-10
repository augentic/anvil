//! The guest shim's HTTP transport: the `wasi:http/incoming-handler`
//! export plus this shim's hand-written route table, bridged through
//! `omnia_wasi_http::serve` — making the specify guest a served
//! component. Structurally symmetric with `command.rs`: a transport
//! entry (`struct Http` + `export!` + `Guest::handle`) calling a
//! route-table function.

use omnia_guest::api::{Client, route};
use omnia_guest::{axum, omnia_wasi_http, wasip3};
use workflow::adapter;
use workflow::adapter::handlers::{SourceResolve, TargetResolve};
use workflow::change::plan::handlers::{
    Add as PlanAdd, Amend, Archive, Create as PlanCreate, Next, Remove as PlanRemove, Status,
    Transition as PlanTransition, Validate as PlanValidate,
};
use workflow::init::handlers::Scaffold;
use workflow::journal::handlers::{Emit, Show};
use workflow::orchestrate::handlers::{Author, Build, Execute, Extract, MergeRun, Refine, Survey};
use workflow::registry::handlers::{
    Add as RegistryAdd, Remove as RegistryRemove, Validate as RegistryValidate,
};
use workflow::slice::handlers::{
    ConflictCheck, Create, Drop, ModelShow, Overlap, Preview, Provenance, Prune, TaskMark,
    TaskProgress, TouchedSpecs, Transition, Validate,
};

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
        .route("/init/scaffold", route::post::<Scaffold, Provider>())
        .route("/source/resolve", route::get::<SourceResolve, Provider>())
        .route("/source/{source}/survey", route::post::<Survey, Provider>())
        .route("/source/{source}/extract", route::post::<Extract, Provider>())
        .route("/target/resolve", route::get::<TargetResolve, Provider>())
        .route("/slice/{name}/create", route::post::<Create, Provider>())
        .route("/slice/{name}/validate", route::get::<Validate, Provider>())
        .route("/slice/{name}/provenance", route::get::<Provenance, Provider>())
        .route("/slice/{name}/model", route::get::<ModelShow, Provider>())
        .route("/slice/{name}/refine", route::post::<Refine, Provider>())
        .route("/slice/{name}/build", route::post::<Build, Provider>())
        .route("/slice/{name}/merge", route::post::<MergeRun, Provider>())
        .route("/slice/{name}/merge/preview", route::get::<Preview, Provider>())
        .route("/slice/{name}/merge/conflict-check", route::get::<ConflictCheck, Provider>())
        .route("/slice/{name}/tasks", route::get::<TaskProgress, Provider>())
        .route("/slice/{name}/tasks/{task-number}", route::post::<TaskMark, Provider>())
        .route("/slice/{name}/transition", route::post::<Transition, Provider>())
        .route("/slice/{name}/touched-specs", route::post::<TouchedSpecs, Provider>())
        .route("/slice/{name}/overlap", route::get::<Overlap, Provider>())
        .route("/slice/{name}/drop", route::post::<Drop, Provider>())
        .route("/archive/prune", route::post::<Prune, Provider>())
        .route("/plan/{name}/create", route::post::<PlanCreate, Provider>())
        .route("/plan/validate", route::get::<PlanValidate, Provider>())
        .route("/plan/next", route::post::<Next, Provider>())
        .route("/plan/status", route::get::<Status, Provider>())
        .route("/plan/{name}/add", route::post::<PlanAdd, Provider>())
        .route("/plan/{name}/amend", route::post::<Amend, Provider>())
        .route("/plan/{name}/remove", route::post::<PlanRemove, Provider>())
        .route("/plan/{name}/transition", route::post::<PlanTransition, Provider>())
        .route("/plan/{name}/author", route::post::<Author, Provider>())
        .route("/plan/execute", route::post::<Execute, Provider>())
        .route("/plan/archive", route::post::<Archive, Provider>())
        .route("/journal", route::post::<Emit, Provider>())
        .route("/journal", route::get::<Show, Provider>())
        .route("/registry/validate", route::get::<RegistryValidate, Provider>())
        .route("/registry", route::post::<RegistryAdd, Provider>())
        .route("/registry/{name}/remove", route::post::<RegistryRemove, Provider>())
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
