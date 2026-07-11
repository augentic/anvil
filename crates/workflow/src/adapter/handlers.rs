//! `specify source resolve` / `specify target resolve` — resolve an
//! adapter component by identity and emit the wire-stable envelope.
//!
//! Project-context-free: the resolvers take the project directory from
//! the input (defaulting to the provider anchor), so the verbs never
//! load `crate::handler::Ctx`.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::{Axis, Origin, ResolvedSource, ResolvedTarget, Resolver};
use crate::handler::{Anchor, Render};
use crate::init::adapter_ref_from_value;

/// Wire input for `source resolve` / `target resolve`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolveInput {
    /// Adapter identifier — kebab name or `name@version`.
    pub value: String,
    /// Project directory containing `.specify/`; `None` anchors at the
    /// provider root.
    #[serde(default)]
    pub project_dir: Option<PathBuf>,
}

/// `specify source resolve <name>`.
#[derive(Clone, Copy, Debug)]
pub struct SourceResolve;

impl<P: Anchor + Resolver> Operation<P> for SourceResolve {
    type Error = crate::handler::Error;
    type Input = ResolveInput;
    type Output = ResolveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let project_dir = project_dir(&input, context.provider);
        let resolved =
            context.provider.resolve_source(&adapter_ref_from_value(&input.value), &project_dir)?;
        Ok(ResolveBody::from(resolved))
    }
}

/// `specify target resolve <value>`.
#[derive(Clone, Copy, Debug)]
pub struct TargetResolve;

impl<P: Anchor + Resolver> Operation<P> for TargetResolve {
    type Error = crate::handler::Error;
    type Input = ResolveInput;
    type Output = ResolveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let project_dir = project_dir(&input, context.provider);
        let resolved =
            context.provider.resolve_target(&adapter_ref_from_value(&input.value), &project_dir)?;
        Ok(ResolveBody::from(resolved))
    }
}

fn project_dir(input: &ResolveInput, provider: &impl Anchor) -> PathBuf {
    input.project_dir.clone().unwrap_or_else(|| provider.project_root().to_path_buf())
}

/// Wire-stable resolve envelope shared by both axes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolveBody {
    /// Axis directory segment (`sources` / `targets`).
    pub axis: &'static str,
    /// Adapter name from the resolved identity.
    pub name: String,
    /// Adapter version.
    pub version: String,
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
        axis: Axis, name: String, version: &semver::Version, origin: Origin,
        operations: Vec<String>,
    ) -> Self {
        Self {
            axis: axis.dir_segment(),
            name,
            version: version.to_string(),
            resolved_path: origin.reference,
            location: origin.label,
            operations,
        }
    }
}

impl From<ResolvedSource> for ResolveBody {
    fn from(resolved: ResolvedSource) -> Self {
        let operations = resolved.manifest.operations().map(ToString::to_string).collect();
        Self::assemble(
            Axis::Source,
            resolved.manifest.name,
            &resolved.manifest.version,
            resolved.origin,
            operations,
        )
    }
}

impl From<ResolvedTarget> for ResolveBody {
    fn from(resolved: ResolvedTarget) -> Self {
        let operations = resolved.manifest.operations().map(ToString::to_string).collect();
        Self::assemble(
            Axis::Target,
            resolved.manifest.name,
            &resolved.manifest.version,
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
        writeln!(w, "  version: {}", self.version)?;
        writeln!(w, "  location: {}", self.location)?;
        writeln!(w, "  operations: {}", self.operations.join(", "))?;
        Ok(())
    }
}
