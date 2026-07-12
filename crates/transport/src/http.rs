//! Shared HTTP route table and Specify JSON projection.

use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::http::{DecodeError, Projector, Router, get_with, post_with};
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::axum::response::{IntoResponse, Response};
use omnia_guest::http::StatusCode;
use omnia_guest::http::header::{CONTENT_TYPE, HeaderValue};
use serde::Serialize;
use workflow::adapter::Resolver;
use workflow::handler::Anchor;
use workflow::seam::{SourceSeam, TargetSeam};

/// Specify's JSON HTTP output and error policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpecifyProjector;

impl<O, P> Projector<O, P> for SpecifyProjector
where
    O: Operation<P, Error = workflow::handler::Error>,
    O::Output: Serialize,
    P: Provider,
{
    fn output(&self, output: O::Output) -> Response {
        json(StatusCode::OK, &output)
    }

    fn error(&self, error: O::Error) -> Response {
        let status = match error.core() {
            error::Error::Validation { .. } | error::Error::Argument { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            error::Error::CliTooOld { .. } | error::Error::AdapterCliTooOld { .. } => {
                StatusCode::UPGRADE_REQUIRED
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut body = serde_json::json!({
            "error": error.core().variant_str(),
            "message": error.core().to_string(),
        });
        if let workflow::handler::Error::Report { body: report, .. } = &error {
            match serde_json::to_value(report) {
                Ok(report) => body["report"] = report,
                Err(source) => return encoding(&source),
            }
        }
        json(status, &body)
    }

    fn decode(&self, error: DecodeError) -> Response {
        json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &serde_json::json!({
                "error": "invalid-request",
                "message": error.description(),
            }),
        )
    }
}

fn json(status: StatusCode, value: &impl Serialize) -> Response {
    match serde_json::to_vec(value) {
        Ok(body) => (status, [(CONTENT_TYPE, HeaderValue::from_static("application/json"))], body)
            .into_response(),
        Err(error) => encoding(&error),
    }
}

fn encoding(error: &serde_json::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        serde_json::json!({
            "error": "json-serialize-failed",
            "message": format!("failed to serialize JSON response: {error}"),
        })
        .to_string(),
    )
        .into_response()
}

/// Assemble every HTTP-exposed workflow operation.
#[must_use]
pub fn router<P>(invoker: Invoker<P>) -> Router<P>
where
    P: Provider + Anchor + Model + Resolver + SourceSeam + TargetSeam,
{
    macro_rules! get {
        ($operation:ty) => {
            get_with::<$operation, P, SpecifyProjector>(SpecifyProjector)
        };
    }
    macro_rules! post {
        ($operation:ty) => {
            post_with::<$operation, P, SpecifyProjector>(SpecifyProjector)
        };
    }

    Router::new(invoker)
        .route("/init/scaffold", post!(workflow::init::handlers::Scaffold))
        .route("/source/resolve", get!(workflow::adapter::handlers::SourceResolve))
        .route("/source/{source}/survey", post!(workflow::source::handlers::Survey))
        .route("/source/{source}/extract", post!(workflow::source::handlers::Extract))
        .route("/target/resolve", get!(workflow::adapter::handlers::TargetResolve))
        .route("/slice/{name}/create", post!(workflow::slice::handlers::Create))
        .route("/slice/{name}/validate", get!(workflow::slice::handlers::Validate))
        .route("/slice/{name}/provenance", get!(workflow::slice::handlers::Provenance))
        .route("/slice/{name}/model", get!(workflow::slice::handlers::ModelShow))
        .route("/slice/{name}/refine", post!(workflow::slice::handlers::Refine))
        .route("/slice/{name}/build", post!(workflow::slice::handlers::Build))
        .route("/slice/{name}/merge", post!(workflow::slice::handlers::MergeRun))
        .route("/slice/{name}/merge/preview", get!(workflow::slice::handlers::Preview))
        .route("/slice/{name}/merge/conflict-check", get!(workflow::slice::handlers::ConflictCheck))
        .route("/slice/{name}/tasks", get!(workflow::slice::handlers::TaskProgress))
        .route("/slice/{name}/tasks/{task-number}", post!(workflow::slice::handlers::TaskMark))
        .route("/slice/{name}/transition", post!(workflow::slice::handlers::Transition))
        .route("/slice/{name}/touched-specs", post!(workflow::slice::handlers::TouchedSpecs))
        .route("/slice/{name}/overlap", get!(workflow::slice::handlers::Overlap))
        .route("/slice/{name}/drop", post!(workflow::slice::handlers::Drop))
        .route("/archive/prune", post!(workflow::slice::handlers::Prune))
        .route("/plan/{name}/create", post!(workflow::change::plan::handlers::Create))
        .route("/plan/validate", get!(workflow::change::plan::handlers::Validate))
        .route("/plan/next", post!(workflow::change::plan::handlers::Next))
        .route("/plan/status", get!(workflow::change::plan::handlers::Status))
        .route("/plan/{name}/add", post!(workflow::change::plan::handlers::Add))
        .route("/plan/{name}/amend", post!(workflow::change::plan::handlers::Amend))
        .route("/plan/{name}/remove", post!(workflow::change::plan::handlers::Remove))
        .route("/plan/{name}/transition", post!(workflow::change::plan::handlers::Transition))
        .route("/plan/{name}/author", post!(workflow::change::plan::handlers::Author))
        .route("/plan/execute", post!(workflow::change::plan::handlers::Execute))
        .route("/plan/archive", post!(workflow::change::plan::handlers::Archive))
        .route("/journal", post!(workflow::journal::handlers::Emit))
        .route("/journal", get!(workflow::journal::handlers::Show))
        .route("/registry/validate", get!(workflow::registry::handlers::Validate))
        .route("/registry", post!(workflow::registry::handlers::Add))
        .route("/registry/{name}/remove", post!(workflow::registry::handlers::Remove))
}
