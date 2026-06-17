//! Shared fixtures, seeds, and re-exports for the `workflow`
//! integration suite.
//!
//! The suite is split across themed submodules grouped by `plan`
//! command family (`validate`, `next`, `mutate`, `source_binding`,
//! `transition`, `create`, `archive`, `propose`, `authority`). Every
//! submodule pulls its shared surface in with `use crate::support::*;`,
//! so the common imports, helpers, and plan seeds live here once.

pub use std::fs;
pub use std::path::{Path, PathBuf};
pub use std::process::Command as ProcessCommand;

pub use serde_json::Value;
pub use specify_workflow::change::{Plan, Status};
pub use tempfile::{TempDir, tempdir};

pub use crate::common::{
    Project, assert_golden_at, copy_dir, expected_cache_dir, init_workspace, omnia_schema_dir,
    parse_stderr, parse_stdout, repo_root, specify_cmd,
};

pub fn plan_fixtures() -> PathBuf {
    repo_root().join("tests/fixtures/plan")
}

pub fn assert_golden(name: &str, actual: Value) {
    assert_golden_at(&plan_fixtures(), name, actual);
}

// -- setup helpers -----------------------------------------------------

/// Load and parse the project's `plan.yaml` into the in-memory model.
/// Used by setup helpers (and tests) that must assert a write actually
/// landed rather than trusting a bare `.assert().success()`.
pub fn load_plan(project: &Project) -> Plan {
    Plan::load(&project.plan_path()).unwrap_or_else(|err| panic!("load plan.yaml: {err}"))
}

/// Run `specify plan add <name>` as a setup step, asserting BOTH that
/// it exits 0 AND that the entry actually landed in `plan.yaml` as a
/// `pending` row. Most call sites previously asserted only `.success()`,
/// so a silent regression in the plan writer would have slipped past
/// the setup and surfaced as a confusing failure in the assertion under
/// test.
pub fn add_pending_entry(project: &Project, name: &str) {
    add_entry_with(project, name, &[]);
}

/// [`add_pending_entry`] with extra `plan add` flags (e.g. `--sources
/// <key>=<lead>`). Asserts the entry is present and `pending` after the
/// write so the binding-shaping tests start from a verified state.
pub fn add_entry_with(project: &Project, name: &str, extra: &[&str]) {
    let mut args = vec!["plan", "add", name];
    args.extend_from_slice(extra);
    specify_cmd().current_dir(project.root()).args(&args).assert().success();

    let plan = load_plan(project);
    let entry = plan.entries.iter().find(|e| e.name == name).unwrap_or_else(|| {
        panic!("`plan add {name}` did not append an entry; entries: {:?}", plan.entries)
    });
    assert_eq!(
        entry.status,
        Status::Pending,
        "`plan add {name}` must land a pending entry, got {:?}",
        entry.status
    );
}

// -- test seeds --------------------------------------------------------

pub const CLEAN_PLAN: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
    depends-on: [a]
";

pub const DUPLICATE_NAME_PLAN: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
  - name: foo
    project: default
    status: pending
";

pub const A_DONE_B_PENDING: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: pending
";

pub const A_IN_PROGRESS: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: in-progress
";

/// One pending entry. Shared by the `mutate` (amend-on-missing) and
/// `transition` submodules.
pub const SINGLE_PENDING: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
";

pub const ALL_DONE: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: done
";
