//! Shared scaffolding for the per-kind evaluator unit tests.
//!
//! Builds in-memory [`WorkspaceModel`] values and rule/hint fixtures so
//! each `lint/eval/<kind>.rs` module can exercise its kernel directly,
//! with no filesystem walk or indexer pass in the loop.

use serde_json::Value as JsonValue;
use specify_diagnostics::Severity;

use crate::lint::{File, FileKind, ScanProfile, WorkspaceModel, WorkspaceModelVersion};
use crate::rules::{HintKind, Origin, PathRoot, ResolvedRule, RuleHint};

/// An empty framework-profile model; tests push facts onto the public
/// fields directly.
pub fn empty_model() -> WorkspaceModel {
    WorkspaceModel {
        version: WorkspaceModelVersion,
        project_dir: "/tmp".to_string(),
        scan_profile: ScanProfile::Framework,
        artifact_paths: vec![],
        languages: vec![],
        files: vec![],
        frontmatter: vec![],
        markdown_sections: vec![],
        markdown_links: vec![],
        symlinks: vec![],
        skills: vec![],
        adapter_manifests: vec![],
        ignore_directives: vec![],
        briefs: vec![],
        fenced_blocks: vec![],
        scenarios: vec![],
        adapter_dirs: vec![],
    }
}

/// A model whose `files` carry the given paths as text files.
pub fn model_with_paths(paths: &[&str]) -> WorkspaceModel {
    let mut model = empty_model();
    model.files = paths
        .iter()
        .map(|p| File {
            path: (*p).to_string(),
            kind: FileKind::Text,
            language: Some("markdown".to_string()),
            sha256: None,
        })
        .collect();
    model
}

/// A minimal resolved rule carrying the given hints.
pub fn rule_with_hints(hints: Vec<RuleHint>) -> ResolvedRule {
    ResolvedRule {
        rule_id: "TEST-001".to_string(),
        title: "test rule".to_string(),
        severity: Severity::Important,
        trigger: String::new(),
        lint_mode: None,
        applicability: None,
        rule_hints: if hints.is_empty() { None } else { Some(hints) },
        references: None,
        origin: Origin::Core,
        path_root: PathRoot::RulesRoot,
        path: "adapters/shared/rules/core/TEST-001.md".to_string(),
        body: String::new(),
        deprecated: None,
    }
}

/// A minimal resolved rule with no hints.
pub fn rule() -> ResolvedRule {
    rule_with_hints(vec![])
}

/// A bare hint with no config.
pub fn hint(kind: HintKind, value: &str) -> RuleHint {
    hint_with_config(kind, value, None)
}

/// A hint carrying a `config:` payload.
pub fn hint_with_config(kind: HintKind, value: &str, config: Option<JsonValue>) -> RuleHint {
    RuleHint {
        kind,
        value: value.to_string(),
        description: None,
        config,
    }
}

/// Candidate paths as the `PathBuf` slice the umbrella hands each arm.
pub fn candidates(paths: &[&str]) -> Vec<std::path::PathBuf> {
    paths.iter().map(std::path::PathBuf::from).collect()
}
