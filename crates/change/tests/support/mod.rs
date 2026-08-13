//! Suite-local source bindings and plan builders over `change` wire
//! types.
//!
//! The builders return `change` types, so they live with the owning
//! suites rather than in the mock crate, keeping `mock` free of
//! workflow-crate dependencies.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use change::plan::wire::SourceAssign;
use mock::session::Session;
use project::handler::Anchor as _;
use serde_json::json;

/// Drain the serial refinement stage through the public `plan refine`
/// handler over every in-scope leaf (RFC-91 D1/D7).
///
/// # Panics
///
/// Panics when the drain stops instead of completing.
pub async fn refine_plan(session: &Session) -> change::plan::handlers::RefineBody {
    refine_slices(session, &[]).await.expect("plan refine drains")
}

/// `emery plan refine --slice …` through the public handler; empty
/// selectors target every in-scope leaf.
///
/// # Errors
///
/// Propagates the handler's failures, including the typed
/// `plan-refine-stopped` halt.
pub async fn refine_slices(
    session: &Session, slices: &[&str],
) -> Result<change::plan::handlers::RefineBody, project::handler::Error> {
    mock::invoke::run::<change::plan::handlers::Refine, _, _>(
        session.provider(),
        change::plan::handlers::RefineInput {
            slice: slices.iter().map(ToString::to_string).collect(),
        },
    )
    .await
}

/// Refinement digest (`sha256:…`) of the slice's on-disk
/// `refinement.yaml` — for coverage assertions.
///
/// # Panics
///
/// Panics when the manifest is absent or unreadable.
#[must_use]
pub fn manifest_digest(root: &std::path::Path, slice: &str) -> project::snapshot::SnapshotId {
    slice::refinement::file_digest(&project::config::Layout::new(root).slice_dir(slice))
        .expect("read refinement manifest")
        .expect("refinement manifest present")
}

/// Hand-stage a fresh refinement manifest for a hand-written slice
/// tree: write the minimal bundle files (proposal / design / tasks +
/// one per-domain spec) when absent, then assemble and persist
/// `refinement.yaml` against the live `plan.yaml`, so execute's
/// freshness recompute passes without a model-driven refine.
///
/// # Panics
///
/// Panics when the plan, the entry, or a write is unavailable.
pub fn stage_manifest(root: &std::path::Path, slice: &str) {
    let layout = project::config::Layout::new(root);
    let plan = project::plan::Plan::load(&layout.plan_path()).expect("plan.yaml");
    let entry =
        plan.entries.iter().find(|e| e.name == slice).expect("plan entry for slice").clone();
    let slice_dir = layout.slice_dir(slice);
    let domain_dir = slice_dir.join("specs/main");
    std::fs::create_dir_all(&domain_dir).expect("specs domain dir");
    for (name, body) in
        [("proposal.md", "# proposal\n"), ("design.md", "# design\n"), ("tasks.md", "# tasks\n")]
    {
        let path = slice_dir.join(name);
        if !path.exists() {
            std::fs::write(&path, body).expect("bundle file");
        }
    }
    let spec = domain_dir.join("spec.md");
    if !spec.exists() {
        std::fs::write(&spec, "## main\n").expect("spec.md");
    }
    // The declared target binding is covered by the entry projection,
    // so assembly must read the same `project.yaml` value freshness
    // recomputes against (an uninitialised root degrades to none).
    let config = project::config::ProjectConfig::load(root).ok();
    slice::refinement::assemble(
        layout,
        &plan,
        &entry,
        &[],
        slice::refinement::TargetInputs {
            guidance: slice::refinement::empty_digest(),
            declarations: &[],
            reference: config.as_ref().and_then(|c| c.adapter.as_deref()),
        },
        Vec::new(),
    )
    .expect("assemble refinement manifest")
    .write(&slice_dir)
    .expect("write refinement manifest");
}

/// Drive the refine phase for one plan entry directly through the
/// slice orchestration — for suites that pin a single slice without
/// draining the whole plan.
///
/// # Errors
///
/// Propagates the orchestration's failures.
///
/// # Panics
///
/// Panics when `plan.yaml` is absent or carries no entry named
/// `slice`.
pub async fn refine(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::RefineOutcome, error::Error> {
    let provider = session.provider();
    let caps = slice::orchestrate::Capabilities::provider(provider);
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let plan = project::plan::Plan::load(&layout.plan_path())?;
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.name == slice)
        .unwrap_or_else(|| panic!("plan entry `{slice}` missing"));
    let target = match project::slice::SliceMetadata::load_opt(&layout.slice_dir(slice))? {
        Some(meta) => meta.target,
        None => project::target_policy::fresh(provider, paths, entry, slice, "refining")?,
    };
    let config = project::config::ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(provider, &config, paths)?;
    slice::orchestrate::refine(
        caps,
        paths,
        jiff::Timestamp::now(),
        slice,
        &target,
        Vec::new(),
        &adapter.manifest.inputs,
    )
    .await
}

/// Claim the next eligible plan entry — the execute loop's advance
/// step, reached directly here so tests can park a claimed slice.
///
/// # Panics
///
/// Panics when config load or the advance kernel fails.
pub fn advance(session: &Session) -> project::plan::AdvanceBody {
    let provider = session.provider();
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let config = project::config::ProjectConfig::load(layout.project_dir()).expect("config loads");
    project::plan::advance_next(provider, paths, jiff::Timestamp::now(), &config)
        .expect("advance claims")
}

