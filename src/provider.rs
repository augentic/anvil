//! WIT-backed capabilities used by workflow orchestrators.
//!
//! Mappings live here so workflow code remains wasm-free. Claims stay
//! raw JSON to preserve open per-kind fields. Compact build reports are
//! widened with caller-owned envelope fields before validation.

use std::future::Future;

use artifacts::evidence::AuthorityClass;
use error::Error;
use project::adapter::metadata::{Metadata, Request};
use project::adapter::{
    AdapterRef, Axis, BuildInputDeclaration, PlatformsCapability, ResolvedSource, ResolvedTarget,
    Resolver,
};
use project::seam::{self, Evidence, Input, Lead, MergePhase, SourceSeam, TargetSeam, WorkingTree};
use schema::diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use slice::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
use wasip3::http_compat::IncomingMessage as _;

use crate::bindings::specify::adapter::{source, target, types};

/// Workflow capabilities backed by the world's WIT imports.
pub struct Provider;

impl omnia_guest::Model for Provider {}

impl project::handler::Anchor for Provider {
    fn project_root(&self) -> &std::path::Path {
        std::path::Path::new(".")
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, project_dir: &std::path::Path,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::resolver::Component::new(metadata)
            .resolve_source(adapter_ref, project_dir)
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, project_dir: &std::path::Path,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::resolver::Component::new(metadata)
            .resolve_target(adapter_ref, project_dir)
    }
}

impl project::adapter::Hydrator for Provider {
    // Straight `wasi:http/client` send — deliberately not
    // `omnia_wasi_http::handle`, whose keyvalue-backed cache would add
    // a `wasi:keyvalue` import no specify deployment links.
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send {
        let url = url.to_string();
        async move {
            let diag = |detail: String| Error::Diag {
                code: "http-fetch",
                detail,
            };
            let request = omnia_guest::http::Request::get(&url)
                .body(omnia_guest::axum::body::Body::empty())
                .map_err(|err| diag(format!("building the request for {url}: {err}")))?;
            let request = wasip3::http_compat::http_into_wasi_request(request)
                .map_err(|err| diag(format!("converting the request for {url}: {err}")))?;
            let response = wasip3::http::client::send(request)
                .await
                .map_err(|err| diag(format!("fetching {url}: {err}")))?;
            let response = wasip3::http_compat::http_from_wasi_response(response)
                .map_err(|err| diag(format!("reading the response from {url}: {err}")))?;
            if !response.status().is_success() {
                return Err(diag(format!("fetching {url}: HTTP {}", response.status())));
            }
            let (_, mut body) = response.into_parts();
            let Some(wasi_response) = body.take_unstarted() else {
                return Ok(Vec::new());
            };
            let (_, body_rx) = wasip3::wit_future::new(|| Ok(()));
            let (stream, _trailers) = wasi_response.consume_body(body_rx);
            Ok(stream.collect().await)
        }
    }
}

impl SourceSeam for Provider {
    fn survey(&self, id: String) -> impl Future<Output = Result<Vec<Lead>, seam::Error>> + Send {
        async move {
            let leads = source::survey(id).await.map_err(map_error)?;
            Ok(leads.into_iter().map(map_lead).collect())
        }
    }

    fn extract(
        &self, id: String, lead: Lead,
    ) -> impl Future<Output = Result<Evidence, seam::Error>> + Send {
        async move {
            let wire = source::Lead {
                lead: lead.lead,
                synopsis: lead.synopsis,
                topics: lead.topics,
            };
            let evidence = source::extract(id, wire).await.map_err(map_error)?;
            Ok(Evidence {
                authority: map_authority(evidence.authority),
                claims: evidence.claims.into_iter().map(claim_json).collect(),
            })
        }
    }
}

impl TargetSeam for Provider {
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, seam::Error>> + Send {
        async move { target::guidance(id).await.map_err(map_error) }
    }

    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, seam::Error>> + Send {
        async move {
            let wire_inputs = inputs.into_iter().map(map_input).collect();
            let wire_tree = target::WorkingTree {
                base: tree.base,
                subpath: tree.subpath,
            };
            let report = target::build(id.clone(), slice.clone(), wire_inputs, wire_tree)
                .await
                .map_err(map_error)?;
            Ok(widen_report(&id, slice, report))
        }
    }

