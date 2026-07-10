//! [`NativeProvider`] — the native seam the verb handlers run against.
//!
//! Project anchoring, judgment (delegated to the configured [`Model`]
//! backend), and `SourceSeam` / `TargetSeam` as an in-process dispatch
//! table over the sibling adapter crates' `operations` modules — the
//! native mirror of Omnia's host-mediated dispatch-by-id.
//!
//! The mapping layer between the adapter seam DTOs
//! ([`adapter::seam`]) and the workflow seam DTOs
//! ([`workflow::seam`]) lives here in full, shaped exactly like the
//! guest shim's WIT mapping (`src/provider.rs` at the repo root): the
//! same claim-JSON projection (`payload` / `backing-path` keys) and the
//! same [`BuildReport`] widening, so evidence documents and build
//! reports are byte-compatible across shims.

use std::path::{Path, PathBuf};

use adapter::seam::{self as aseam, Context};
use artifacts::evidence::AuthorityClass;
use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Reply, Request};
use schema::diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use workflow::adapter::metadata::{Metadata, MetadataRequest};
use workflow::adapter::{Axis, BuildInputDeclaration, PlatformsCapability};
use workflow::seam::{self, Evidence, Input, Lead, SourceSeam, TargetSeam, WorkingTree};
use workflow::slice::build::wire::BUILD_VERSION;
use workflow::slice::{BuildOutput, BuildReport, BuildStatus, UiSurface};

/// The native shim's seam provider: every capability the orchestrators
/// need, backed by the linked adapter crates and a native [`Model`].
///
/// Generic over the model backend so the dev binary binds
/// [`crate::model::DevModel`] and tests bind `testkit::MockModel`.
#[derive(Debug)]
pub struct NativeProvider<M> {
    /// The configured project root every project-scoped verb anchors at.
    project_dir: PathBuf,
    /// The judgment backend behind both the orchestrators' own legs and
    /// the adapter operations.
    model: M,
    /// Base URL of the serve-mode listener carrying the `/mcp/<name>`
    /// reference shelves (e.g. `http://127.0.0.1:<port>`); `None` runs
    /// judgment legs without reference grants.
    mcp_base: Option<String>,
}

impl<M> NativeProvider<M> {
    /// A provider anchored at `project_dir` over the given model backend.
    pub fn new(project_dir: impl Into<PathBuf>, model: M) -> Self {
        Self {
            project_dir: project_dir.into(),
            model,
            mcp_base: None,
        }
    }

    /// Attach the reference-shelf base URL (the grant-URL rewrite):
    /// each adapter's judgment context grants
    /// `<base>/mcp/<name>` as its MCP references endpoint.
    #[must_use]
    pub fn mcp_base(mut self, base: impl Into<String>) -> Self {
        self.mcp_base = Some(base.into());
        self
    }

    /// The configured model backend — read access for test suites that
    /// assert on a scripted mock's recorded requests.
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The adapter's granted references URL under the shelf base.
    fn mcp_url(&self, id: &str) -> Option<String> {
        let name = id.rsplit(':').next().unwrap_or(id);
        self.mcp_base.as_ref().map(|base| format!("{base}/mcp/{name}"))
    }
}

impl<M: Send + Sync> workflow::handler::Anchor for NativeProvider<M> {
    fn project_root(&self) -> &Path {
        &self.project_dir
    }
}

impl<M: Model> Model for NativeProvider<M> {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl<M: Model> SourceSeam for NativeProvider<M> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = Context {
            adapter_id: &id,
            project_root: &self.project_dir,
            mcp_url: url.as_deref(),
        };
        let leads = match id.as_str() {
            "source:captures" => captures::operations::survey(&self.model, &ctx).await,
            "source:documentation" => documentation::operations::survey(&self.model, &ctx).await,
            "source:intent" => intent::operations::survey(&self.model, &ctx).await,
            "source:screenshots" => screenshots::operations::survey(&self.model, &ctx).await,
            "source:typescript" => typescript::operations::survey(&self.model, &ctx).await,
            other => return Err(unlinked(other)),
        }
        .map_err(map_error)?;
        Ok(leads.into_iter().map(map_lead).collect())
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = Context {
            adapter_id: &id,
            project_root: &self.project_dir,
            mcp_url: url.as_deref(),
        };
        let lead = aseam::Lead {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
        };
        let evidence = match id.as_str() {
            "source:captures" => captures::operations::extract(&self.model, &ctx, &lead).await,
            "source:documentation" => {
                documentation::operations::extract(&self.model, &ctx, &lead).await
            }
            "source:intent" => intent::operations::extract(&self.model, &ctx, &lead).await,
            "source:screenshots" => {
                screenshots::operations::extract(&self.model, &ctx, &lead).await
            }
            "source:typescript" => typescript::operations::extract(&self.model, &ctx, &lead).await,
            other => return Err(unlinked(other)),
        }
        .map_err(map_error)?;
        Ok(Evidence {
            authority: map_authority(evidence.authority),
            claims: evidence.claims.iter().map(claim_json).collect(),
        })
    }
}

