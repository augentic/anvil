//! Default implementation for `emery:vcs`.

use anyhow::bail;
use omnia::Backend;
use project::handler::{ExecutionPaths, GUEST_STAGING_MOUNT};
use project::seam::{
    TreeCredentials, TreeError, TreeLimits, WorktreeError, WorktreeRequest, WorktreeState,
};
use project::snapshot::SnapshotId;
use project::workspace::Store;

use crate::host::{
    Credentials, Error, ExportError, ExportRequest, ExportState, Fetched, ForgeError, ForgeResult,
    Limits, PrState, PullRequest, TreesResult, WasiVcsCtx, WorktreeResult, blocking,
    blocking_forge,
};

/// Staging trees older than this are abandoned (a crashed run never
/// discarded them); each fetch sweeps opportunistically.
const STALE_AFTER: std::time::Duration = std::time::Duration::from_hours(24);

/// Default implementation for `emery:vcs`.
#[derive(Clone, Debug)]
pub struct VcsDefault {
    paths: ExecutionPaths,
}

impl Backend for VcsDefault {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: ExecutionPaths::host(),
        })
    }
}

impl WasiVcsCtx for VcsDefault {
    fn fetch(
        &self, locator: String, credentials: Credentials, limits: Limits,
    ) -> TreesResult<Fetched> {
        let paths = self.paths.clone();
        blocking(move || {
            let staging = paths.locations().staging_root();
            project::vcs::sweep_stale(staging, STALE_AFTER);
            let credentials = match credentials {
                Credentials::None => TreeCredentials::None,
                Credentials::Ambient => TreeCredentials::Ambient,
            };
            let limits = TreeLimits {
                max_bytes: limits.max_bytes,
                max_redirects: limits.max_redirects,
                time_ms: limits.time_ms,
            };
            let fetched =
                project::vcs::fetch(staging, &locator, credentials, &limits).map_err(wire_error)?;
            Ok(Fetched {
                root: format!("{GUEST_STAGING_MOUNT}/{}", fetched.name),
                revision: fetched.revision,
            })
        })
    }

    fn discard(&self, root: String) -> TreesResult<()> {
        let paths = self.paths.clone();
        blocking(move || {
            let name = staged_name(&root).map_err(|err| Error::InvalidRequest(err.to_string()))?;
            project::vcs::discard(paths.locations().staging_root(), &name).map_err(wire_error)
        })
    }

    fn export(&self, req: ExportRequest) -> WorktreeResult<(String, ExportState)> {
        let paths = self.paths.clone();
        Box::pin(async move {
            let cid = SnapshotId::parse(&req.cid).map_err(|err| {
                ExportError::InvalidRequest(format!("`{}` is not a snapshot id: {err}", req.cid))
            })?;
            let request = WorktreeRequest {
                repository: req.repository,
                parent_revision: req.parent_revision,
                branch: req.branch,
                cid,
                plan: req.plan,
                target: req.target,
                allow_in_place: req.allow_in_place,
            };
            let store = Store::new(paths.locations().snapshots_root().to_path_buf());
            let env = project::vcs::worktree::ExportEnv {
                store: &store,
                publication_root: paths.locations().publication_root(),
                product_root: (!paths.is_detached()).then(|| paths.project_root()),
            };
            let (path, state) =
                project::vcs::worktree::export(&env, &request).await.map_err(wire_export_error)?;
            let state = match state {
                WorktreeState::Created => ExportState::Created,
                WorktreeState::Matched => ExportState::Matched,
                WorktreeState::Rematerialized => ExportState::Rematerialized,
            };
            Ok((path.display().to_string(), state))
        })
    }

    fn find(&self, repository: String, branch: String) -> ForgeResult<Vec<PullRequest>> {
        blocking_forge(move || {
            let config = project::vcs::forge::Config::github();
            let rows = project::vcs::forge::find(&config, &repository, &branch)
                .map_err(wire_forge_error)?;
            Ok(rows.into_iter().map(wire_pull_request).collect())
        })
    }
}

/// Carry one typed pull request onto the WIT wire record.
fn wire_pull_request(row: project::seam::PullRequest) -> PullRequest {
    PullRequest {
        url: row.url,
        body: row.body,
        state: match row.state {
            project::seam::PrState::Open => PrState::Open,
            project::seam::PrState::Merged => PrState::Merged,
            project::seam::PrState::Closed => PrState::Closed,
        },
        base: row.base,
        merged_at: row.merged_at,
        merge_commit: row.merge_commit,
    }
}

/// Carry the kernel's typed forge error onto the WIT wire variant.
fn wire_forge_error(err: project::seam::ForgeError) -> ForgeError {
    match err {
        project::seam::ForgeError::InvalidRequest(detail) => ForgeError::InvalidRequest(detail),
        project::seam::ForgeError::Auth(detail) => ForgeError::Auth(detail),
        project::seam::ForgeError::Transport(detail) => ForgeError::Transport(detail),
        project::seam::ForgeError::Internal(detail) => ForgeError::Internal(detail),
    }
}

/// Carry the kernel's typed D11 refusal onto the WIT wire variant.
fn wire_export_error(err: WorktreeError) -> ExportError {
    match err {
        WorktreeError::Dirty => ExportError::Dirty,
        WorktreeError::BranchDiverged => ExportError::BranchDiverged,
        WorktreeError::BranchCheckedOutElsewhere => ExportError::BranchCheckedOutElsewhere,
        WorktreeError::DestinationConflict => ExportError::DestinationConflict,
        WorktreeError::ParentUnavailable => ExportError::ParentUnavailable,
        WorktreeError::CloneFailed(detail) => ExportError::CloneFailed(detail),
        WorktreeError::InvalidRequest(detail) => ExportError::InvalidRequest(detail),
        WorktreeError::Internal(detail) => ExportError::Internal(detail),
    }
}

/// Carry the kernel's typed tree error onto the WIT wire variant.
fn wire_error(err: TreeError) -> Error {
    match err {
        TreeError::InvalidRequest(detail) => Error::InvalidRequest(detail),
        TreeError::Access(detail) => Error::Access(detail),
        TreeError::Limit(detail) => Error::Limit(detail),
        TreeError::Internal(detail) => Error::Internal(detail),
    }
}

/// The staged-tree name inside a guest-reported root — refuses
/// anything not directly beneath the staging mount.
fn staged_name(root: &str) -> anyhow::Result<String> {
    if let Some(name) =
        root.strip_prefix(GUEST_STAGING_MOUNT).and_then(|rest| rest.strip_prefix('/'))
        && !name.is_empty()
        && !name.contains('/')
    {
        return Ok(name.to_string());
    }
    bail!("staged root `{root}` is not beneath the staging mount")
}
