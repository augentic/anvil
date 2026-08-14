//! Capability traits over the `emery:adapter` guest-to-guest contract.
//!
//! [`Source`] and [`Target`] mirror the WIT interfaces; their DTOs omit
//! caller-owned fields such as the orchestrator-added source attribution.

pub mod wire;

use std::future::Future;

use artifacts::evidence::{AuthorityClass, Claim};
use serde::{Deserialize, Serialize};
pub use wire::{
    BuildReport, DeferredRequirement, PhaseOutcome, PhaseReport, PhaseRoot, PhaseSource,
    PhaseWrite, RepairOrigin,
};

use crate::snapshot::{CodePatch, SnapshotId};

/// Typed seam failure, mirroring the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The request itself is malformed; retrying unchanged is pointless.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// A filesystem operation failed on the adapter side.
    #[error("io: {0}")]
    Io(String),
    /// A judgment call or answer-handling step failed on the adapter side.
    #[error("internal: {0}")]
    Internal(String),
}

/// One lead surfaced by a survey.
///
/// The shape is the catalog lead minus the envelope `source` key,
/// which the orchestrator stamps. Doubles as the item shape of the
/// generated `survey` judgment-answer schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Lead {
    /// Stable kebab-case lead identifier, unique only within its
    /// source; identity is the `(source, lead)` pair.
    pub lead: String,
    /// Headline used for cross-source reconciliation.
    pub synopsis: String,
    /// Agent-authored per-lead topic slugs (kebab-case). Empty means
    /// unclassified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Parent lead id within the same source. Absent on a top-level lead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Source-local focus that produced this lead. Absent on an
    /// unfocused import or survey row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

impl Lead {
    /// A top-level lead with no topics, parent, or focus.
    #[must_use]
    pub fn new(lead: impl Into<String>, synopsis: impl Into<String>) -> Self {
        Self {
            lead: lead.into(),
            synopsis: synopsis.into(),
            topics: Vec::new(),
            parent: None,
            focus: None,
        }
    }

    /// Project a catalog row onto the seam record (drops `source`).
    #[must_use]
    pub fn from_catalog(lead: &artifacts::leads::Lead) -> Self {
        Self {
            lead: lead.lead.clone(),
            synopsis: lead.synopsis.clone(),
            topics: lead.topics.clone(),
            parent: lead.parent.clone(),
            focus: lead.focus.clone(),
        }
    }
}

/// Read-only CID view for a location-backed source. No artifacts
/// grant — the change home is not on this record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWorkspace {
    /// Opaque identity of the preparation.
    pub id: String,
    /// Deployment-local path of the read-only view root.
    pub root: String,
}

/// Workspace-or-value payload on a source dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContent {
    /// Read-only CID view of a location-backed source.
    Workspace(SourceWorkspace),
    /// Inline value; no filesystem lend.
    Value(String),
}

/// Typed source-operation input: source key, workspace-or-value, and
/// optional catalog lead (parent focus on survey; terminal on extract).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInput {
    /// Plan source-binding key (`plan.yaml.sources.<key>`).
    pub key: String,
    /// Read-only CID view or inline value.
    pub content: SourceContent,
    /// Parent-lead focus (survey) or terminal lead (extract).
    pub focus: Option<Lead>,
}

impl SourceInput {
    /// Inline-value input with no focus — the unfocused survey / value extract shape.
    #[must_use]
    pub fn value(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Value(value.into()),
            focus: None,
        }
    }
}

/// Survey response distinguishing an unfocused top-level set from
/// focused children under the named parent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SurveyResult {
    /// Top-level leads from an unfocused survey. Empty when focused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leads: Vec<Lead>,
    /// Stable child leads under the focused parent. Empty when unfocused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Lead>,
}

/// Evidence returned by an extract, without the caller-owned `lead` key.
///
/// Claims are the typed [`Claim`] mirror of the WIT record; open
/// per-kind fields ride its flattened `extras` map. Doubles as the
/// generated `extract` judgment-answer schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// Document-level authority class of this evidence.
    pub authority: AuthorityClass,
    /// The claims extracted from the source, in answer order.
    pub claims: Vec<Claim>,
}

/// One slice-artifact payload, mirroring the WIT `payload` variant.
///
/// `Path` is the artifact's project-relative location ('/'-separated),
/// resolvable in the adapter guest's `"."` preopen and rendered
/// against the lent workspace's artifact root for spawned agents —
/// never host-absolute. `Body` is the inlined artifact text for
/// non-lent deployments (RFC-55). The cases are exclusive: the engine
/// sends `Path` while every deployment lends a workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    /// Project-relative artifact path ('/'-separated).
    Path(String),
    /// Inlined artifact text when the deployment does not lend a workspace.
    Body(String),
}

