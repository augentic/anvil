//! Suite-local source bindings and plan builders over `change` wire
//! types.
//!
//! The builders return `change` types, so they live with the owning
//! suites rather than in the mock crate, keeping `mock` free of
//! workflow-crate dependencies.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use artifacts::leads::{Lead, Leads};
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::handler::Anchor as _;
use project::plan::{Entry, Plan, SliceSourceBinding, SourceBinding, TargetBinding};
use project::snapshot::SnapshotId;

/// Stub `plan.yaml.targets.default` row for in-memory plans.
#[must_use]
pub fn stub_target() -> TargetBinding {
    stub_target_named("mock")
}

fn stub_target_named(adapter: &str) -> TargetBinding {
    TargetBinding::new(
        Pin::emery(adapter, semver::Version::new(0, 0, 0)),
        ".",
        SnapshotId::from_digest(&"0".repeat(64)),
    )
}

/// Value-backed source row with an exact pin.
#[must_use]
pub fn source_value(adapter: &str, value: &str) -> SourceBinding {
    SourceBinding::intent(Pin::emery(adapter, semver::Version::new(0, 0, 0)), value)
}

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
pub fn manifest_digest(root: &std::path::Path, slice: &str) -> SnapshotId {
    slice::refinement::file_digest(&fixture_layout(root).slice_dir(slice))
        .expect("read refinement manifest")
        .expect("refinement manifest present")
}

/// In-place layout when `project.yaml` exists; detached otherwise.
#[must_use]
pub fn fixture_layout(root: &std::path::Path) -> project::config::Layout<'_> {
    if root.join(".emery/project.yaml").is_file() {
        project::config::Layout::new(root)
    } else {
        project::config::Layout::detached(root)
    }
}

/// Hand-stage a fresh refinement manifest for a hand-written slice
/// tree.
///
/// # Panics
///
/// Panics when the plan, the entry, or a write is unavailable.
pub fn stage_manifest(root: &std::path::Path, slice: &str) {
    let layout = fixture_layout(root);
    let plan = Plan::load(&layout.plan_path()).expect("plan.yaml");
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
    let catalog =
        Leads::load(&layout.leads_path()).unwrap_or_else(|_| Leads::from_leads(Vec::new()));
    slice::refinement::assemble(
        layout,
        &plan,
        &entry,
        catalog.leads(),
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
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.name == slice)
        .unwrap_or_else(|| panic!("plan entry `{slice}` missing"));
    let target = match project::slice::SliceMetadata::load_opt(&layout.slice_dir(slice))? {
        Some(meta) => meta.target,
        None => project::target_policy::fresh(provider, paths, entry, slice, "refining")?,
    };
    let binding = plan.target(&entry.target)?;
    let adapter =
        project::adapter::Resolver::resolve_target(provider, &binding.adapter.selector(), paths)?;
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
/// Panics when the advance kernel fails.
pub fn advance(session: &Session) -> project::plan::AdvanceBody {
    let provider = session.provider();
    project::plan::advance_next(provider, provider.paths(), jiff::Timestamp::now())
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
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.name == slice)
        .unwrap_or_else(|| panic!("plan entry `{slice}` missing"));
    let binding = plan.target(&entry.target)?;
    let adapter =
        project::adapter::Resolver::resolve_target(provider, &binding.adapter.selector(), paths)?;
    Box::pin(slice::orchestrate::build(
        provider,
        layout,
        jiff::Timestamp::now(),
        slice,
        &adapter.manifest,
    ))
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
    let layout = provider.paths().layout();
    slice::orchestrate::merge(provider, layout, jiff::Timestamp::now(), slice, false).await
}

/// The single `main` value binding onto the minimal mock source.
#[must_use]
pub fn greeting_sources() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![("main", "mock", "The greeting service.")]
}

/// Write a fixture `plan.yaml` + `leads.md` for suites that do not
/// need the full `plan author` judgment path.
///
/// `sources` is `(key, adapter, value)`; `slices` is `(name, source, lead)`.
///
/// # Panics
///
/// Panics when the fixture write fails.
pub fn write_plan_fixture(
    root: &std::path::Path, name: &str, sources: &[(&str, &str, &str)],
    slices: &[(&str, &str, &str)],
) {
    let layout = fixture_layout(root);
    std::fs::create_dir_all(layout.change_root()).expect("change home");
    let mut plan = Plan::named(name);
    let adapter = project::config::ProjectConfig::load(root)
        .ok()
        .and_then(|config| config.adapter)
        .unwrap_or_else(|| "mock".into());
    plan.targets.insert("default".into(), stub_target_named(&adapter));
    for (key, adapter, value) in sources {
        plan.sources.insert((*key).into(), source_value(adapter, value));
    }
    let mut imported = Vec::new();
    for (slice, source, lead) in slices {
        if plan.entries.iter().any(|entry| entry.name.as_str() == *slice) {
            let entry =
                plan.entries.iter_mut().find(|entry| entry.name.as_str() == *slice).expect("slice");
            entry.sources.push(SliceSourceBinding::structured(*source, *lead));
        } else {
            let mut entry = Entry::named(*slice, "default");
            entry.sources = vec![SliceSourceBinding::structured(*source, *lead)];
            plan.entries.push(entry);
        }
        imported.push(Lead::new(*lead, *source, *lead));
    }
    plan.save(&layout.plan_path()).expect("plan.yaml");
    Leads::from_leads(imported).write_atomic(&layout.leads_path()).expect("leads.md");
}

/// A minimal in-memory plan named `test` wrapping `changes`.
#[must_use]
pub fn plan_with_changes(changes: Vec<Entry>) -> Plan {
    let mut plan = Plan::named("test");
    plan.targets.insert("default".into(), stub_target());
    plan.entries = changes;
    plan
}

/// A minimal plan entry bound to target `default`.
#[must_use]
pub fn change(name: &str) -> Entry {
    Entry::named(name, "default")
}

/// [`change()`] plus a `depends-on` list.
#[must_use]
pub fn change_with_deps(name: &str, deps: &[&str]) -> Entry {
    let mut entry = change(name);
    entry.depends_on = deps.iter().map(|s| (*s).into()).collect();
    entry
}

/// Write a fixture greeting plan and refine it to Refined — the floor
/// for the RFC-90 build-phase suites. These suites stay fixture-driven
/// so they do not re-enter the author judgment path.
///
/// # Panics
///
/// Panics when the fixture write or refine fails.
pub async fn greeting_ready(session: &Session) {
    let root = session.provider().paths().project_root();
    write_greeting_plan(root);
    refine(session, "greeting").await.expect("refine");
}

/// Adversarial two-source fixture used by refine/execute suites that
/// previously went through survey-driven authoring.
pub fn write_adversarial_plan(root: &std::path::Path) {
    write_plan_fixture(
        root,
        "auth",
        &[("docs", "mock-docs", "The docs source."), ("code", "mock-code", "The code source.")],
        &[
            ("login-flow", "docs", "login-flow"),
            ("login-flow", "code", "login-flow"),
            ("session-policy", "docs", "session-timeout"),
            ("session-policy", "code", "session-timeout"),
            ("password-reset", "docs", "password-reset"),
        ],
    );
}

pub fn write_greeting_plan(root: &std::path::Path) {
    write_plan_fixture(
        root,
        "demo",
        &[("main", "mock", "The greeting service.")],
        &[("greeting", "main", "greeting")],
    );
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
