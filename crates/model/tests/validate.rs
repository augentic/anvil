//! Filesystem tests for `specify_model::validate::validate_slice` — the
//! contracts-brief (`contracts.*`) and cross-brief (`cross.*`) rule paths.
//!
//! Each test seeds a throw-away slice dir under `tempfile::TempDir`, runs the
//! public `validate_slice` runner, and asserts on the rule ids it reports.
//! All `contracts.*` / `cross.*` rules are structural, so every diagnostic in
//! those namespaces is a violation; other briefs' findings are ignored.

use std::fs;
use std::path::{Path, PathBuf};

use specify_model::validate::validate_slice;
use tempfile::TempDir;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, body).expect("write fixture");
}

struct Slice {
    _tmp: TempDir,
    dir: PathBuf,
}

impl Slice {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let dir = tmp.path().join("slice");
        fs::create_dir_all(&dir).expect("slice dir");
        Self { _tmp: tmp, dir }
    }

    fn file(&self, rel: &str, body: &str) {
        write(&self.dir.join(rel), body);
    }

    fn rule_ids(&self) -> Vec<String> {
        validate_slice(&self.dir)
            .expect("validate_slice ok")
            .into_iter()
            .filter_map(|d| d.rule_id)
            .collect()
    }

    fn fired(&self, rule_id: &str) -> bool {
        self.rule_ids().iter().any(|id| id == rule_id)
    }
}

const VALID_SCHEMA: &str =
    "$id: https://example.com/user.yaml\ntitle: User\ndescription: A user schema\ntype: object\n";

#[test]
fn contracts_clean_overlay_passes() {
    let s = Slice::new();
    s.file("contracts/schemas/user.yaml", VALID_SCHEMA);

    assert!(!s.fired("contracts.schemas-dir-has-files"));
    assert!(!s.fired("contracts.refs-resolve"));
    assert!(!s.fired("contracts.schema-metadata"));
}

#[test]
fn contracts_missing_schemas_dir_fails() {
    let s = Slice::new();
    // A yaml under http/ triggers the contracts brief; schemas/ is absent.
    s.file("contracts/http/api.yaml", "openapi: 3.1.0\n");

    assert!(s.fired("contracts.schemas-dir-has-files"));
}

#[test]
fn contracts_schemas_dir_no_yaml_fails() {
    let s = Slice::new();
    s.file("contracts/schemas/notes.txt", "not a schema\n");
    s.file("contracts/http/api.yaml", "openapi: 3.1.0\n");

    assert!(s.fired("contracts.schemas-dir-has-files"));
}

#[test]
fn contracts_unresolved_ref_fails() {
    let s = Slice::new();
    s.file("contracts/schemas/user.yaml", VALID_SCHEMA);
    s.file("contracts/http/api.yaml", "paths:\n  /u:\n    $ref: \"./missing.yaml\"\n");

    assert!(s.fired("contracts.refs-resolve"));
    // The schema overlay is otherwise well-formed.
    assert!(!s.fired("contracts.schemas-dir-has-files"));
}

#[test]
fn contracts_schema_metadata_fails() {
    let s = Slice::new();
    s.file("contracts/schemas/bad.yaml", "type: object\n");

    assert!(s.fired("contracts.schema-metadata"));
}

#[test]
fn cross_composition_screens_fail() {
    let s = Slice::new();
    s.file(
        "composition.yaml",
        "screens:\n  home:\n    maps_to: \"\"\n  settings:\n    maps_to: 42\n",
    );

    assert!(s.fired("cross.composition-maps-to-consistent"));
}

#[test]
fn cross_composition_delta_fail() {
    let s = Slice::new();
    s.file(
        "composition.yaml",
        "delta:\n  added:\n    home:\n      maps_to: \"\"\n  modified:\n    settings:\n      maps_to: 7\n",
    );

    assert!(s.fired("cross.composition-maps-to-consistent"));
}

#[test]
fn cross_composition_well_formed_passes() {
    let s = Slice::new();
    s.file("composition.yaml", "screens:\n  home:\n    maps_to: \"REQ-001\"\n");

    assert!(!s.fired("cross.composition-maps-to-consistent"));
}

#[test]
fn cross_proposal_domain_without_spec_fails() {
    let s = Slice::new();
    s.file("proposal.md", "## Domains\n\n- auth\n");
    // No specs/auth/spec.md on disk.

    assert!(s.fired("cross.proposal-domains-have-specs"));
}

#[test]
fn cross_proposal_domain_with_spec_passes() {
    let s = Slice::new();
    s.file("proposal.md", "## Domains\n\n- auth\n");
    s.file("specs/auth/spec.md", "# Auth\n");

    assert!(!s.fired("cross.proposal-domains-have-specs"));
}

#[test]
fn cross_design_ref_missing_fails() {
    let s = Slice::new();
    s.file("design.md", "The flow implements REQ-999 end to end.\n");
    s.file("specs/auth/spec.md", "ID: REQ-001\nfooter\n");

    assert!(s.fired("cross.design-references-valid"));
}

#[test]
fn cross_design_ref_present_passes() {
    let s = Slice::new();
    s.file("design.md", "The flow implements REQ-001 end to end.\n");
    s.file("specs/auth/spec.md", "ID: REQ-001\nfooter\n");

    assert!(!s.fired("cross.design-references-valid"));
}
