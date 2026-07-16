//! Native seam provider: project anchoring, judgment, and adapter dispatch.
//! Maps adapter seam DTOs onto workflow seam DTOs like the wasm guest shim.

use std::path::{Path, PathBuf};

use adapter::seam::{self as aseam, Context};
use artifacts::evidence::AuthorityClass;
use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Error;
use omnia_guest::Model;
use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::model::{Reply, Request};
use project::adapter::{AdapterRef, Axis, Origin, ResolvedSource, ResolvedTarget, Resolver};
use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
use project::seam::{self, Evidence, Input, Lead, Source, Target, WorkingTree};

use crate::catalog::{Binding, Catalog, Entry};
use crate::mcp;

/// Native shim provider over a linked-adapter [`Catalog`] and a
/// [`Model`] backend.
#[derive(Debug)]
pub struct Provider<M> {
    project_dir: PathBuf,
    model: M,
    catalog: Catalog<M>,
    mcp_base: Option<String>,
}

impl<M: Clone> Clone for Provider<M> {
    fn clone(&self) -> Self {
        Self {
            project_dir: self.project_dir.clone(),
            model: self.model.clone(),
            catalog: self.catalog.clone(),
            mcp_base: self.mcp_base.clone(),
        }
    }
}

impl<M> Provider<M> {
    /// A provider anchored at `project_dir` over the given model backend
    /// and linked-adapter catalog.
    pub fn new(project_dir: impl Into<PathBuf>, model: M, catalog: Catalog<M>) -> Self {
        Self {
            project_dir: project_dir.into(),
            model,
            catalog,
            mcp_base: None,
        }
    }

    /// Attach the reference-shelf base URL for MCP grant rewrite.
    #[must_use]
    pub fn mcp_base(mut self, base: impl Into<String>) -> Self {
        self.mcp_base = Some(base.into());
        self
    }

    /// A provider over `B`'s catalog with its reference shelves served
    /// on an ephemeral background listener (skipped when no port can
    /// be bound).
    pub async fn bound<B: Binding>(root: impl Into<PathBuf>, model: M) -> Self
    where
        M: Model,
    {
        let catalog = B::catalog();
        let base = mcp::ephemeral_base(&catalog).await;
        let mut provider = Self::new(root, model, catalog);
        if let Some(base) = base {
            provider = provider.mcp_base(base);
        }
        provider
    }

    /// The configured model backend.
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The linked-adapter catalog.
    pub const fn catalog(&self) -> &Catalog<M> {
        &self.catalog
    }

    fn mcp_url(&self, id: &str) -> Option<String> {
        let name = id.rsplit(':').next().unwrap_or(id);
        self.mcp_base.as_ref().map(|base| format!("{base}/mcp/{name}"))
    }
}

impl<M: Send + Sync + 'static> project::handler::Anchor for Provider<M> {
    fn project_root(&self) -> &Path {
        &self.project_dir
    }
}

impl<M: Send + Sync> Resolver for Provider<M> {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        require_bare(adapter_ref)?;
        let entry = self.catalog.get(Axis::Source, &adapter_ref.name)?;
        project::adapter::resolver::source(adapter_ref, entry.metadata(), origin(&entry))
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, _project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        require_bare(adapter_ref)?;
        let entry = self.catalog.get(Axis::Target, &adapter_ref.name)?;
        project::adapter::resolver::target(adapter_ref, entry.metadata(), origin(&entry))
    }
}

impl<M: Send + Sync> project::adapter::Hydrator for Provider<M> {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        Err(Error::Diag {
            code: "adapter-hydrate-unavailable",
            detail: format!(
                "the native harness links adapters directly and fetches nothing (requested {url})"
            ),
        })
    }
}

impl<M: Model> Model for Provider<M> {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

impl<M: Model> Source for Provider<M> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = Context {
            adapter_id: &id,
            project_root: &self.project_dir,
            mcp_url: url.as_deref(),
        };
        let leads = self.catalog.survey(&self.model, &ctx, &id).await.map_err(map_error)?;
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
        let evidence =
            self.catalog.extract(&self.model, &ctx, &id, &lead).await.map_err(map_error)?;
        Ok(Evidence {
            authority: map_authority(evidence.authority),
            claims: evidence.claims.into_iter().map(map_claim).collect(),
        })
    }
}

impl<M: Model> Target for Provider<M> {
    async fn guidance(&self, id: String) -> Result<String, seam::Error> {
        let prompt = self.catalog.guidance(&id).map_err(map_error)?;
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
        let report = self
            .catalog
            .build(&self.model, &ctx, &id, &slice, &inputs, &tree)
            .await
            .map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }

    async fn merge(
        &self, id: String, slice: String, phase: seam::MergePhase, tree: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        let url = self.mcp_url(&id);
        let ctx = Context {
            adapter_id: &id,
            project_root: &self.project_dir,
            mcp_url: url.as_deref(),
        };
        let phase = match phase {
            seam::MergePhase::Preflight => aseam::MergePhase::Preflight,
            seam::MergePhase::Postflight => aseam::MergePhase::Postflight,
        };
        let tree = aseam::WorkingTree {
            base: tree.base,
            subpath: tree.subpath,
        };
        let report = self
            .catalog
            .merge(&self.model, &ctx, &id, &slice, phase, &tree)
            .await
            .map_err(map_error)?;
        Ok(widen_report(&id, slice, report))
    }
}