impl<M: Model> TargetSeam for NativeProvider<M> {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        let prompt = match id.as_str() {
            "target:contracts" => contracts::operations::guidance(),
            "target:omnia" => omnia_target::operations::guidance(),
            "target:vectis" => vectis::operations::guidance(),
            other => return Err(unlinked(other)),
        };
        Ok(prompt.to_string())
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = Context {
            adapter_id: &id,
            project_root: &self.project_dir,
            mcp_url: url.as_deref(),
        };
        let inputs: Vec<aseam::Input> = inputs.into_iter().map(map_input).collect();
        let tree = aseam::WorkingTree {
            base: tree.base,
            subpath: tree.subpath,
        };
        let report = match id.as_str() {
            "target:contracts" => {
                contracts::operations::build(&self.model, &ctx, &slice, &inputs, &tree).await
            }
            "target:omnia" => {
                omnia_target::operations::build(&self.model, &ctx, &slice, &inputs, &tree).await
            }
            "target:vectis" => {
                vectis::operations::build(&self.model, &ctx, &slice, &inputs, &tree).await
            }
            other => return Err(unlinked(other)),
        }
        .map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }
}

/// In-process metadata dispatch.
///
/// The resolvers' [`MetadataRequest`] is answered by calling each
/// linked adapter's `operations::metadata()` directly — the native
/// counterpart of the guest shim's WIT-routed runner. Registered by
/// `specify-dev` at startup.
///
/// # Errors
///
/// `adapter-metadata-failed` when the request names an adapter this
/// shim does not link.
pub fn metadata(request: &MetadataRequest<'_>) -> Result<Metadata, Error> {
    match request.axis {
        Axis::Source => {
            let record = match request.adapter_id {
                "source:captures" => captures::operations::metadata(),
                "source:documentation" => documentation::operations::metadata(),
                "source:intent" => intent::operations::metadata(),
                "source:screenshots" => screenshots::operations::metadata(),
                "source:typescript" => typescript::operations::metadata(),
                other => return Err(not_linked(other)),
            };
            Ok(Metadata {
                specify_floor: record.specify_floor,
                inputs: Vec::new(),
                platforms: None,
            })
        }
        Axis::Target => {
            let record = match request.adapter_id {
                "target:contracts" => contracts::operations::metadata(),
                "target:omnia" => omnia_target::operations::metadata(),
                "target:vectis" => vectis::operations::metadata(),
                other => return Err(not_linked(other)),
            };
            Ok(Metadata {
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
            })
        }
    }
}

/// A dispatch to an adapter id this shim does not link.
fn unlinked(id: &str) -> seam::Error {
    seam::Error::InvalidRequest(format!("adapter `{id}` is not linked into the native shim"))
}

/// The metadata-time flavour of [`unlinked`].
fn not_linked(id: &str) -> Error {
    Error::Diag {
        code: "adapter-metadata-failed",
        detail: format!("adapter `{id}` is not linked into the native shim"),
    }
}