/// One slice-artifact input to a build.
///
/// Paths cross the seam typed; adapters render them verbatim and never
/// re-derive the engine's slice layout from prose conventions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md`.
    Proposal(Payload),
    /// The slice's `design.md`.
    Design(Payload),
    /// The slice's `tasks.md`.
    Tasks(Payload),
    /// One behavioural spec (`specs/<domain>/spec.md`).
    Spec(Payload),
    /// Any additional artifact.
    Other(Payload),
}

/// Deterministic per-slice facts the engine forwards to a build,
/// mirroring the WIT `build-context` record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildContext {
    /// Kebab-case adapter names of the slice's bound sources, resolved
    /// from the plan entry's `sources[]` through the plan-level
    /// bindings map. Empty when the slice has no resolvable plan entry.
    pub sources: Vec<String>,
}

/// The attempt-local writable artifact stage, mirroring the WIT
/// `artifact-stage` record (RFC-90 D5).
///
/// An agent-visible mirror rooted at the candidate slice tree. The
/// engine seeds it before `build`, derives its diff after every
/// mutating phase, enforces the target's declared writable-artifact
/// grants, and promotes the diff transactionally on terminal success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactStage {
    /// Opaque identity of the stage preparation.
    pub id: String,
    /// Deployment-local path of the writable stage root.
    pub root: String,
}

/// One prepared private workspace, mirroring the WIT `workspace`
/// record (RFC-87).
///
/// Guests share mount preopens, so no directory handle crosses the
/// seam: `root` is a deployment-local path the receiving side resolves
/// against its own preopens (or opens directly off-wasm), and
/// `artifacts` is the agent-visible read-only artifact root.
/// `artifact_stage` is the writable slice-artifact mirror, present on
/// the build-loop operations and absent on `merge`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Opaque identity of the preparation.
    pub id: String,
    /// Deployment-local path of the writable workspace root.
    pub root: String,
    /// Agent-visible read-only artifact root (the project tree).
    pub artifacts: String,
    /// Writable artifact stage for the active slice; absent on merge
    /// dispatches, whose workspace view is read-only.
    pub artifact_stage: Option<ArtifactStage>,
}

/// Which side of the deterministic core merge a `merge` dispatch runs
/// on, mirroring the WIT `merge-phase` enum. The engine's merge stays
/// deterministic; the target's judgment brackets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum MergePhase {
    /// Before the deterministic commit: a blocking finding aborts the
    /// merge with the slice still `built`.
    Preflight,
    /// After the commit and archive: a blocking finding is a terminal
    /// diagnostic, never a rollback.
    Postflight,
}

/// Plan-time discovery and slice-time extraction for a source adapter.
///
/// Every dispatch carries the caller's source `key` and a prepared
/// [`SourceInput`] — the adapter never recovers a source location from
/// `plan.yaml` or the `"."` preopen.
pub trait Source: Send + Sync {
    /// Lightly survey the source into a lead set.
    fn survey(
        &self, id: String, input: SourceInput,
    ) -> impl Future<Output = Result<SurveyResult, Error>> + Send;