/// Invoke one operation against the provider.
///
/// Typed operation execution alongside the CLI-argv path in
/// [`crate::command`], so a trial can inspect an operation's typed
/// output (e.g. the execute body's phases) without reimplementing
/// router machinery. The operation type leads the generics so call
/// sites write `run::<Op, _, _>(&provider, …)`.
///
/// # Errors
///
/// Propagates the operation's typed failure.
pub async fn run<R, B, M>(
    provider: &Provider<M>, input: R::Input,
) -> Result<B, project::handler::Error>
where
    M: Clone + Send + Sync + 'static,
    R: Operation<Provider<M>, Output = B, Error = project::handler::Error>,
    B: Send,
{
    Invoker::new("specify", provider.clone()).invoke::<R>(Invocation::new(input)).await
}

fn require_bare(adapter_ref: &AdapterRef) -> Result<(), Error> {
    if adapter_ref.version.is_none() {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "native adapter resolution accepts bare development identities only; \
             `{}` is pinned and must resolve through the component deployment",
            adapter_ref.name
        ),
    })
}

fn origin<M>(entry: &Entry<M>) -> Origin {
    Origin {
        label: "native".to_string(),
        reference: format!("rust:{}", entry.id()),
    }
}

fn map_error(error: aseam::Error) -> seam::Error {
    match error {
        aseam::Error::InvalidRequest(detail) => seam::Error::InvalidRequest(detail),
        aseam::Error::Io(detail) => seam::Error::Io(detail),
        aseam::Error::Internal(detail) => seam::Error::Internal(detail),
    }
}

fn map_lead(lead: aseam::Lead) -> Lead {
    Lead {
        lead: lead.lead,
        synopsis: lead.synopsis,
        topics: lead.topics,
    }
}

const fn map_authority(authority: aseam::Authority) -> AuthorityClass {
    match authority {
        aseam::Authority::Intent => AuthorityClass::Intent,
        aseam::Authority::Documentation => AuthorityClass::Documentation,
        aseam::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

// Open per-kind claim fields do not cross the compact seam record.
fn map_claim(claim: aseam::Claim) -> artifacts::evidence::Claim {
    let mut typed = artifacts::evidence::Claim::new(map_claim_kind(claim.kind));
    typed.id = claim.id;
    typed.path = claim.path;
    typed.synopsis = claim.synopsis;
    typed.set_backing(claim.backing.map(|backing| match backing {
        aseam::Backing::Payload(payload) => artifacts::evidence::Backing::Payload(payload),
        aseam::Backing::Path(path) => artifacts::evidence::Backing::Path(path),
    }));
    typed
}

const fn map_claim_kind(kind: aseam::ClaimKind) -> artifacts::evidence::ClaimKind {
    use artifacts::evidence::ClaimKind;
    match kind {
        aseam::ClaimKind::Intent => ClaimKind::Intent,
        aseam::ClaimKind::Requirement => ClaimKind::Requirement,
        aseam::ClaimKind::Criterion => ClaimKind::Criterion,
        aseam::ClaimKind::Decision => ClaimKind::Decision,
        aseam::ClaimKind::Section => ClaimKind::Section,
        aseam::ClaimKind::Diagram => ClaimKind::Diagram,
        aseam::ClaimKind::Contract => ClaimKind::Contract,
        aseam::ClaimKind::Example => ClaimKind::Example,
        aseam::ClaimKind::Excerpt => ClaimKind::Excerpt,
        aseam::ClaimKind::Type => ClaimKind::Type,
        aseam::ClaimKind::Call => ClaimKind::Call,
        aseam::ClaimKind::Region => ClaimKind::Region,
        aseam::ClaimKind::Container => ClaimKind::Container,
        aseam::ClaimKind::Leaf => ClaimKind::Leaf,
    }
}

fn map_input(input: Input) -> aseam::Input {
    match input {
        Input::Proposal(body) => aseam::Input::Proposal(body),
        Input::Design(body) => aseam::Input::Design(body),
        Input::Tasks(body) => aseam::Input::Tasks(body),
        Input::Spec(body) => aseam::Input::Spec(body),
        Input::Other(body) => aseam::Input::Other(body),
    }
}

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

// The folded `detail` prose serves as title, impact, and remediation.
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
    diagnostic.fingerprint = diagnostics::fingerprint(&diagnostic);
    diagnostic
}

const fn map_severity(severity: aseam::Severity) -> Severity {
    match severity {
        aseam::Severity::Critical => Severity::Critical,
        aseam::Severity::Important => Severity::Important,
        aseam::Severity::Suggestion => Severity::Suggestion,
        aseam::Severity::Optional => Severity::Optional,
    }
}

const fn map_platform(platform: aseam::Platform) -> project::platform::Platform {
    use project::platform::Platform;
    match platform {
        aseam::Platform::Core => Platform::Core,
        aseam::Platform::Ios => Platform::Ios,
        aseam::Platform::Android => Platform::Android,
        aseam::Platform::Web => Platform::Web,
        aseam::Platform::Desktop => Platform::Desktop,
    }
}
