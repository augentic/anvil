//! The `emery adapter` and axis-resolve operations.
//!
//! `adapter add` seeds the project component cache, `adapter upgrade`
//! refreshes a bare name (or, with `--all`, every bare project
//! binding) to the newest published version, and `source resolve` /
//! `target resolve` resolve adapter components by identity.
//!
//! Project-context-light: every verb takes the project directory from
//! the input (defaulting to the provider anchor) and none loads
//! `crate::handler::Ctx`. Only `adapter upgrade --all` requires
//! `.emery/project.yaml` (it enumerates the recorded bindings);
//! `adapter add` still runs before `init`.

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

/// Wire input for `adapter upgrade`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpgradeInput {
    /// Bare adapter name to upgrade; `None` when `all` is set.
    #[serde(default)]
    pub name: Option<String>,
    /// Upgrade every bare adapter binding recorded by the project
    /// (`project.yaml` target plus `plan.yaml` sources).
    #[serde(default)]
    pub all: bool,
    /// Project directory the `--all` collection anchors at; `None`
    /// anchors at the provider root.
    #[serde(default)]
    pub project_dir: Option<PathBuf>,
}

/// `emery adapter upgrade <name>` / `--all` — refresh bare-named
/// adapters to the newest published version and report what each name
/// now resolves to.
///
/// The network leg is deployment-owned: the launcher derives the
/// refresh set from argv and runs the registry check when this
/// handler's resolve dispatches the routed id. The handler itself is
/// axis-neutral over the unique-across-axes name space: it tries the
/// target axis first, then the source axis.
#[derive(Clone, Copy, Debug)]
pub struct AdapterUpgrade;

impl<P: Anchor + Resolver> Operation<P> for AdapterUpgrade {
    type Error = crate::handler::Error;
    type Input = UpgradeInput;
    type Output = UpgradeBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let paths = input.project_dir.clone().map_or_else(
            || context.provider.paths().clone(),
            |dir| context.provider.paths().with_root(dir),
        );
        let names: Vec<String> = match (input.name, input.all) {
            (Some(value), false) => {
                let AdapterSelector::Bare { name } = AdapterSelector::parse(&value)? else {
                    return Err(error::Error::Diag {
                        code: "adapter-upgrade-not-bare",
                        detail: format!(
                            "`adapter upgrade` takes a bare adapter name (`omnia`), not \
                             `{value}`: pinned versions are immutable and local components \
                             refresh through `emery adapter add`"
                        ),
                    }
                    .into());
                };
                vec![name]
            }
            (None, true) => super::upgrade::targets(paths.project_root())?.into_iter().collect(),
            _ => {
                return Err(error::Error::Diag {
                    code: "adapter-upgrade-arguments",
                    detail: "pass exactly one of a bare adapter name or --all".to_string(),
                }
                .into());
            }
        };
        let mut adapters = Vec::with_capacity(names.len());
        for name in names {
            let selector = AdapterSelector::Bare { name };
            // Adapter names are unique across axes, so the first axis
            // that resolves is the adapter's axis; the wrong axis
            // fails at the dispatch/metadata gate.
            let body = match context.provider.resolve_target(&selector, &paths) {
                Ok(resolved) => ResolveBody::from(resolved),
                Err(_) => ResolveBody::from(context.provider.resolve_source(&selector, &paths)?),
            };
            adapters.push(body);
        }
        Ok(UpgradeBody { adapters })
    }
}

/// Success envelope for `adapter upgrade`.
///
/// Carries the resolved identity per upgraded adapter in the shared
/// [`ResolveBody`] wire shape (one element for the named form), with
/// an upgrade-specific text rendering that states what each refresh
/// settled on.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpgradeBody {
    /// Resolved identities, in name order.
    pub adapters: Vec<ResolveBody>,
}

impl Render for UpgradeBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.adapters.is_empty() {
            writeln!(w, "no bare adapter bindings to upgrade")?;
            return Ok(());
        }
        for body in &self.adapters {
            render_upgraded(w, body)?;
        }
        Ok(())
    }
}

/// One adapter's upgrade outcome line block.
fn render_upgraded(w: &mut dyn Write, body: &ResolveBody) -> std::io::Result<()> {
    // A seeded cache entry always wins over published versions, so
    // an upgrade settles back on the seed — say so instead of
    // implying a refresh happened.
    if body.location == "cache" {
        writeln!(
            w,
            "`{}` still resolves the seeded cache entry — a cache seed shadows published \
             versions",
            body.name
        )?;
        writeln!(w, "  entry: {}", body.resolved_path)?;
        writeln!(
            w,
            "  hint: re-seed with `emery adapter add <path.wasm>` or delete the cache entry to \
             track published releases"
        )?;
        return Ok(());
    }
    match &body.version {
        Some(version) => writeln!(
            w,
            "`{}` resolves {} ({}) — the newest published version",
            body.name, version, body.location
        )?,
        None => writeln!(w, "`{}` resolves via {}", body.name, body.location)?,
    }
    writeln!(w, "  path: {}", body.resolved_path)?;
    Ok(())
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
