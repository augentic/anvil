//! [`Provider`] — the WIT-backed seam the orchestrator verbs run
//! against: `omnia_guest::Model` (judgment through
//! `omnia:model/completion`, the wasm32 default body) plus
//! `SourceSeam` / `TargetSeam` over this world's `source` / `target`
//! imports.
//!
//! The mapping layer between the WIT records and the seam DTOs lives
//! here in full, so `workflow-lib` and `dispatch` stay
//! wasm-clean. Two mappings are judgment-free but deliberately shaped:
//!
//! - **Claims** cross as raw JSON objects (the evidence schema leaves
//!   per-kind body fields open). The compact WIT claim's fields land
//!   under their schema keys (`kind`, `id`, `path`, `synopsis`); the
//!   `backing` variant lands as `payload` for an inline payload and as
//!   `backing-path` for a filesystem pointer — distinct from the
//!   anchor `path` key, which the schema's grammar owns.
//! - **The build report** widens the compact WIT `report` into the
//!   canonical [`BuildReport`] wire shape: `version` is pinned to
//!   [`BUILD_VERSION`], `slice` / `target` are stamped from the call's
//!   own arguments (the WIT report omits envelope keys the caller
//!   already knows — lossy only in that the adapter's `name@version`
//!   identity collapses to the plan-bound name), and each compact
//!   finding widens into a full [`Diagnostic`] via
//!   [`Diagnostic::finding`] with the folded `detail` prose serving as
//!   title, impact, and remediation.

use std::future::Future;

use artifacts::evidence::AuthorityClass;
use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Error;
use workflow_lib::adapter::describe::{DescribeAnswer, DescribeRequest};
use workflow_lib::adapter::{Axis, BuildInputDeclaration, PlatformsCapability};
use workflow_lib::seam::{self, Evidence, Input, Lead, SourceSeam, TargetSeam, WorkingTree};
use workflow_lib::slice::build::wire::BUILD_VERSION;
use workflow_lib::slice::{BuildOutput, BuildReport, BuildStatus, UiSurface};

use crate::bindings::specify::adapter::{source, target, types};

/// The workflow guest's seam provider: every capability the
/// orchestrators need, backed by the world's WIT imports.
pub struct Provider;

/// `Model` rides the wasm32 default body — judgment calls go straight
/// to `omnia:model/completion` with the `"."` preopen lend resolved at
/// the call site.
impl omnia_guest::Model for Provider {}

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
}

/// In-guest describe dispatch: the resolver's [`DescribeRequest`]
/// routed through this world's `source` / `target` imports by the
/// request's `adapter-id` (Omnia's host-mediated dispatch reaches the
/// deployment guest exporting that id). Registered by the guest shim
/// at startup, so `SourceAdapter::resolve` / `TargetAdapter::resolve`
/// work for forwarded verbs against the read-only store and cache
/// mounts.
///
/// Routing is by name, not by file: the answer comes from the
/// *deployed* component with the request's id. Drive-time deployment
/// assembly uses the same resolvers with the same precedence, so the
/// deployed component and the resolver-located file agree for every
/// bound or cached adapter; an unbound pin resolved diagnostically
/// answers from its deployed namesake (same adapter family — the
/// identity fields never come from describe).
///
/// # Errors
///
/// Infallible today: WIT `describe` carries no error channel, so a
/// dispatch to an id absent from the deployment fails at the Omnia
/// seam, not here.
pub fn describe_runner(request: &DescribeRequest<'_>) -> Result<DescribeAnswer, Error> {
    Ok(match request.axis {
        Axis::Source => {
            let manifest = source::describe(request.adapter_id);
            DescribeAnswer {
                specify_floor: manifest.specify_floor,
                inputs: Vec::new(),
                platforms: None,
            }
        }
        Axis::Target => {
            let manifest = target::describe(request.adapter_id);
            DescribeAnswer {
                specify_floor: manifest.specify_floor,
                inputs: manifest
                    .inputs
                    .into_iter()
                    .map(|input| BuildInputDeclaration {
                        path: input.path,
                        required: input.required,
                    })
                    .collect(),
                platforms: manifest.platforms.map(|capability| PlatformsCapability {
                    required: capability.required,
                    allowed: capability.allowed.into_iter().map(map_platform).collect(),
                    default: capability.default.into_iter().map(map_platform).collect(),
                }),
            }
        }
    })
}

/// WIT `types.error` → the seam's typed failure. Shared by both axes:
/// `source` and `target` alias the same `types.error`.
fn map_error(error: types::Error) -> seam::Error {
    match error {
        types::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        types::Error::Io(detail) => seam::Error::Io(detail),
        types::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// WIT `source.lead` → the seam's [`Lead`] (field-for-field).
fn map_lead(lead: source::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// WIT `source.authority` → the document-level [`AuthorityClass`].
const fn map_authority(authority: source::Authority) -> AuthorityClass {
    match authority {
        source::Authority::Intent => AuthorityClass::Intent,
        source::Authority::Documentation => AuthorityClass::Documentation,
        source::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// WIT `source.claim` → the open claim JSON object the composed
/// Evidence document carries (see the module docs for the `backing`
/// key mapping).
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

/// The closed claim-kind enum's schema token.
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

/// Seam [`Input`] → WIT `target.input` (variant-for-variant).
fn map_input(input: Input) -> target::Input {
    match input {
        Input::Proposal(body) => target::Input::Proposal(body),
        Input::Design(body) => target::Input::Design(body),
        Input::Tasks(body) => target::Input::Tasks(body),
        Input::Spec(body) => target::Input::Spec(body),
        Input::Other(body) => target::Input::Other(body),
    }
}

/// Widen the compact WIT `target.report` into the canonical
/// [`BuildReport`] wire shape the orchestrator's finalize tail
/// schema-gates (see the module docs for the envelope stamping).
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

/// Widen one compact WIT `target.finding` into a full [`Diagnostic`]:
/// the folded `detail` prose serves as title, impact, and remediation
/// (the adapter folded the full diagnostic's prose into it), the
/// producer is the judgment leg (`model-assisted`), and an absent
/// `rule-id` stays absent — the fingerprint is recomputed after the
/// override so it stays canonical.
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
    diagnostic.fingerprint = diagnostics::fingerprint(&diagnostic);
    diagnostic
}

/// WIT `target.severity` → the diagnostics [`Severity`].
const fn map_severity(severity: target::Severity) -> Severity {
    match severity {
        target::Severity::Critical => Severity::Critical,
        target::Severity::Important => Severity::Important,
        target::Severity::Suggestion => Severity::Suggestion,
        target::Severity::Optional => Severity::Optional,
    }
}

/// WIT `target.platform` → the workflow [`Platform`] taxonomy.
const fn map_platform(platform: target::Platform) -> workflow_lib::platform::Platform {
    use workflow_lib::platform::Platform;
    match platform {
        target::Platform::Core => Platform::Core,
        target::Platform::Ios => Platform::Ios,
        target::Platform::Android => Platform::Android,
        target::Platform::Web => Platform::Web,
        target::Platform::Desktop => Platform::Desktop,
    }
}
