//! `specify source resolve` / `specify target resolve` — resolve an
//! adapter component by identity and emit the wire-stable envelope.
//!
//! Project-context-free: the resolvers take the project directory from
//! the input (defaulting to the provider anchor), so the verbs never
//! load [`crate::verb::Ctx`].

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};
use super::{Axis, SourceAdapter, TargetAdapter};
use crate::init::adapter_ref_from_value;
use crate::verb::{Anchor, Out, Render};

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
#[derive(Debug)]
pub struct SourceResolve {
    input: ResolveInput,
}

impl<P: Anchor> Handler<P> for SourceResolve {
    type Error = crate::verb::Error;
    type Input = ResolveInput;
    type Output = Out<ResolveBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let project_dir = project_dir(&self.input, ctx.provider);
        let resolved =
            SourceAdapter::resolve(&adapter_ref_from_value(&self.input.value), &project_dir)?;
        Ok(Reply::ok(Out(ResolveBody {
            axis: Axis::Source.dir_segment(),
            name: resolved.manifest.name.clone(),
            version: resolved.manifest.version.to_string(),
            resolved_path: resolved.location.path().display().to_string(),
            location: resolved.location.label(),
            operations: resolved.manifest.operations().map(ToString::to_string).collect(),
        })))
    }
}

/// `specify target resolve <value>`.
#[derive(Debug)]
pub struct TargetResolve {
    input: ResolveInput,
}

impl<P: Anchor> Handler<P> for TargetResolve {
    type Error = crate::verb::Error;
    type Input = ResolveInput;
    type Output = Out<ResolveBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let project_dir = project_dir(&self.input, ctx.provider);
        let resolved =
            TargetAdapter::resolve(&adapter_ref_from_value(&self.input.value), &project_dir)?;
        Ok(Reply::ok(Out(ResolveBody {
            axis: Axis::Target.dir_segment(),
            name: resolved.manifest.name.clone(),
            version: resolved.manifest.version.to_string(),
            resolved_path: resolved.location.path().display().to_string(),
            location: resolved.location.label(),
            operations: resolved.manifest.operations().map(ToString::to_string).collect(),
        })))
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
    /// Adapter name from the component's `describe` metadata.
    pub name: String,
    /// Adapter version.
    pub version: String,
    /// Display path of the resolved `.wasm` component.
    pub resolved_path: String,
    /// Which probe answered (store / cache / dev build).
    pub location: &'static str,
    /// The axis's closed operation set.
    pub operations: Vec<String>,
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
