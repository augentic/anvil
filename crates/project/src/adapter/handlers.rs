//! The `emery adapter` and axis-resolve operations.
//!
//! `adapter add` seeds the project component cache, `adapter update`
//! refreshes a bare name to the newest published version, and
//! `source resolve` / `target resolve` resolve adapter components by
//! identity.
//!
//! Project-context-free: every verb takes the project directory from
//! the input (defaulting to the provider anchor) and none requires
//! `.emery/project.yaml` to exist, so `adapter add` can run before
//! `init` and the verbs never load `crate::handler::Ctx`.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::{
    AdapterSelector, Axis, Origin, ResolvedSource, ResolvedTarget, Resolver, SourceAdapter,
    TargetAdapter, ensure,
};
use crate::handler::{Anchor, ExecutionPaths, Render};

/// Wire input for `adapter add`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddInput {
    /// Local `.wasm` component to seed; relative paths anchor at the
    /// selected project directory.
    pub component: PathBuf,
    /// Project directory the cache is keyed by; `None` anchors at the
    /// provider root.
    #[serde(default)]
    pub project_dir: Option<PathBuf>,
}

/// `emery adapter add <path.wasm>` — mirror a local component into
/// the project component cache so a bare binding (project target or
/// plan source, either axis) resolves it.
///
/// Pre-init and axis-neutral: no `.emery/` is required, the
/// component's exports are not inspected (a wrong-world component
/// fails at the later dispatch/metadata axis gate), and re-seeding the
/// same name replaces the entry and its provenance sidecar.
#[derive(Clone, Copy, Debug)]
pub struct AdapterAdd;

impl<P: Anchor> Operation<P> for AdapterAdd {
    type Error = crate::handler::Error;
    type Input = AddInput;
    type Output = AddBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let paths = input.project_dir.clone().map_or_else(
            || context.provider.paths().clone(),
            |dir| context.provider.paths().with_root(dir),
        );
        let seeded = ensure::seed(&input.component, &paths, jiff::Timestamp::now())?;
        Ok(AddBody::from(seeded))
    }
}

/// Success envelope for `adapter add`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddBody {
    /// Kebab-case adapter name the cache entry resolves as.
    pub name: String,
    /// The seeded project component cache entry.
    pub entry: PathBuf,
    /// The canonical operator-supplied component the entry mirrors.
    pub source: PathBuf,
}

impl From<ensure::Seeded> for AddBody {
    fn from(seeded: ensure::Seeded) -> Self {
        Self {
            name: seeded.name,
            entry: seeded.entry,
            source: seeded.source,
        }
    }
}

impl Render for AddBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Seeded `{}` into the project component cache", self.name)?;
        writeln!(w, "  entry: {}", self.entry.display())?;
        writeln!(w, "  source: {}", self.source.display())?;
        Ok(())
    }
}

/// Wire input for `adapter update`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateInput {
    /// Bare adapter name to update.
    pub name: String,
}

/// `emery adapter update <name>` — refresh a bare-named adapter.
///
/// Updates the name to the newest published version and reports what
/// it resolves to.
///
/// The network leg is deployment-owned: the launcher derives the
/// refresh set from argv and runs the registry check when this
/// handler's resolve dispatches the routed id. The handler itself is
/// axis-neutral over the unique-across-axes name space: it tries the
/// target axis first, then the source axis.
#[derive(Clone, Copy, Debug)]
pub struct AdapterUpdate;

impl<P: Anchor + Resolver> Operation<P> for AdapterUpdate {
    type Error = crate::handler::Error;
    type Input = UpdateInput;
    type Output = ResolveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let selector = AdapterSelector::parse(&input.name)?;
        if !matches!(selector, AdapterSelector::Bare { .. }) {
            return Err(error::Error::Diag {
                code: "adapter-update-not-bare",
                detail: format!(
                    "`adapter update` takes a bare adapter name (`omnia`), not `{}`: pinned \
                     versions are immutable and local components update through `emery adapter \
                     add`",
                    input.name
                ),
            }
            .into());
        }
        let paths = context.provider.paths().clone();
        // Adapter names are unique across axes, so the first axis that
        // resolves is the adapter's axis; the wrong axis fails at the
        // dispatch/metadata gate.
        if let Ok(resolved) = context.provider.resolve_target(&selector, &paths) {
            return Ok(ResolveBody::from(resolved));
        }
        let resolved = context.provider.resolve_source(&selector, &paths)?;
        Ok(ResolveBody::from(resolved))
    }
}

