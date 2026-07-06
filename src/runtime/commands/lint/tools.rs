//! Declared WASI-tool inventory for `specify lint project`.
//!
//! The `kind: tool` lint evaluator resolves tool names against the
//! project-scope `tools[]` declarations in `project.yaml`; the
//! inventory assembled here is the closed declaration surface the
//! [`super::project`] runner consults before dispatching a tool
//! through `specify_registry::resolver::resolve` +
//! `specify_registry::host::WasiRunner`.

use specify_error::{Error, Result};
use specify_registry::load;
use specify_registry::manifest::{Extension, ExtensionManifest, ExtensionScope};

use crate::runtime::context::Ctx;

/// One declared tool paired with the scope it was declared in.
#[derive(Debug, Clone)]
pub struct ScopedTool {
    scope: ExtensionScope,
    tool: Extension,
}

impl ScopedTool {
    /// Borrow the resolved scope the tool was declared in.
    pub const fn scope(&self) -> &ExtensionScope {
        &self.scope
    }

    /// Borrow the tool record (name, version, source, permissions).
    pub const fn tool(&self) -> &Extension {
        &self.tool
    }
}

/// The declared-tool inventory for one project.
#[derive(Debug)]
pub struct Inventory {
    tools: Vec<ScopedTool>,
}

impl Inventory {
    /// Look up the declared tool with the given `name`, or return
    /// `None` if no declaration matches.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&ScopedTool> {
        self.tools.iter().find(|scoped| scoped.tool.name == name)
    }
}

/// Assemble the declared-tool inventory from the project.yaml
/// `tools[]` declarations, validating the manifest structure first.
pub fn build_inventory(ctx: &Ctx) -> Result<Inventory> {
    let project_scope = ExtensionScope::Project {
        project_name: ctx.config.name.clone(),
    };
    validate_manifest_tools(&ctx.config.tools, &project_scope)?;
    let tools = load::project_tools(ctx.config.name.clone(), ctx.config.tools.clone())
        .into_iter()
        .map(|(scope, tool)| ScopedTool { scope, tool })
        .collect();
    Ok(Inventory { tools })
}

fn validate_manifest_tools(tools: &[Extension], scope: &ExtensionScope) -> Result<()> {
    let manifest = ExtensionManifest {
        tools: tools.to_vec(),
    };
    // `validate_structure` returns one deterministic `violation`
    // diagnostic per failing rule (passing rules emit nothing), so an
    // empty vector means the manifest is structurally valid. Collapse
    // any failures into a single payload-free `Error::Validation` keyed
    // on the first rule id; per-row detail is joined into the message.
    let diagnostics = manifest.validate_structure(scope);
    let Some(first) = diagnostics.first() else {
        return Ok(());
    };
    let code = first.rule_id.clone().unwrap_or_else(|| "tool-manifest-invalid".to_string());
    let detail = diagnostics.iter().map(|d| d.impact.as_str()).collect::<Vec<_>>().join("; ");
    Err(Error::validation_failed(code, "declared extensions must satisfy structural rules", detail))
}
