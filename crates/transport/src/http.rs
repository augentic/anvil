//! Shared HTTP route table and Specify JSON projection.

use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::http::{DecodeError, Projector, Router, get_with, post_with};
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::axum::response::{IntoResponse, Response};
use omnia_guest::http::StatusCode;
use omnia_guest::http::header::{CONTENT_TYPE, HeaderValue};
use project::adapter::{Hydrator, Resolver};
use project::handler::Anchor;
use project::seam::{SourceSeam, TargetSeam};
use serde::Serialize;

/// Specify's JSON HTTP output and error policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpecifyProjector;

impl<O, P> Projector<O, P> for SpecifyProjector
where
    O: Operation<P, Error = project::handler::Error>,
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
        if let project::handler::Error::Report { body: report, .. } = &error {
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
    P: Provider + Anchor + Model + Resolver + Hydrator + SourceSeam + TargetSeam,
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
        .route("/init", post!(project::init::handlers::Init))
        .route("/source/resolve", get!(project::adapter::handlers::SourceResolve))
        .route("/source/{source}/survey", post!(::change::source::Survey))
        .route("/source/{source}/extract", post!(::slice::source::Extract))
        .route("/target/resolve", get!(project::adapter::handlers::TargetResolve))
        .route("/slice", get!(::slice::handlers::List))
        .route("/slice/{name}/validate", get!(::slice::handlers::Validate))
        .route("/slice/{name}/provenance", get!(::slice::handlers::Provenance))
        .route("/slice/{name}/model", get!(::slice::handlers::ModelShow))
        .route("/slice/{name}/refine", post!(::slice::handlers::Refine))
        .route("/slice/{name}/build", post!(::slice::handlers::Build))
        .route("/slice/{name}/merge", post!(::slice::handlers::MergeRun))
        .route("/slice/{name}/merge/preview", get!(::slice::handlers::Preview))
        .route("/slice/{name}/merge/conflict-check", get!(::slice::handlers::ConflictCheck))
        .route("/slice/{name}/drop", post!(::slice::handlers::Drop))
        .route("/archive/prune", post!(::slice::handlers::Prune))
        .route("/plan/validate", get!(::change::plan::handlers::Validate))
        .route("/plan/next", post!(::change::plan::handlers::Next))
        .route("/plan/status", get!(::change::plan::handlers::Status))
        .route("/plan/{name}/add", post!(::change::plan::handlers::Add))
        .route("/plan/{name}/amend", post!(::change::plan::handlers::Amend))
        .route("/plan/{name}/remove", post!(::change::plan::handlers::Remove))
        .route("/plan/{name}/transition", post!(::change::plan::handlers::Transition))
        .route("/plan/{name}/author", post!(::change::plan::handlers::Author))
        .route("/plan/execute", post!(::change::plan::handlers::Execute))
        .route("/plan/archive", post!(::change::plan::handlers::Archive))
        .route("/journal", post!(project::journal::handlers::Emit))
        .route("/journal", get!(project::journal::handlers::Show))
        .route("/registry/validate", get!(project::registry::handlers::Validate))
        .route("/registry", post!(project::registry::handlers::Add))
        .route("/registry/{name}/remove", post!(project::registry::handlers::Remove))
}
