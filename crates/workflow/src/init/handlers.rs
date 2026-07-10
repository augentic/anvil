//! `specify init --scaffold-only` — the project-scoped scaffold leg.
//!
//! The scaffold writes project-scoped state only — `.specify/`,
//! `project.yaml`, `registry.yaml` (workspace mode), `.gitignore`
//! lines, and the per-project derived component-mirror cache tenant.
//! Everything the full `init` adds around it (hydration, deployment-
//! manifest generation, `AGENTS.md` context generation, the workspace
//! sync chain) awaits its in-guest implementation.
//!
//! Unlike the project-scoped verbs, the scaffold runs *before* a
//! project exists, so it anchors at the provider's raw
//! [`Anchor::project_root`] instead of loading [`crate::handler::Ctx`].

use std::io::Write;
use std::path::Path;

use error::Error;
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use super::{InitOptions, InitResult, init};
use crate::handler::{Anchor, Out, Render};
use crate::platform::parse_platforms_csv;

/// Wire input for the scaffold leg — the `init` argument surface minus
/// nothing: the same flags parse, only the provisioning legs are
/// absent.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScaffoldInput {
    /// Adapter identifier recorded on `project.yaml.adapter`.
    #[serde(default)]
    pub adapter: Option<String>,
    /// Project name override.
    #[serde(default)]
    pub name: Option<String>,
    /// Project description.
    #[serde(default)]
    pub description: Option<String>,
    /// Scaffold a registry-only workspace.
    #[serde(default)]
    pub workspace: bool,
    /// Raw `--platforms` CSV.
    #[serde(default)]
    pub platforms: Option<String>,
}

/// `specify init --scaffold-only` against the provider's anchor root
/// (`"."` on both sides: the guest's mount preopen, the native
/// process CWD).
#[derive(Debug)]
pub struct Scaffold {
    input: ScaffoldInput,
}

impl<P: Anchor> Handler<P> for Scaffold {
    type Error = crate::handler::Error;
    type Input = ScaffoldInput;
    type Output = Out<ScaffoldBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let ScaffoldInput {
            adapter,
            name,
            description,
            workspace,
            platforms,
        } = self.input;
        let parsed_platforms =
            platforms.as_deref().map(parse_platforms_csv).transpose().map_err(|e| {
                Error::Argument {
                    flag: "--platforms",
                    detail: e,
                }
            })?;

        let opts = InitOptions {
            project_dir: ctx.provider.project_root(),
            adapter: adapter.as_deref(),
            name: name.as_deref(),
            description: description.as_deref(),
            workspace,
            platforms: parsed_platforms.as_deref(),
            upgrade: false,
        };
        let result = init(opts, jiff::Timestamp::now())?;
        Ok(Reply::ok(Out(ScaffoldBody::from(&result))))
    }
}

/// Display a path as the canonical absolute form when it exists; fall
/// back to the lossy display when it does not.
fn canonical(p: &Path) -> String {
    std::fs::canonicalize(p).map_or_else(|_| p.display().to_string(), |c| c.display().to_string())
}

/// Success envelope for the scaffold leg.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScaffoldBody {
    /// Display path of the written `project.yaml`.
    pub config_path: String,
    /// Resolved adapter name (or `"workspace"` for workspace init).
    pub adapter_name: String,
    /// Whether the project component cache tenant already existed.
    pub cache_present: bool,
    /// Directories the scaffold created.
    pub directories_created: Vec<String>,
    /// Rule keys scaffolded into the project.
    pub scaffolded_rule_keys: Vec<String>,
    /// The `specify` version pinned on `project.yaml`.
    pub specify_version: String,
    /// Whether a wasm-pkg config was written.
    pub wasm_pkg_config_written: bool,
}

impl From<&InitResult> for ScaffoldBody {
    fn from(result: &InitResult) -> Self {
        Self {
            config_path: canonical(&result.config_path),
            adapter_name: result.adapter_name.clone(),
            cache_present: result.cache_present,
            directories_created: result.directories_created.iter().map(|p| canonical(p)).collect(),
            scaffolded_rule_keys: result.scaffolded_rule_keys.clone(),
            specify_version: result.specify_version.clone(),
            wasm_pkg_config_written: result.wasm_pkg_config_written,
        }
    }
}

impl Render for ScaffoldBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.adapter_name == "workspace" {
            writeln!(w, "Scaffolded .specify/ as a registry-only workspace")?;
        } else {
            writeln!(w, "Scaffolded .specify/")?;
        }
        writeln!(w, "  adapter: {}", self.adapter_name)?;
        writeln!(w, "  config: {}", self.config_path)?;
        writeln!(w, "  specify: {}", self.specify_version)?;
        Ok(())
    }
}