/// Wire input for `source resolve` / `target resolve`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolveInput {
    /// Adapter identifier — kebab name or `name@version`.
    pub value: String,
    /// Project directory containing `.emery/`; `None` anchors at the
    /// provider root.
    #[serde(default)]
    pub project_dir: Option<PathBuf>,
}

/// `emery source resolve <name>`.
#[derive(Clone, Copy, Debug)]
pub struct SourceResolve;

impl<P: Anchor + Resolver> Operation<P> for SourceResolve {
    type Error = crate::handler::Error;
    type Input = ResolveInput;
    type Output = ResolveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let paths = paths(&input, context.provider);
        let resolved =
            context.provider.resolve_source(&AdapterSelector::parse(&input.value)?, &paths)?;
        Ok(ResolveBody::from(resolved))
    }
}

/// `emery target resolve <value>`.
#[derive(Clone, Copy, Debug)]
pub struct TargetResolve;

impl<P: Anchor + Resolver> Operation<P> for TargetResolve {
    type Error = crate::handler::Error;
    type Input = ResolveInput;
    type Output = ResolveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let paths = paths(&input, context.provider);
        let resolved =
            context.provider.resolve_target(&AdapterSelector::parse(&input.value)?, &paths)?;
        Ok(ResolveBody::from(resolved))
    }
}

/// The provider's cache placement, re-anchored at the input's
/// `--project-dir` when supplied.
fn paths(input: &ResolveInput, provider: &impl Anchor) -> ExecutionPaths {
    input
        .project_dir
        .clone()
        .map_or_else(|| provider.paths().clone(), |dir| provider.paths().with_root(dir))
}

/// Wire-stable resolve envelope shared by both axes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolveBody {
    /// Axis directory segment (`sources` / `targets`).
    pub axis: &'static str,
    /// Adapter name from the resolved identity.
    pub name: String,
    /// Adapter version — omitted for an unpinned cache resolve, which
    /// carries no package identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Resolver-defined display reference.
    pub resolved_path: String,
    /// Resolver-defined origin label.
    pub location: String,
    /// The axis's closed operation set.
    pub operations: Vec<String>,
}

impl ResolveBody {
    /// Shared envelope assembly for both axes; only the axis tag, the
    /// manifest identity, and the closed operation set differ.
    fn assemble(
        axis: Axis, name: String, version: Option<&semver::Version>, origin: Origin,
        operations: Vec<String>,
    ) -> Self {
        Self {
            axis: axis.dir_segment(),
            name,
            version: version.map(ToString::to_string),
            resolved_path: origin.reference,
            location: origin.label,
            operations,
        }
    }
}

impl From<ResolvedSource> for ResolveBody {
    fn from(resolved: ResolvedSource) -> Self {
        let operations = SourceAdapter::operations().map(ToString::to_string).collect();
        Self::assemble(
            Axis::Source,
            resolved.manifest.name,
            resolved.manifest.version.as_ref(),
            resolved.origin,
            operations,
        )
    }
}

impl From<ResolvedTarget> for ResolveBody {
    fn from(resolved: ResolvedTarget) -> Self {
        let operations = TargetAdapter::operations().map(ToString::to_string).collect();
        Self::assemble(
            Axis::Target,
            resolved.manifest.name,
            resolved.manifest.version.as_ref(),
            resolved.origin,
            operations,
        )
    }
}

impl Render for ResolveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "{}", self.resolved_path)?;
        writeln!(w, "  axis: {}", self.axis)?;
        writeln!(w, "  name: {}", self.name)?;
        if let Some(version) = &self.version {
            writeln!(w, "  version: {version}")?;
        }
        writeln!(w, "  location: {}", self.location)?;
        writeln!(w, "  operations: {}", self.operations.join(", "))?;
        Ok(())
    }
}