    /// Thoroughly extract evidence from the prepared input for one lead.
    fn extract(
        &self, id: String, input: SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// Synthesis guidance, the build-loop phase operations, and phased
/// merge gates for a target adapter.
///
/// Each build-loop dispatch (`build` / `verify` / `repair` /
/// `review`) performs exactly one operation and returns one typed
/// [`PhaseReport`]; operation order, repair routing, and budgets are
/// engine policy (RFC-90 D1).
pub trait Target: Send + Sync {
    /// Guidance on the expected build artifacts for this target.
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, Error>> + Send;

    /// Generation only: build `slice` inside its prepared private
    /// workspace. Must not verify, repair, or run standards
    /// remediation; `build` alone declares outputs and the UI surface.
    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, context: BuildContext,
        workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One model-assisted check pass over the lent workspace. Receives
    /// only the candidate workspace — every other operation names a
    /// slice.
    fn verify(
        &self, id: String, workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One findings-directed repair pass. `origin` names the engine
    /// gate that supplied `findings` (the deterministic bounded repair
    /// brief); the returned report never selects the next operation.
    fn repair(
        &self, id: String, slice: String, origin: RepairOrigin,
        findings: Vec<diagnostics::Diagnostic>, continuation: Option<Vec<u8>>,
        workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One engineering-standards review pass.
    fn review(
        &self, id: String, slice: String, continuation: Option<Vec<u8>>, workspace: Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// Run one target-specific merge gate (`phase`) around the engine's
    /// deterministic core merge. Dispatched twice per slice merge —
    /// preflight before the commit, postflight after it — each over the
    /// merge workspace.
    fn merge(
        &self, id: String, slice: String, phase: MergePhase, workspace: Workspace,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;
}

/// The host-owned private-workspace capability (RFC-87).
///
/// Mirrors the WIT `workspaces` interface: immutable snapshots in a
/// content-addressed store, disposable private workspaces, and code
/// patches derived by comparing trees.
///
/// Implemented by the same providers that carry the other seam
/// capabilities: the native provider calls the
/// [`crate::workspace`] kernel in-process; the engine guest maps the
/// host-implemented WIT imports.
pub trait Workspaces: Send + Sync {
    /// Freeze the product tree (the project root minus `.git` and a
    /// nested change home) as an immutable snapshot. The build phase
    /// calls this when it opens a target's *first* wave: later waves
    /// open against the current accepted CID instead. Refinement never
    /// freezes the product tree.
    fn freeze(&self) -> impl Future<Output = Result<SnapshotId, Error>> + Send;

    /// Freeze an arbitrary deployment-local tree (a directory, or a
    /// single file as a one-file tree) as an immutable snapshot — the
    /// source-input preparation leg. Unlike [`Self::freeze`], the
    /// caller names the tree.
    fn snapshot(&self, path: String) -> impl Future<Output = Result<SnapshotId, Error>> + Send;

    /// Materialize `base` into a fresh private workspace.
    /// `writable: false` prepares a read-only source view — same
    /// preparation, discarded without capture.
    fn prepare(
        &self, base: SnapshotId, writable: bool,
    ) -> impl Future<Output = Result<Workspace, Error>> + Send;

    /// Capture the workspace's result tree: store and verify every
    /// object, record the result snapshot, and derive the touched
    /// paths against the recorded base.
    fn capture(&self, id: String) -> impl Future<Output = Result<CodePatch, Error>> + Send;

    /// Discard a workspace. Idempotent; captured snapshots survive by
    /// digest.
    fn discard(&self, id: String) -> impl Future<Output = Result<(), Error>> + Send;

    /// Change-scoped snapshot collection (RFC-88 D2): delete the
    /// store objects reachable from `dead` roots but not from `live`
    /// roots. `plan archive` is the collection point — the archived
    /// plan's pins stop being GC roots. Returns the number of objects
    /// deleted.
    fn sweep(
        &self, dead: Vec<SnapshotId>, live: Vec<SnapshotId>,
    ) -> impl Future<Output = Result<usize, Error>> + Send;
}

/// Host-staged locator ingest: Git/HTTPS I/O plus CID snapshot.
///
/// Path locators are read in-process; Git clone and HTTPS fetch run on
/// the host (native in-process, guest via WIT). Always uses
/// [`crate::binding::Policy::standard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetched {
    /// Exact locator (Git revisions are SHAs).
    pub locator: String,
    /// Tree identity of the staged file-or-tree.
    pub cid: SnapshotId,
    /// Deployment-local path of the staged tree (fingerprint / `project.yaml`).
    pub root: String,
    /// Freshness warning (moved branch); ingest still used the recorded SHA.
    pub warning: Option<String>,
}

/// Host ingest capability for wave binding.
pub trait Ingest: Send + Sync {
    /// Stage `locator`, snapshot it, and return the exact pin plus a
    /// local tree the caller can fingerprint.
    fn fetch(
        &self, locator: String, recorded: Option<SnapshotId>, prior: Option<String>,
    ) -> impl Future<Output = Result<Fetched, Error>> + Send;
}

/// One fetched origin: the deployment-local tree the fetch
/// materialized plus the origin's revision report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginFetched {
    /// Deployment-local root of the fetched tree, beneath the
    /// deployment's workspaces mount. The caller snapshots it and
    /// then discards it via [`Origins::discard_fetched`].
    pub root: String,
    /// The commit the fetch reports, when the origin is Git.
    pub revision: Option<String>,
}

/// The host-owned origin-fetch capability (RFC-104).
///
/// Resolves a remote coverage locator (Git or HTTPS) into a
/// deployment-local tree: the native provider runs host `git` /
/// HTTPS fetch in-process; the engine guest maps the
/// host-implemented `emery:origins` import (the guest has no network
/// or git). Private Git uses ambient host credentials — no secrets
/// surface in the definition home. Only system survey dispatches it.
pub trait Origins: Send + Sync {
    /// Fetch `locator` into a deployment-local tree: a Git origin
    /// clones, any other HTTPS locator downloads as a one-file tree.
    fn fetch(&self, locator: String) -> impl Future<Output = Result<OriginFetched, Error>> + Send;

    /// Discard a fetched tree by its deployment-local root.
    /// Best-effort and idempotent.
    fn discard_fetched(&self, root: String) -> impl Future<Output = Result<(), Error>> + Send;
}

/// The borrowed capability bundle one orchestration run dispatches
/// across: model judgment, source-axis seam, target-axis seam, and
/// adapter resolver.
///
/// The four capabilities stay independent type parameters so tests
/// bind independent mocks per seam; [`Capabilities::provider`] bundles
/// one provider that satisfies all four.
#[derive(Debug)]
pub struct Capabilities<'a, P, S, T, R> {
    /// Judgment-leg model dispatch.
    pub model: &'a P,
    /// Source-axis seam (survey / extract).
    pub sources: &'a S,
    /// Target-axis seam (guidance / build / merge).
    pub targets: &'a T,
    /// Adapter resolver.
    pub resolver: &'a R,
}

impl<'a, Provider> Capabilities<'a, Provider, Provider, Provider, Provider> {
    /// Bundle one provider that carries every capability — the
    /// handler-side constructor over `context.provider`.
    pub const fn provider(provider: &'a Provider) -> Self {
        Self {
            model: provider,
            sources: provider,
            targets: provider,
            resolver: provider,
        }
    }
}

impl<'a, P, S, T, R> Capabilities<'a, P, S, T, R> {
    /// Drop the target seam for phases that never dispatch it (plan
    /// authoring surveys and reconciles but builds nothing).
    #[must_use]
    pub const fn sans_targets(self) -> Capabilities<'a, P, S, (), R> {
        Capabilities {
            model: self.model,
            sources: self.sources,
            targets: &(),
            resolver: self.resolver,
        }
    }
}

// Manual `Copy`/`Clone`: the bundle is four shared borrows, copyable
// regardless of whether the capability types themselves are.
impl<P, S, T, R> Clone for Capabilities<'_, P, S, T, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, S, T, R> Copy for Capabilities<'_, P, S, T, R> {}

/// Map a seam dispatch failure onto the wire contract.
///
/// `operation` is the seam method (`survey`, `extract`, `guidance`,
/// `build`, `merge`); `id` is the routed adapter id (e.g.
/// `source:typescript`).
#[must_use]
pub fn seam_failure(operation: &'static str, id: &str, err: &Error) -> error::Error {
    error::Error::Diag {
        code: "seam-dispatch-failed",
        detail: format!("seam `{operation}` dispatch to `{id}` failed: {err}"),
    }
}

/// Bind one plan source row into a seam [`SourceInput`].
///
/// Location-backed rows prepare a read-only CID view and return its
/// workspace id for the caller to discard. Inline values carry no view.
///
/// # Errors
///
/// `source-cid-missing` when a locator row has no recorded CID;
/// `source-view-prepare-failed` when the read-only prepare fails.
pub async fn bind_source(
    workspaces: &impl Workspaces, key: &str, binding: &crate::plan::SourceBinding,
    focus: Option<Lead>,
) -> Result<(SourceInput, Option<String>), error::Error> {
    if let Some(value) = binding.value.as_ref().filter(|value| !value.is_empty()) {
        return Ok((
            SourceInput {
                key: key.to_string(),
                content: SourceContent::Value(value.clone()),
                focus,
            },
            None,
        ));
    }
    let cid = binding.cid.as_ref().ok_or_else(|| error::Error::Diag {
        code: "source-cid-missing",
        detail: format!(
            "source `{key}` is location-backed but has no recorded cid; re-run plan author \
             to bind it"
        ),
    })?;
    let prepared =
        workspaces.prepare(cid.clone(), false).await.map_err(|err| error::Error::Diag {
            code: "source-view-prepare-failed",
            detail: format!("preparing a read-only view of source `{key}` failed: {err}"),
        })?;
    Ok((
        SourceInput {
            key: key.to_string(),
            content: SourceContent::Workspace(SourceWorkspace {
                id: prepared.id.clone(),
                root: prepared.root,
            }),
            focus,
        },
        Some(prepared.id),
    ))
}

/// Discard a source view, warning on failure so a dispatch error stays
/// the primary result.
pub async fn discard_source_view(workspaces: &impl Workspaces, id: Option<String>) {
    if let Some(id) = id
        && let Err(err) = workspaces.discard(id.clone()).await
    {
        tracing::warn!(workspace = %id, "source view discard failed: {err}");
    }
}

/// The exact routed adapter id for a source dispatch
/// (`source:<name>[@<version>]`) — versioned for a package-resolved
/// identity, unversioned for a cache-backed one.
#[must_use]
pub fn source_id(adapter: &crate::adapter::SourceAdapter) -> String {
    crate::adapter::RoutedId::new(
        crate::adapter::Axis::Source,
        adapter.name.clone(),
        adapter.version.clone(),
    )
    .to_string()
}

/// The exact routed adapter id for a target dispatch
/// (`target:<name>[@<version>]`) — versioned for a package-resolved
/// identity, unversioned for a cache-backed one.
#[must_use]
pub fn target_id(adapter: &crate::adapter::TargetAdapter) -> String {
    crate::adapter::RoutedId::new(
        crate::adapter::Axis::Target,
        adapter.name.clone(),
        adapter.version.clone(),
    )
    .to_string()
}