/// Adapter seam error → the workflow seam's typed failure
/// (variant-for-variant; both mirror the WIT `types.error`).
fn map_error(error: aseam::Error) -> seam::Error {
    match error {
        aseam::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        aseam::Error::Io(detail) => seam::Error::Io(detail),
        aseam::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

/// Adapter [`aseam::Lead`] → the workflow seam's [`Lead`]
/// (field-for-field).
fn map_lead(lead: aseam::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

/// Adapter [`aseam::Authority`] → the document-level
/// [`AuthorityClass`].
const fn map_authority(authority: aseam::Authority) -> AuthorityClass {
    match authority {
        aseam::Authority::Intent => AuthorityClass::Intent,
        aseam::Authority::Documentation => AuthorityClass::Documentation,
        aseam::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

/// Adapter [`aseam::Claim`] → the open claim JSON object the composed
/// Evidence document carries — the same projection the guest shim
/// applies to the WIT claim record (`payload` for an inline payload,
/// `backing-path` for a filesystem pointer).
fn claim_json(claim: &aseam::Claim) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("kind".into(), claim_kind_str(claim.kind).into());
    if let Some(id) = &claim.id {
        object.insert("id".into(), id.clone().into());
    }
    if let Some(path) = &claim.path {
        object.insert("path".into(), path.clone().into());
    }
    if let Some(synopsis) = &claim.synopsis {
        object.insert("synopsis".into(), synopsis.clone().into());
    }
    match &claim.backing {
        Some(aseam::Backing::Payload(payload)) => {
            object.insert("payload".into(), payload.clone().into());
        }
        Some(aseam::Backing::Path(path)) => {
            object.insert("backing-path".into(), path.clone().into());
        }
        None => {}
    }
    serde_json::Value::Object(object)
}

/// The closed claim-kind enum's schema token.
const fn claim_kind_str(kind: aseam::ClaimKind) -> &'static str {
    match kind {
        aseam::ClaimKind::Intent => "intent",
        aseam::ClaimKind::Requirement => "requirement",
        aseam::ClaimKind::Criterion => "criterion",
        aseam::ClaimKind::Decision => "decision",
        aseam::ClaimKind::Section => "section",
        aseam::ClaimKind::Diagram => "diagram",
        aseam::ClaimKind::Contract => "contract",
        aseam::ClaimKind::Example => "example",
        aseam::ClaimKind::Excerpt => "excerpt",
        aseam::ClaimKind::Type => "type",
        aseam::ClaimKind::Call => "call",
        aseam::ClaimKind::Region => "region",
        aseam::ClaimKind::Container => "container",
        aseam::ClaimKind::Leaf => "leaf",
    }
}

/// Workflow seam [`Input`] → adapter [`aseam::Input`]
/// (variant-for-variant).
fn map_input(input: Input) -> aseam::Input {
    match input {
        Input::Proposal(body) => aseam::Input::Proposal(body),
        Input::Design(body) => aseam::Input::Design(body),
        Input::Tasks(body) => aseam::Input::Tasks(body),
        Input::Spec(body) => aseam::Input::Spec(body),
        Input::Other(body) => aseam::Input::Other(body),
    }
}

/// Widen the compact adapter [`aseam::Report`] into the canonical
/// [`BuildReport`] wire shape the orchestrator's finalize tail
/// schema-gates — the same envelope stamping the guest shim applies.
fn widen_report(id: &str, slice: String, report: aseam::Report) -> BuildReport {
    BuildReport {
        version: BUILD_VERSION,
        slice,
        target: id.strip_prefix("target:").unwrap_or(id).to_string(),
        status: match report.status {
            aseam::Status::Success => BuildStatus::Success,
            aseam::Status::Failure => BuildStatus::Failure,
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

/// Widen one compact adapter [`aseam::Finding`] into a full
/// [`Diagnostic`], mirroring the guest shim: the folded `detail` prose
/// serves as title, impact, and remediation, and an absent `rule-id`
/// stays absent with the fingerprint recomputed after the override.
fn widen_finding(finding: aseam::Finding) -> Diagnostic {
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

/// Adapter [`aseam::Severity`] → the diagnostics [`Severity`].
const fn map_severity(severity: aseam::Severity) -> Severity {
    match severity {
        aseam::Severity::Critical => Severity::Critical,
        aseam::Severity::Important => Severity::Important,
        aseam::Severity::Suggestion => Severity::Suggestion,
        aseam::Severity::Optional => Severity::Optional,
    }
}

/// Adapter [`aseam::Platform`] → the workflow [`Platform`] taxonomy.
const fn map_platform(platform: aseam::Platform) -> workflow::platform::Platform {
    use workflow::platform::Platform;
    match platform {
        aseam::Platform::Core => Platform::Core,
        aseam::Platform::Ios => Platform::Ios,
        aseam::Platform::Android => Platform::Android,
        aseam::Platform::Web => Platform::Web,
        aseam::Platform::Desktop => Platform::Desktop,
    }
}
