use std::io::Write;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::Serialize;
use specify_error::Error;
use specify_workflow::adapter::{ResolvedTargetAdapter, TargetAdapter};
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::init::adapter_ref_from_value;

use crate::output::Format;
use crate::runtime::output;

/// Shared context for every subcommand that operates inside an
/// initialised `.specify/` project. Created once at the top of each
/// command handler via [`Ctx::load`].
pub struct Ctx {
    pub(crate) format: Format,
    pub(crate) project_dir: PathBuf,
    pub(crate) config: ProjectConfig,
    /// Plan root override from the global `--plan-dir` flag
    /// (env `SPECIFY_PLAN_DIR`): the initiating workspace root while
    /// phase verbs run inside a workspace slot. `None` anchors plan
    /// artifacts at the project root as usual.
    pub(crate) plan_dir: Option<PathBuf>,
}

impl Ctx {
    /// Resolve the current project root, load `.specify/project.yaml`,
    /// and bundle everything into a `Ctx`.
    ///
    /// Returns `Err(Error)` on failure so callers can propagate with `?`.
    /// The top-level dispatcher (`scoped`) converts `Error` to
    /// the format-aware exit code.
    pub(crate) fn load(format: Format, plan_dir: Option<PathBuf>) -> Result<Self, Error> {
        let current_dir = std::env::current_dir().map_err(Error::Io)?;
        Self::load_at(format, plan_dir, &current_dir)
    }

    /// Variant of [`Self::load`] that walks from `start_dir` instead of
    /// the process CWD. Used by handlers that accept a `--project-dir`
    /// flag (e.g. `specify lint`); the resolved `project_dir` is the
    /// nearest ancestor of `start_dir` containing `.specify/project.yaml`.
    pub(crate) fn load_at(
        format: Format, plan_dir: Option<PathBuf>, start_dir: &Path,
    ) -> Result<Self, Error> {
        let project_dir = ProjectConfig::find_root(start_dir).ok_or(Error::NotInitialized)?;
        let config = ProjectConfig::load(&project_dir)?;
        Ok(Self {
            format,
            project_dir,
            config,
            plan_dir,
        })
    }

    /// Resolve this project's target adapter into a
    /// [`ResolvedTargetAdapter`].
    ///
    /// Workspace projects (`workspace: true`, `adapter:` omitted) do not declare
    /// an adapter, so this returns a `workspace-no-adapter` diagnostic
    /// naming the workspace case rather than a stray adapter-resolution
    /// error lower down the stack.
    pub(crate) fn resolve_target_adapter(&self) -> Result<ResolvedTargetAdapter, Error> {
        let Some(adapter_value) = self.config.adapter.as_deref() else {
            return Err(Error::Diag {
                code: "workspace-no-adapter",
                detail: "this project has no adapter declared (workspaces do not run \
                         per-target operations); only `specify registry` and `specify plan` \
                         verbs are supported on workspaces"
                    .to_string(),
            });
        };
        let adapter_ref = adapter_ref_from_value(adapter_value);
        TargetAdapter::resolve(&adapter_ref, &self.project_dir)
    }

    /// Typed view over `.specify/`-anchored paths. Hand this to
    /// [`specify_workflow::config::with_state`] in handlers that mutate
    /// `plan.yaml` / `registry.yaml`.
    pub(crate) fn layout(&self) -> Layout<'_> {
        Layout::new(&self.project_dir).with_plan_dir(self.plan_dir.as_deref())
    }

    /// Single dispatcher-boundary read of the wall clock. Library crates
    /// never call `Timestamp::now()` (architecture §Time injection); a
    /// handler reads `now` here once and threads it into the workflow
    /// functions that stamp serialised artifacts, so tests pin time by
    /// driving those functions with a fixed `Timestamp`.
    #[expect(
        clippy::unused_self,
        reason = "the clock read is the named seam on Ctx so a future injected test clock has one home; keeping it a method keeps handler call sites uniform (ctx.now())"
    )]
    pub(crate) fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    pub(crate) fn slices_dir(&self) -> PathBuf {
        self.layout().slices_dir()
    }

    pub(crate) fn archive_dir(&self) -> PathBuf {
        self.layout().archive_dir()
    }

    /// Serialise `body` and write it to stdout in this `Ctx`'s
    /// format, using `render_text` for the text-format branch. The
    /// text rendering is a free function colocated with the handler,
    /// so the response shape stays in a single block of code.
    ///
    /// # Errors
    ///
    /// Propagates the underlying serialization or I/O error from
    /// [`output::emit`].
    pub(crate) fn write<T: Serialize>(
        &self, body: &T, render_text: impl FnOnce(&mut dyn Write, &T) -> std::io::Result<()>,
    ) -> Result<(), Error> {
        output::emit(&mut std::io::stdout().lock(), self.format, body, render_text)
    }
}