/// Drive the build phase for one slice the way the execute loop does.
///
/// # Errors
///
/// Propagates config-load, adapter-resolution, and build failures.
pub async fn build(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::BuildOutcome, error::Error> {
    let provider = session.provider();
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let config = project::config::ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(provider, &config, paths)?;
    slice::orchestrate::build(provider, layout, jiff::Timestamp::now(), slice, &adapter.manifest)
        .await
}

/// Drive the merge phase for one slice the way the execute loop does
/// (no composition-replace authorization).
///
/// # Errors
///
/// Propagates the merge orchestration's failures.
pub async fn merge(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::MergeOutcome, error::Error> {
    let provider = session.provider();
    let layout = project::config::Layout::new(provider.paths().project_root());
    slice::orchestrate::merge(provider, layout, jiff::Timestamp::now(), slice, false).await
}

/// The single `main` binding onto the minimal mock source.
///
/// # Panics
///
/// Panics when the binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn greeting_binding() -> Vec<SourceAssign> {
    greeting_binding_for("mock")
}

/// The single `main` binding onto the named mock source adapter
/// (for the typed-failure profiles).
///
/// # Panics
///
/// Panics when the binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn greeting_binding_for(adapter: &str) -> Vec<SourceAssign> {
    let main: SourceAssign = serde_json::from_value(
        json!({ "key": "main", "adapter": adapter, "value": "The greeting service." }),
    )
    .expect("mock binding parses");
    vec![main]
}

/// The adversarial two-source pair: a docs source and a code source,
/// both served by the mock core under different adapter names.
///
/// # Panics
///
/// Panics when a binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn adversarial_bindings() -> Vec<SourceAssign> {
    ["docs", "code"]
        .map(|key| {
            serde_json::from_value(json!({
                "key": key,
                "adapter": format!("mock-{key}"),
                "value": format!("The {key} source."),
            }))
            .expect("mock binding parses")
        })
        .to_vec()
}

/// A minimal in-memory plan named `test` wrapping `changes`.
#[must_use]
pub fn plan_with_changes(changes: Vec<project::plan::Entry>) -> project::plan::Plan {
    project::plan::Plan {
        name: "test".into(),
        sources: std::collections::BTreeMap::new(),
        entries: changes,
    }
}

/// A minimal plan entry bound to project `default`.
#[must_use]
pub fn change(name: &str) -> project::plan::Entry {
    project::plan::Entry {
        name: name.into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
        allow_composition_replace: false,
    }
}

/// [`change()`] plus a `depends-on` list.
#[must_use]
pub fn change_with_deps(name: &str, deps: &[&str]) -> project::plan::Entry {
    let mut entry = change(name);
    entry.depends_on = deps.iter().map(|s| (*s).into()).collect();
    entry
}

/// Author the single-slice greeting plan and refine it to Refined —
/// the fixture floor for the RFC-90 build-phase suites.
///
/// # Panics
///
/// Panics when author or refine fails.
pub async fn greeting_ready(session: &Session) {
    mock::invoke::run::<change::plan::handlers::Author, _, _>(
        session.provider(),
        change::plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");
    refine(session, "greeting").await.expect("refine");
}

/// Drop one mock control-plane marker file at the project root.
///
/// # Panics
///
/// Panics when the marker cannot be written.
pub fn marker(root: &std::path::Path, name: &str) {
    std::fs::write(root.join(name), "").expect("write marker");
}

/// The attempt directory
/// `.emery/change/slices/<slice>/build/attempts/<NNNN>/`.
#[must_use]
pub fn attempt_dir(root: &std::path::Path, slice: &str, attempt: u32) -> std::path::PathBuf {
    root.join(".emery/change/slices")
        .join(slice)
        .join("build/attempts")
        .join(format!("{attempt:04}"))
}

/// Sorted `phases/` file names of one attempt.
///
/// # Panics
///
/// Panics when the attempt has no readable `phases/` directory.
#[must_use]
pub fn phase_files(root: &std::path::Path, slice: &str, attempt: u32) -> Vec<String> {
    let dir = attempt_dir(root, slice, attempt).join("phases");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("phases dir `{}`: {err}", dir.display()))
        .map(|entry| entry.expect("phase entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `(attempt, ordinal, operation, source)` of every
/// `slice.build.phase-completed` fact, in journal order.
///
/// # Panics
///
/// Panics when the journal union cannot be read.
#[must_use]
pub fn phase_events(root: &std::path::Path) -> Vec<(u32, u32, String, String)> {
    project::journal::read_union(project::config::Layout::new(root))
        .expect("journal union")
        .into_iter()
        .filter_map(|event| match event.kind {
            project::journal::EventKind::SliceBuildPhaseCompleted {
                attempt,
                ordinal,
                operation,
                source,
                ..
            } => Some((attempt, ordinal, operation, source)),
            _ => None,
        })
        .collect()
}

/// The parsed canonical `build/report.yaml` terminal projection.
///
/// # Panics
///
/// Panics when the canonical report is absent or stops parsing.
#[must_use]
pub fn canonical_report(root: &std::path::Path, slice: &str) -> slice::BuildReport {
    let path = root.join(".emery/change/slices").join(slice).join("build/report.yaml");
    let yaml = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("canonical report `{}`: {err}", path.display()));
    serde_saphyr::from_str(&yaml).expect("canonical report parses")
}

/// Rule ids carried by a failing validate operation's report.
///
/// # Panics
///
/// Panics when `err` is not a report-carrying failure.
#[must_use]
pub fn report_rule_ids(err: &project::handler::Error) -> Vec<String> {
    let project::handler::Error::Report {
        body: project::handler::FailureBody::Findings(body),
        ..
    } = err
    else {
        panic!("expected findings report error, got {err:?}");
    };
    body.report().findings.iter().filter_map(|finding| finding.rule_id.clone()).collect()
}
