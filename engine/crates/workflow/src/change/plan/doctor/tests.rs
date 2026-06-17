use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use specify_diagnostics::{Diagnostic, FindingEvidence, Severity};
use tempfile::tempdir;

use super::*;
use crate::change::plan::core::{
    Entry, Plan, SliceSourceBinding, SourceBinding, Status, change, change_with_deps,
    plan_with_changes,
};
use crate::registry::{Registry, RegistryProject};

/// Match a neutral diagnostic on its stable check code (`rule_id`).
fn has_code(d: &Diagnostic, code: &str) -> bool {
    d.rule_id.as_deref() == Some(code)
}

/// Read the structured-evidence payload a health check carries. Panics
/// if the diagnostic does not carry `FindingEvidence::Structured`.
fn data(d: &Diagnostic) -> &serde_json::Value {
    match &d.evidence {
        FindingEvidence::Structured { data, .. } => data,
        other => panic!("expected structured evidence, got {other:?}"),
    }
}

fn plan_with_sources(sources: Vec<(&str, &str)>, changes: Vec<Entry>) -> Plan {
    let mut map = BTreeMap::new();
    for (k, v) in sources {
        map.insert(k.to_string(), SourceBinding::path("typescript", v));
    }
    Plan {
        name: "test".into(),
        lifecycle: crate::change::plan::core::Lifecycle::Pending,
        sources: map,
        entries: changes,
    }
}

// ------- 1. Cycle detection ----------------------------------------

#[test]
fn cycle_two_node() {
    let plan = plan_with_changes(vec![
        change_with_deps("a", Status::Pending, &["b"]),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    let hits: Vec<_> =
        doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, CYCLE)).collect();
    assert_eq!(hits.len(), 1, "expected one cycle, got {hits:#?}");
    assert_eq!(data(&hits[0])["cycle"], serde_json::json!(["a", "b", "a"]));
}

#[test]
fn cycle_three_node() {
    let plan = plan_with_changes(vec![
        change_with_deps("a", Status::Pending, &["c"]),
        change_with_deps("b", Status::Pending, &["a"]),
        change_with_deps("c", Status::Pending, &["b"]),
    ]);
    let hits: Vec<_> =
        doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, CYCLE)).collect();
    assert_eq!(hits.len(), 1, "single SCC, single diagnostic");
    assert_eq!(data(&hits[0])["cycle"], serde_json::json!(["a", "b", "c", "a"]));
}

#[test]
fn cycle_two_disjoint() {
    let plan = plan_with_changes(vec![
        change_with_deps("a", Status::Pending, &["b"]),
        change_with_deps("b", Status::Pending, &["a"]),
        change_with_deps("c", Status::Pending, &["d"]),
        change_with_deps("d", Status::Pending, &["c"]),
    ]);
    let count = doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, CYCLE)).count();
    assert_eq!(count, 2, "expected two distinct cycles");
}

#[test]
fn cycle_self_loop() {
    let plan = plan_with_changes(vec![change_with_deps("a", Status::Pending, &["a"])]);
    let hits: Vec<_> =
        doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, CYCLE)).collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(data(&hits[0])["cycle"], serde_json::json!(["a", "a"]));
}

#[test]
fn no_cycle_quiet() {
    let plan = plan_with_changes(vec![
        change("a", Status::Done),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    let hits: Vec<_> =
        doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, CYCLE)).collect();
    assert!(hits.is_empty(), "no cycle expected, got {hits:#?}");
}

// ------- 2. Orphan source keys -------------------------------------

#[test]
fn orphan_source_zero() {
    let mut e = change("a", Status::Pending);
    e.sources = vec![SliceSourceBinding::bare("monolith")];
    let plan = plan_with_sources(vec![("monolith", "/path")], vec![e]);
    let any_orphan =
        doctor(&plan, None, None, None).into_iter().any(|d| has_code(&d, ORPHAN_SOURCE));
    assert!(!any_orphan);
}

#[test]
fn orphan_source_one() {
    let plan = plan_with_sources(
        vec![("monolith", "/path"), ("orphan", "/elsewhere")],
        vec![{
            let mut e = change("a", Status::Pending);
            e.sources = vec![SliceSourceBinding::bare("monolith")];
            e
        }],
    );
    let hits: Vec<_> = doctor(&plan, None, None, None)
        .into_iter()
        .filter(|d| has_code(d, ORPHAN_SOURCE))
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(data(&hits[0])["key"], "orphan");
    assert_eq!(hits[0].severity, Severity::Suggestion);
}