    fn merge(
        &self, id: String, slice: String, phase: MergePhase, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, seam::Error>> + Send {
        async move {
            let wire_phase = match phase {
                MergePhase::Preflight => target::MergePhase::Preflight,
                MergePhase::Postflight => target::MergePhase::Postflight,
            };
            let wire_tree = target::WorkingTree {
                base: tree.base,
                subpath: tree.subpath,
            };
            let report = target::merge(id.clone(), slice.clone(), wire_phase, wire_tree)
                .await
                .map_err(map_error)?;
            Ok(widen_report(&id, slice, report))
        }
    }
}

/// Resolve metadata through the deployed adapter identified by the request.
///
/// Dispatch is by adapter id rather than component path; deployment
/// assembly uses the same resolver precedence.
///
/// # Errors
///
/// Reserved for the resolver callback contract; WIT metadata has no
/// error channel.
pub fn metadata(request: &Request<'_>) -> Result<Metadata, Error> {
    Ok(match request.axis {
        Axis::Source => {
            let record = source::metadata(request.adapter_id);
            Metadata {
                specify_floor: record.specify_floor,
                inputs: Vec::new(),
                platforms: None,
            }
        }
        Axis::Target => {
            let record = target::metadata(request.adapter_id);
            Metadata {
                specify_floor: record.specify_floor,
                inputs: record
                    .inputs
                    .into_iter()
                    .map(|input| BuildInputDeclaration {
                        path: input.path,
                        required: input.required,
                    })
                    .collect(),
                platforms: record.platforms.map(|capability| PlatformsCapability {
                    required: capability.required,
                    allowed: capability.allowed.into_iter().map(map_platform).collect(),
                    default: capability.default.into_iter().map(map_platform).collect(),
                }),
            }
        }
    })
}

fn map_error(error: types::Error) -> seam::Error {
    match error {
        types::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        types::Error::Io(detail) => seam::Error::Io(detail),
        types::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

fn map_lead(lead: source::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

const fn map_authority(authority: source::Authority) -> AuthorityClass {
    match authority {
        source::Authority::Intent => AuthorityClass::Intent,
        source::Authority::Documentation => AuthorityClass::Documentation,
        source::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// Preserve open claim fields in their evidence-schema representation.
///
/// A backing path uses `backing-path` to remain distinct from the claim
/// anchor's `path`.
fn claim_json(claim: source::Claim) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("kind".into(), claim_kind_str(claim.kind).into());
    if let Some(id) = claim.id {
        object.insert("id".into(), id.into());
    }
    if let Some(path) = claim.path {
        object.insert("path".into(), path.into());
    }
    if let Some(synopsis) = claim.synopsis {
        object.insert("synopsis".into(), synopsis.into());
    }
    match claim.backing {
        Some(source::Backing::Payload(payload)) => {
            object.insert("payload".into(), payload.into());
        }
        Some(source::Backing::Path(path)) => {
            object.insert("backing-path".into(), path.into());
        }
        None => {}
    }
    serde_json::Value::Object(object)
}

const fn claim_kind_str(kind: source::ClaimKind) -> &'static str {
    match kind {
        source::ClaimKind::Intent => "intent",
        source::ClaimKind::Requirement => "requirement",
        source::ClaimKind::Criterion => "criterion",
        source::ClaimKind::Decision => "decision",
        source::ClaimKind::Section => "section",
        source::ClaimKind::Diagram => "diagram",
        source::ClaimKind::Contract => "contract",
        source::ClaimKind::Example => "example",
        source::ClaimKind::Excerpt => "excerpt",
        source::ClaimKind::Type => "type",
        source::ClaimKind::Call => "call",
        source::ClaimKind::Region => "region",
        source::ClaimKind::Container => "container",
        source::ClaimKind::Leaf => "leaf",
    }
}

fn map_input(input: Input) -> target::Input {
    match input {
        Input::Proposal(body) => target::Input::Proposal(body),
        Input::Design(body) => target::Input::Design(body),
        Input::Tasks(body) => target::Input::Tasks(body),
        Input::Spec(body) => target::Input::Spec(body),
        Input::Other(body) => target::Input::Other(body),
    }
}

/// Add caller-owned envelope fields required by the build-report schema.
fn widen_report(id: &str, slice: String, report: target::Report) -> BuildReport {
    BuildReport {
        version: BUILD_VERSION,
        slice,
        target: id.strip_prefix("target:").unwrap_or(id).to_string(),
        status: match report.status {
            target::Status::Success => BuildStatus::Success,
            target::Status::Failure => BuildStatus::Failure,
        },
        findings: report.findings.into_iter().map(widen_finding).collect(),
        outputs: report
            .outputs
            .into_iter()
            .map(|output| BuildOutput {
                platform: map_platform(output.platform),
                path: output.path,
            })
            .collect(),
        ui_surface: report.ui_surface.map(|surface| UiSurface {
            screens: surface.screens,
        }),
    }
}

/// Restore a canonical fingerprint after preserving an absent rule id.
fn widen_finding(finding: target::Finding) -> Diagnostic {
    let mut diagnostic = Diagnostic::finding(
        finding.rule_id.clone().unwrap_or_else(|| "target-build-finding".to_string()),
        finding.detail.clone(),
        finding.detail,
        map_severity(finding.severity),
        DiagnosticKind::Violation,
        DiagnosticSource::ModelAssisted,
        Artifact::Code,
        None,
    );
    diagnostic.rule_id = finding.rule_id;
    diagnostic.fingerprint = schema::diagnostics::fingerprint(&diagnostic);
    diagnostic
}

const fn map_severity(severity: target::Severity) -> Severity {
    match severity {
        target::Severity::Critical => Severity::Critical,
        target::Severity::Important => Severity::Important,
        target::Severity::Suggestion => Severity::Suggestion,
        target::Severity::Optional => Severity::Optional,
    }
}

const fn map_platform(platform: target::Platform) -> project::platform::Platform {
    use project::platform::Platform;
    match platform {
        target::Platform::Core => Platform::Core,
        target::Platform::Ios => Platform::Ios,
        target::Platform::Android => Platform::Android,
        target::Platform::Web => Platform::Web,
        target::Platform::Desktop => Platform::Desktop,
    }
}