#[test]
fn orphan_source_multiple_sorted() {
    let plan = plan_with_sources(
        vec![("alpha", "/a"), ("beta", "/b"), ("gamma", "/g"), ("monolith", "/m")],
        vec![{
            let mut e = change("a", Status::Pending);
            e.sources = vec![SliceSourceBinding::bare("monolith")];
            e
        }],
    );
    let hits: Vec<_> = doctor(&plan, None, None, None)
        .into_iter()
        .filter(|d| has_code(d, ORPHAN_SOURCE))
        .collect();
    let keys: Vec<String> =
        hits.iter().map(|d| data(d)["key"].as_str().expect("key string").to_string()).collect();
    assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn orphan_source_mixed_references() {
    let plan = plan_with_sources(
        vec![("monolith", "/m"), ("orders", "/o"), ("ghost", "/g")],
        vec![
            {
                let mut e = change("a", Status::Pending);
                e.sources =
                    vec![SliceSourceBinding::bare("monolith"), SliceSourceBinding::bare("orders")];
                e
            },
            {
                let mut e = change("b", Status::Done);
                e.sources = vec![SliceSourceBinding::bare("orders")];
                e
            },
        ],
    );
    let count =
        doctor(&plan, None, None, None).into_iter().filter(|d| has_code(d, ORPHAN_SOURCE)).count();
    assert_eq!(count, 1, "only `ghost` should orphan");
}

// ------- 4. Stale workspace clones --------------------------------

fn registry_with(projects: Vec<RegistryProject>) -> Registry {
    Registry { version: 1, projects }
}

fn rp(name: &str, url: &str, schema: &str, description: &str) -> RegistryProject {
    RegistryProject {
        name: name.into(),
        url: url.into(),
        adapter: Some(schema.into()),
        description: Some(description.into()),
        contracts: None,
        greenfield_seed: None,
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git").arg("-C").arg(cwd).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git -C {} {} failed\nstdout:\n{}\nstderr:\n{}",
        cwd.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Set up a project root with a `workspace/<name>/` slot
/// wired as a git clone.
fn make_clone_slot(root: &Path, name: &str, origin: Option<&str>) -> std::path::PathBuf {
    let slot = root.join("workspace").join(name);
    std::fs::create_dir_all(&slot).unwrap();
    run_git(&slot, &["init"]);
    if let Some(origin) = origin {
        run_git(&slot, &["remote", "add", "origin", origin]);
    }
    slot
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).unwrap();
}

#[test]
fn stale_clone_reports_missing_origin() {
    let tmp = tempdir().unwrap();
    let _slot = make_clone_slot(tmp.path(), "alpha", None);
    let registry = registry_with(vec![rp(
        "alpha",
        "git@github.com:org/alpha.git",
        "omnia@1.0.0",
        "alpha service",
    )]);
    let plan = plan_with_changes(vec![]);
    let hits: Vec<_> = doctor(&plan, None, Some(&registry), Some(tmp.path()))
        .into_iter()
        .filter(|d| has_code(d, STALE_CLONE))
        .collect();
    assert_eq!(hits.len(), 1, "expected single stale-clone, got {hits:#?}");
    let payload = data(&hits[0]);
    assert_eq!(payload["project"], "alpha");
    assert_eq!(payload["reason"], "slot-mismatch");
    assert_eq!(payload["expected"]["slot-kind"], "git-clone");
    assert_eq!(payload["observed"]["slot-kind"], "git-clone");
    assert!(payload["observed"]["url"].is_null());
    assert!(
        hits[0].impact.contains("has no origin remote"),
        "missing origin should be reported via sync slot rules: {:?}",
        hits[0].impact
    );
}

#[test]
fn stale_clone_signature_changed() {
    let tmp = tempdir().unwrap();
    make_clone_slot(tmp.path(), "alpha", Some("git@github.com:old/alpha.git"));
    let registry = registry_with(vec![rp(
        "alpha",
        "git@github.com:org/alpha.git",
        "omnia@1.0.0",
        "alpha service",
    )]);
    let plan = plan_with_changes(vec![]);
    let hits: Vec<_> = doctor(&plan, None, Some(&registry), Some(tmp.path()))
        .into_iter()
        .filter(|d| has_code(d, STALE_CLONE))
        .collect();
    assert_eq!(hits.len(), 1);
    let payload = data(&hits[0]);
    assert_eq!(payload["reason"], "signature-changed");
    assert_eq!(payload["expected"]["url"], "git@github.com:org/alpha.git");
    assert_eq!(payload["observed"]["url"], "git@github.com:old/alpha.git");
}

#[test]
fn stale_clone_signature_current() {
    let tmp = tempdir().unwrap();
    make_clone_slot(tmp.path(), "alpha", Some("git@github.com:org/alpha.git"));
    let registry = registry_with(vec![rp(
        "alpha",
        "git@github.com:org/alpha.git",
        "omnia@1.0.0",
        "alpha service",
    )]);
    let plan = plan_with_changes(vec![]);
    let hits: Vec<_> = doctor(&plan, None, Some(&registry), Some(tmp.path()))
        .into_iter()
        .filter(|d| has_code(d, STALE_CLONE))
        .collect();
    assert!(hits.is_empty(), "current signature must not warn, got {hits:#?}");
}

#[test]
fn stale_clone_wrong_symlink_target() {
    let tmp = tempdir().unwrap();
    let peer = tmp.path().join("peer");
    let other = tmp.path().join("other");
    std::fs::create_dir_all(&peer).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    symlink_dir(&other, &workspace.join("peer"));
    let registry = registry_with(vec![rp("peer", "./peer", "omnia@1.0.0", "peer service")]);
    let plan = plan_with_changes(vec![]);
    let hits: Vec<_> = doctor(&plan, None, Some(&registry), Some(tmp.path()))
        .into_iter()
        .filter(|d| has_code(d, STALE_CLONE))
        .collect();
    assert_eq!(hits.len(), 1, "wrong symlink target must surface stale slot");
    let payload = data(&hits[0]);
    assert_eq!(payload["reason"], "slot-mismatch");
    assert_eq!(payload["expected"]["slot-kind"], "symlink");
    assert_eq!(payload["observed"]["slot-kind"], "symlink");
    assert!(
        payload["observed"]["target"].as_str().expect("observed target").contains("other"),
        "observed target should name the wrong symlink target"
    );
}

#[test]
fn stale_clone_ignores_missing_slots() {
    let tmp = tempdir().unwrap();
    let registry = registry_with(vec![rp("self", ".", "omnia@1.0.0", "self service")]);
    let plan = plan_with_changes(vec![]);
    let any_stale = doctor(&plan, None, Some(&registry), Some(tmp.path()))
        .into_iter()
        .any(|d| has_code(&d, STALE_CLONE));
    assert!(!any_stale, "missing slots are left to workspace sync");
}

// ------- Combined / negative cases --------------------------------

#[test]
fn healthy_plan_no_diagnostics() {
    let plan = plan_with_sources(
        vec![("monolith", "/m")],
        vec![
            {
                let mut e = change("a", Status::Done);
                e.sources = vec![SliceSourceBinding::bare("monolith")];
                e
            },
            {
                let mut e = change_with_deps("b", Status::Pending, &["a"]);
                e.sources = vec![SliceSourceBinding::bare("monolith")];
                e
            },
        ],
    );
    let diagnostics = doctor(&plan, None, None, None);
    for code in [CYCLE, ORPHAN_SOURCE, STALE_CLONE] {
        assert!(
            !diagnostics.iter().any(|d| has_code(d, code)),
            "healthy plan should not emit {code}: {diagnostics:#?}"
        );
    }
}

#[test]
fn includes_validate_findings() {
    // A plan with an unknown depends-on (validate-only). Doctor must
    // forward the validate diagnostic with code unchanged.
    let plan = plan_with_changes(vec![
        change("a", Status::Done),
        change_with_deps("b", Status::Pending, &["a", "ghost"]),
    ]);
    let diagnostics = doctor(&plan, None, None, None);
    assert!(
        diagnostics.iter().any(|d| has_code(d, "unknown-depends-on")),
        "validate's `unknown-depends-on` must pass through doctor unchanged: {diagnostics:#?}"
    );
}

/// A13: every health diagnostic is the neutral currency. A doctor
/// finding serialises with kebab-case keys, carries the stable code as
/// `rule-id`, maps the orphan-source warning to a non-blocking
/// `suggestion`, and preserves its machine-readable payload on the
/// structured evidence (`evidence.data`) — validating against the
/// shared diagnostic schema.
#[test]
fn doctor_finding_is_canonical_diagnostic() {
    let mut e = change("a", Status::Pending);
    e.sources = vec![SliceSourceBinding::bare("monolith")];
    let plan = plan_with_sources(vec![("monolith", "/path"), ("orphan", "/elsewhere")], vec![e]);
    let hit = doctor(&plan, None, None, None)
        .into_iter()
        .find(|d| has_code(d, ORPHAN_SOURCE))
        .expect("orphan-source diagnostic");

    specify_diagnostics::validate_diagnostic(&hit).expect("doctor finding is valid");
    assert!(specify_diagnostics::verify_fingerprint(&hit), "fingerprint covers evidence");

    let v = serde_json::to_value(&hit).expect("serialise");
    assert_eq!(v["severity"], "suggestion");
    assert_eq!(v["rule-id"], ORPHAN_SOURCE);
    assert_eq!(v["artifact"], "plan");
    assert_eq!(v["evidence"]["kind"], "structured");
    assert_eq!(v["evidence"]["data"]["key"], "orphan");
}
