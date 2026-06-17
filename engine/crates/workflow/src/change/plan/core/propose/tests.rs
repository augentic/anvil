use std::path::Path;

use specify_error::Error;
use specify_model::discovery::Discovery;

use super::super::model::{Lifecycle, SourceBinding};
use super::*;
use crate::change::{Plan, SliceSourceBinding, Status};
use crate::config::ProjectConfig;
use crate::name::{PlanName, SliceName};
use crate::platform::Platform;
use crate::registry::Surface;
use crate::schema::validate_proposal_json;

fn discovery(body: &str) -> Discovery {
    Discovery::parse(body).expect("discovery parses")
}

fn project(name: &str, target: &str, description: &str) -> ProjectRef {
    ProjectRef {
        name: name.to_string(),
        target: target.to_string(),
        description: Some(description.to_string()),
        surface: Vec::new(),
        recent: Vec::new(),
        decisions: Vec::new(),
        decisions_more: None,
        platforms: Vec::new(),
    }
}

// The N=1 `build_request` shape is pinned at the CLI level by
// `tests/workflow/propose.rs::propose_dry_run_n1_request_golden`
// (`propose-dry-run-n1-request.json`). The hub case below has no CLI
// dry-run request golden, so it stays as the multi-project coverage.
#[test]
fn build_request_hub_validates_as_request() {
    let doc = discovery(
        "## Lead inventory\n\n\
             ### docs:identity-api\n\n\
             - lead: identity-api\n\
             - source: docs\n\
             - synopsis: Identity API contract.\n\n\
             ### legacy:identity-api\n\n\
             - lead: identity-api\n\
             - source: legacy\n\
             - synopsis: Legacy identity endpoints.\n\n\
             ### docs:password-reset\n\n\
             - lead: password-reset\n\
             - source: docs\n\
             - synopsis: Users can request a password reset email.\n",
    );
    let topology = vec![
        project("identity-contracts", "contracts@1.0.0", "Versioned API contracts crate."),
        project("identity-service", "omnia@1.0.0", "Omnia identity service."),
    ];

    let request = build_request(&doc, &topology).expect("request builds");
    assert_eq!(request.leads.len(), 3);

    let json = serde_json::to_string(&request).expect("serialise request");
    validate_proposal_json(&json).expect("hub request validates against the schema");
}

#[test]
fn build_request_empty_catalog_errors() {
    let doc = discovery("# Discovery\n\nNo leads surveyed yet.\n\n## Lead inventory\n");
    let topology = vec![project("my-app", "omnia@1.0.0", "Single service.")];

    match build_request(&doc, &topology) {
        Err(Error::Validation { code, .. }) => {
            assert_eq!(code, "plan-reconcile-empty-catalog");
        }
        other => panic!("expected empty-catalog validation error, got {other:?}"),
    }
}

#[test]
fn build_catalog_membership_and_size() {
    let doc = discovery(
        "## Lead inventory\n\n\
             ### docs:identity-api\n\n\
             - lead: identity-api\n\
             - source: docs\n\
             - synopsis: Identity API.\n\n\
             ### legacy:identity-api\n\n\
             - lead: identity-api\n\
             - source: legacy\n\
             - synopsis: Legacy identity.\n",
    );
    let catalog = build_catalog(&doc);

    assert_eq!(catalog.len(), 2);
    assert!(!catalog.is_empty());
    assert!(catalog.contains("docs", "identity-api"));
    assert!(catalog.contains("legacy", "identity-api"));
    // Same slug under the wrong source is not in the catalog.
    assert!(!catalog.contains("docs", "password-reset"));
}

#[test]
fn response_rfc_multi_source() {
    // Multi-source fan-out response (the proposal-schema envelope example).
    // Fan-out is two ordinary slices that reference the same lead and
    // are joined by `depends-on` — there is no `scope` grouping.
    let yaml = "\
version: 1
kind: response
slices:
  - name: identity-contracts
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-contracts
    rationale: \"identity API surface matched by shared slug across docs + legacy\"
  - name: identity-service
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-service
    depends-on: [identity-contracts]
  - name: password-reset
    sources:
      - { source: docs, lead: password-reset }
      - { source: legacy, lead: reset-password }
    project: identity-service
    rationale: \"password-reset (docs) and reset-password (legacy) are the same flow by synopsis judgment\"
";
    let response: ProposalResponse = serde_saphyr::from_str(yaml).expect("response deserialises");

    assert_eq!(response.version, PROPOSAL_VERSION);
    assert_eq!(response.kind, ProposalKind::Response);
    assert_eq!(response.slices.len(), 3);

    let contracts = &response.slices[0];
    assert_eq!(contracts.name, "identity-contracts");
    assert_eq!(contracts.project.as_deref(), Some("identity-contracts"));
    assert_eq!(contracts.sources.len(), 2);
    assert_eq!(contracts.sources[0].source, "docs");
    assert_eq!(contracts.sources[0].lead, "identity-api");
    assert!(contracts.depends_on.is_empty());

    let service = &response.slices[1];
    assert_eq!(service.name, "identity-service");
    assert_eq!(service.depends_on, vec!["identity-contracts"]);
    assert!(service.rationale.is_none());

    let reset = &response.slices[2];
    assert_eq!(reset.name, "password-reset");
    assert_eq!(reset.sources[1].source, "legacy");
    assert_eq!(reset.sources[1].lead, "reset-password");

    // The DTO re-serialises into a schema-valid response, locking the
    // shape the projection kernel will consume.
    let json = serde_json::to_string(&response).expect("serialise response");
    validate_proposal_json(&json).expect("round-tripped response validates");
}

fn hub_config() -> ProjectConfig {
    ProjectConfig {
        name: "platform".to_string(),
        description: None,
        adapter: None,
        specify_version: None,
        rules: std::collections::BTreeMap::new(),
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: true,
    }
}

#[test]
fn resolve_topology_hub_reads_topology_lock() {
    // Workspace topology is projected from the committed
    // `.specify/topology.lock`, not `registry.yaml`.
    let dir = tempfile::tempdir().expect("tempdir");
    let specify = dir.path().join(".specify");
    std::fs::create_dir_all(&specify).expect("mkdir .specify");
    std::fs::write(
        specify.join("topology.lock"),
        "version: 1\n\
             projects:\n  \
               - name: identity-contracts\n    \
                 target: contracts@1.0.0\n    \
                 description: Contracts crate.\n    \
                 surface:\n      \
                   - domain: identity-api\n        \
                     requirements:\n          \
                       - Authenticate user\n  \
               - name: identity-service\n    \
                 target: omnia@1.0.0\n",
    )
    .expect("write topology.lock");

    let topology = resolve_topology(&hub_config(), dir.path()).expect("hub topology resolves");
    assert_eq!(
        topology,
        vec![
            ProjectRef {
                name: "identity-contracts".to_string(),
                target: "contracts@1.0.0".to_string(),
                description: Some("Contracts crate.".to_string()),
                surface: vec![Surface {
                    domain: "identity-api".to_string(),
                    requirements: vec!["Authenticate user".to_string()],
                    more: None,
                }],
                recent: Vec::new(),
                decisions: Vec::new(),
                decisions_more: None,
                platforms: Vec::new(),
            },
            ProjectRef {
                name: "identity-service".to_string(),
                target: "omnia@1.0.0".to_string(),
                description: None,
                surface: Vec::new(),
                recent: Vec::new(),
                decisions: Vec::new(),
                decisions_more: None,
                platforms: Vec::new(),
            },
        ]
    );
}

#[test]
fn topology_hub_missing_cache() {
    let dir = tempfile::tempdir().expect("tempdir");
    match resolve_topology(&hub_config(), dir.path()) {
        Err(Error::Validation { code, .. }) => assert_eq!(code, "topology-cache-missing"),
        other => panic!("expected topology-cache-missing, got {other:?}"),
    }
}

#[test]
fn topology_regular_no_adapter() {
    let config = ProjectConfig {
        name: "demo".to_string(),
        description: None,
        adapter: None,
        specify_version: None,
        rules: std::collections::BTreeMap::new(),
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: false,
    };
    match resolve_topology(&config, Path::new("/unused")) {
        Err(Error::Validation { code, .. }) => {
            assert_eq!(code, "plan-propose-project-adapter-missing");
        }
        other => panic!("expected adapter-missing validation error, got {other:?}"),
    }
}

// --- propose_from projection kernel ---------------------------------

fn member(source: &str, lead: &str) -> ResponseMember {
    ResponseMember {
        source: source.to_string(),
        lead: lead.to_string(),
    }
}

fn slice(name: &str, sources: Vec<ResponseMember>) -> ResponseSlice {
    ResponseSlice {
        name: name.to_string(),
        sources,
        rationale: None,
        depends_on: Vec::new(),
        project: None,
        divergence: None,
        disagreements: Vec::new(),
    }
}

fn response(slices: Vec<ResponseSlice>) -> ProposalResponse {
    ProposalResponse {
        version: PROPOSAL_VERSION,
        kind: ProposalKind::Response,
        slices,
    }
}

fn discovery_with(leads: &[(&str, &str)]) -> Discovery {
    let body: String = std::iter::once("## Lead inventory\n\n".to_string())
        .chain(leads.iter().map(|(source, lead)| {
            format!(
                "### {source}:{lead}\n\n\
                     - lead: {lead}\n\
                     - source: {source}\n\
                     - synopsis: {lead} synopsis\n\n",
            )
        }))
        .collect();
    discovery(&body)
}

fn plan_with_sources(lifecycle: Lifecycle, keys: &[&str]) -> Plan {
    Plan {
        name: PlanName::new("p"),
        lifecycle,
        sources: keys
            .iter()
            .map(|k| ((*k).to_string(), SourceBinding::value("intent", "brief")))
            .collect(),
        entries: Vec::new(),
    }
}

fn assert_code(result: specify_error::Result<ProposeOutcome>, expected: &str) {
    match result {
        Err(Error::Validation { code, .. }) => assert_eq!(code, expected),
        other => panic!("expected {expected} validation error, got {other:?}"),
    }
}

#[test]
fn propose_rejects_non_replaceable_plan() {
    let mut plan = plan_with_sources(Lifecycle::Approved, &["intent"]);
    let doc = discovery_with(&[("intent", "fix-typo")]);
    let topo = vec![project("my-app", "omnia@1.0.0", "Single service.")];
    let resp = response(vec![slice("fix-typo", vec![member("intent", "fix-typo")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "plan-reconcile-plan-not-replaceable");
}

#[test]
fn propose_rejects_lead_orphan() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "real")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let resp = response(vec![slice("s", vec![member("docs", "ghost")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "plan-reconcile-lead-orphan");
}

#[test]
fn propose_rejects_slice_source_collision() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a"), ("docs", "b")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let resp = response(vec![slice("s", vec![member("docs", "a"), member("docs", "b")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "plan-reconcile-slice-source-collision");
}

#[test]
fn propose_emits_topic_overlap_review() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery(
        "## Lead inventory\n\n\
             ### intent:add-feature\n\n\
             - lead: add-feature\n\
             - source: intent\n\
             - synopsis: Add a feature.\n\
             - topics: [identity, validation]\n",
    );
    let mut proj = project("my-app", "omnia@1.0.0", "service");
    proj.decisions = vec![crate::registry::topology::Decision {
        id: "DEC-0001".to_string(),
        title: "Identity store is Postgres".to_string(),
        topics: vec!["identity".to_string()],
    }];
    let topo = vec![proj];
    let resp = response(vec![slice("s", vec![member("intent", "add-feature")])]);

    let out = plan.propose_from(resp, &doc, &topo).expect("projects");
    assert_eq!(out.topic_overlaps.len(), 1, "got {:#?}", out.topic_overlaps);
    let finding = &out.topic_overlaps[0];
    assert_eq!(finding.rule_id.as_deref(), Some("lead-decision-topic-overlap"));
    assert_eq!(finding.kind, specify_diagnostics::DiagnosticKind::Review);
    assert!(!specify_diagnostics::blocking(finding), "topic overlap is advisory");
    assert!(finding.impact.contains("DEC-0001"), "{}", finding.impact);
}

#[test]
fn topic_overlap_empty_without_topics() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery(
        "## Lead inventory\n\n\
             ### intent:add-feature\n\n\
             - lead: add-feature\n\
             - source: intent\n\
             - synopsis: Add a feature.\n\
             - topics: [identity]\n",
    );
    // Decision present but carries no topics — the join degrades to empty.
    let mut proj = project("my-app", "omnia@1.0.0", "service");
    proj.decisions = vec![crate::registry::topology::Decision {
        id: "DEC-0001".to_string(),
        title: "Some decision".to_string(),
        topics: Vec::new(),
    }];
    let topo = vec![proj];
    let resp = response(vec![slice("s", vec![member("intent", "add-feature")])]);

    let out = plan.propose_from(resp, &doc, &topo).expect("projects");
    assert!(out.topic_overlaps.is_empty(), "got {:#?}", out.topic_overlaps);
}

#[test]
fn propose_rejects_coverage_gap() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a"), ("docs", "b")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    // Catalog carries two leads; the response covers only one.
    let resp = response(vec![slice("s", vec![member("docs", "a")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "lead-coverage-orphan");
}

#[test]
fn propose_rejects_project_binding_required() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a")]);
    let topo =
        vec![project("p1", "omnia@1.0.0", "first"), project("p2", "contracts@1.0.0", "second")];
    // Two projects offered, slice omits `project`.
    let resp = response(vec![slice("s", vec![member("docs", "a")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "plan-reconcile-project-binding-required");
}

#[test]
fn propose_rejects_project_orphan() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let mut s = slice("s", vec![member("docs", "a")]);
    s.project = Some("ghost".to_string());
    assert_code(plan.propose_from(response(vec![s]), &doc, &topo), "plan-reconcile-project-orphan");
}

#[test]
fn propose_rejects_slice_name_collision() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a"), ("docs", "b")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let mut s1 = slice("dup", vec![member("docs", "a")]);
    s1.project = Some("p".to_string());
    let mut s2 = slice("dup", vec![member("docs", "b")]);
    s2.project = Some("p".to_string());
    assert_code(
        plan.propose_from(response(vec![s1, s2]), &doc, &topo),
        "plan-reconcile-slice-name-collision",
    );
}

#[test]
fn propose_rejects_slice_name_not_kebab() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let resp = response(vec![slice("Not Kebab", vec![member("docs", "a")])]);
    assert_code(plan.propose_from(resp, &doc, &topo), "plan-reconcile-slice-name-invalid");
}

#[test]
fn propose_rejects_depends_on_cycle() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs"]);
    let doc = discovery_with(&[("docs", "a"), ("docs", "b")]);
    let topo = vec![project("p", "omnia@1.0.0", "svc")];
    let mut s1 = slice("alpha", vec![member("docs", "a")]);
    s1.project = Some("p".to_string());
    s1.depends_on = vec!["beta".to_string()];
    let mut s2 = slice("beta", vec![member("docs", "b")]);
    s2.project = Some("p".to_string());
    s2.depends_on = vec!["alpha".to_string()];
    assert_code(
        plan.propose_from(response(vec![s1, s2]), &doc, &topo),
        "plan-reconcile-depends-on-cycle",
    );
}

#[test]
fn propose_n1_auto_binds_sole_project() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery_with(&[("intent", "fix-typo")]);
    let topo = vec![project("my-app", "omnia@1.0.0", "Single Omnia service.")];
    // Explicit name, no project (auto-bound to the sole project).
    let resp = response(vec![slice("fix-typo", vec![member("intent", "fix-typo")])]);

    let out = plan.propose_from(resp, &doc, &topo).expect("N=1 projects");

    assert_eq!(out.slice_names, vec!["fix-typo"]);

    assert_eq!(plan.entries.len(), 1);
    let entry = &plan.entries[0];
    assert_eq!(entry.name, "fix-typo");
    assert_eq!(entry.project.as_deref(), Some("my-app"));
    // Target is not stored; it resolves from the bound project.
    assert_eq!(resolve_target(entry, &topo).unwrap().to_string(), "omnia@1.0.0");
    assert_eq!(entry.status, Status::Pending);
    assert!(entry.depends_on.is_empty());
    assert_eq!(entry.sources, vec![SliceSourceBinding::structured("intent", "fix-typo")]);
}

#[test]
fn propose_multi_homes_cross_cutting_lead() {
    // Same-project multi-membership: coverage is at-least-once, so a
    // cross-cutting lead (a conventions document informing several
    // work leads from another source) may be bound into every slice
    // it informs within one project. Fan-out is not restricted to the
    // cross-project case, and no `depends-on` edge is implied.
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs", "legacy"]);
    let doc = discovery_with(&[
        ("docs", "conventions"),
        ("legacy", "user-registration"),
        ("legacy", "password-reset"),
    ]);
    let topo = vec![project("identity-service", "omnia@1.0.0", "Omnia identity service.")];

    let resp = response(vec![
        slice(
            "user-registration",
            vec![member("legacy", "user-registration"), member("docs", "conventions")],
        ),
        slice(
            "password-reset",
            vec![member("legacy", "password-reset"), member("docs", "conventions")],
        ),
    ]);

    let out = plan.propose_from(resp, &doc, &topo).expect("multi-homed lead projects");

    assert_eq!(out.slice_names, vec!["user-registration", "password-reset"]);
    assert_eq!(plan.entries.len(), 2);
    for entry in &plan.entries {
        assert_eq!(entry.project.as_deref(), Some("identity-service"));
        assert!(entry.depends_on.is_empty(), "multi-homing implies no depends-on edge");
        assert!(
            entry.sources.contains(&SliceSourceBinding::structured("docs", "conventions")),
            "every informed slice carries the cross-cutting lead: {:?}",
            entry.sources
        );
    }
}

#[test]
fn propose_multi_source_fan_out() {
    let doc = discovery_with(&[
        ("docs", "identity-api"),
        ("legacy", "identity-api"),
        ("docs", "password-reset"),
        ("legacy", "reset-password"),
    ]);
    let topo = vec![
        project("identity-contracts", "contracts@1.0.0", "Versioned API contracts crate."),
        project("identity-service", "omnia@1.0.0", "Omnia identity service."),
    ];
    let mut plan = plan_with_sources(Lifecycle::Pending, &["docs", "legacy"]);

    // Multi-source fan-out response. Fan-out is two ordinary slices
    // referencing the same `identity-api` lead, joined by `depends-on`;
    // there is no `scope` grouping.
    let yaml = "\
version: 1
kind: response
slices:
  - name: identity-contracts
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-contracts
    rationale: \"identity API surface matched by shared slug across docs + legacy\"
  - name: identity-service
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-service
    depends-on: [identity-contracts]
  - name: password-reset
    sources:
      - { source: docs, lead: password-reset }
      - { source: legacy, lead: reset-password }
    project: identity-service
    rationale: \"password-reset (docs) and reset-password (legacy) are the same flow by synopsis judgment\"
";
    let resp: ProposalResponse =
        serde_saphyr::from_str(yaml).expect("multi-source response deserialises");

    let out = plan.propose_from(resp, &doc, &topo).expect("fan-out projects");

    assert_eq!(out.slice_names, vec!["identity-contracts", "identity-service", "password-reset"]);

    assert_eq!(plan.entries.len(), 3);

    let names: Vec<&str> = plan.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["identity-contracts", "identity-service", "password-reset"]);

    let projects: Vec<Option<&str>> = plan.entries.iter().map(|e| e.project.as_deref()).collect();
    assert_eq!(
        projects,
        vec![Some("identity-contracts"), Some("identity-service"), Some("identity-service")]
    );

    // Targets are not stored; each resolves from its bound project.
    let targets: Vec<String> =
        plan.entries.iter().map(|e| resolve_target(e, &topo).unwrap().to_string()).collect();
    assert_eq!(
        targets,
        vec!["contracts@1.0.0".to_string(), "omnia@1.0.0".to_string(), "omnia@1.0.0".to_string()]
    );

    assert_eq!(
        plan.entries[0].sources,
        vec![
            SliceSourceBinding::structured("docs", "identity-api"),
            SliceSourceBinding::structured("legacy", "identity-api"),
        ]
    );
    assert_eq!(
        plan.entries[2].sources,
        vec![
            SliceSourceBinding::structured("docs", "password-reset"),
            SliceSourceBinding::structured("legacy", "reset-password"),
        ]
    );

    assert!(plan.entries[0].depends_on.is_empty());
    assert_eq!(plan.entries[1].depends_on, vec!["identity-contracts"]);
}

// --- reconcile_platforms tests ------------------------------------

fn propose_single_slice(plan: &mut Plan) -> ProposeOutcome {
    let doc = discovery_with(&[("intent", "add-feature")]);
    let topo = vec![project("my-app", "vectis@1.0.0", "Crux app.")];
    let resp = response(vec![slice("add-feature", vec![member("intent", "add-feature")])]);
    plan.propose_from(resp, &doc, &topo).expect("propose succeeds")
}

#[test]
fn reconcile_greenfield_foundation() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].name, "add-feature");

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![Platform::Core, Platform::Ios, Platform::Android],
    }];

    let names = plan.reconcile_platforms(&missing).expect("reconcile succeeds");
    assert_eq!(names, vec!["app-foundation"]);

    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].name, "app-foundation");
    assert_eq!(plan.entries[0].project.as_deref(), Some("my-app"));
    assert!(plan.entries[0].depends_on.is_empty());
    assert!(plan.entries[0].description.is_some());

    assert_eq!(plan.entries[1].name, "add-feature");
    assert_eq!(plan.entries[1].depends_on, vec!["app-foundation"]);
}

#[test]
fn reconcile_incremental_bootstrap() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![Platform::Android],
    }];

    let names = plan.reconcile_platforms(&missing).expect("reconcile succeeds");
    assert_eq!(names, vec!["bootstrap-android"]);

    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].name, "bootstrap-android");
    assert_eq!(plan.entries[0].project.as_deref(), Some("my-app"));

    assert_eq!(plan.entries[1].name, "add-feature");
    assert_eq!(plan.entries[1].depends_on, vec!["bootstrap-android"]);
}

#[test]
fn reconcile_all_present_inserts_nothing() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![],
    }];

    let names = plan.reconcile_platforms(&missing).expect("reconcile succeeds");
    assert!(names.is_empty());
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].name, "add-feature");
    assert!(plan.entries[0].depends_on.is_empty());
}

#[test]
fn reconcile_empty_missing_list_is_noop() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);

    let names = plan.reconcile_platforms(&[]).expect("reconcile succeeds");
    assert!(names.is_empty());
    assert_eq!(plan.entries.len(), 1);
}

#[test]
fn reconcile_incremental_two_missing() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![Platform::Ios, Platform::Android],
    }];

    let names = plan.reconcile_platforms(&missing).expect("reconcile succeeds");
    assert_eq!(names, vec!["bootstrap-ios", "bootstrap-android"]);

    assert_eq!(plan.entries.len(), 3);
    assert_eq!(plan.entries[0].name, "bootstrap-ios");
    assert_eq!(plan.entries[1].name, "bootstrap-android");
    assert_eq!(plan.entries[2].name, "add-feature");
    assert!(plan.entries[2].depends_on.contains(&SliceName::new("bootstrap-ios")));
    assert!(plan.entries[2].depends_on.contains(&SliceName::new("bootstrap-android")));
}

#[test]
fn reconcile_preserves_existing_depends_on() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery_with(&[("intent", "a"), ("intent", "b")]);
    let topo = vec![project("my-app", "vectis@1.0.0", "Crux app.")];
    let mut s1 = slice("slice-a", vec![member("intent", "a")]);
    s1.project = Some("my-app".to_string());
    let mut s2 = slice("slice-b", vec![member("intent", "b")]);
    s2.project = Some("my-app".to_string());
    s2.depends_on = vec!["slice-a".to_string()];
    plan.propose_from(response(vec![s1, s2]), &doc, &topo).expect("propose ok");
    assert_eq!(plan.entries[1].depends_on, vec!["slice-a"]);

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![Platform::Android],
    }];

    plan.reconcile_platforms(&missing).expect("reconcile ok");

    assert_eq!(plan.entries[2].name, "slice-b");
    assert!(plan.entries[2].depends_on.contains(&SliceName::new("slice-a")));
    assert!(plan.entries[2].depends_on.contains(&SliceName::new("bootstrap-android")));
}

#[test]
fn reconcile_rejects_name_collision() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery_with(&[("intent", "app-foundation")]);
    let topo = vec![project("my-app", "vectis@1.0.0", "Crux app.")];
    let resp = response(vec![slice("app-foundation", vec![member("intent", "app-foundation")])]);
    plan.propose_from(resp, &doc, &topo).expect("propose ok");

    let missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing: vec![Platform::Core, Platform::Ios, Platform::Android],
    }];

    match plan.reconcile_platforms(&missing) {
        Err(Error::Validation { code, .. }) => {
            assert_eq!(code, "plan-reconcile-bootstrap-name-collision");
        }
        other => panic!("expected plan-reconcile-bootstrap-name-collision, got {other:?}"),
    }
}

#[test]
fn reconcile_multi_project_bootstraps() {
    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    let doc = discovery_with(&[("intent", "feat-a"), ("intent", "feat-b")]);
    let topo = vec![
        project("app-one", "vectis@1.0.0", "First Crux app."),
        project("app-two", "vectis@1.0.0", "Second Crux app."),
    ];
    let mut s1 = slice("feat-a", vec![member("intent", "feat-a")]);
    s1.project = Some("app-one".to_string());
    let mut s2 = slice("feat-b", vec![member("intent", "feat-b")]);
    s2.project = Some("app-two".to_string());
    plan.propose_from(response(vec![s1, s2]), &doc, &topo).expect("propose ok");

    let missing = vec![
        ProjectMissingPlatforms {
            project: "app-one".to_string(),
            missing: vec![Platform::Ios],
        },
        ProjectMissingPlatforms {
            project: "app-two".to_string(),
            missing: vec![Platform::Android],
        },
    ];

    let names = plan.reconcile_platforms(&missing).expect("reconcile succeeds");
    assert_eq!(names, vec!["app-one-bootstrap-ios", "app-two-bootstrap-android"]);

    // feat-a is bound to app-one: should depend only on its own bootstrap.
    let feat_a = plan.entries.iter().find(|e| e.name == "feat-a").unwrap();
    assert_eq!(feat_a.depends_on, vec!["app-one-bootstrap-ios"]);
    assert!(!feat_a.depends_on.contains(&SliceName::new("app-two-bootstrap-android")));

    // feat-b is bound to app-two: should depend only on its own bootstrap.
    let feat_b = plan.entries.iter().find(|e| e.name == "feat-b").unwrap();
    assert_eq!(feat_b.depends_on, vec!["app-two-bootstrap-android"]);
    assert!(!feat_b.depends_on.contains(&SliceName::new("app-one-bootstrap-ios")));
}

// --- shell presence detection (via specify-vectis-shell-detect) --------

fn missing_platforms(project_dir: &Path, platforms: &[Platform]) -> Vec<Platform> {
    let names: Vec<String> = platforms.iter().map(ToString::to_string).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    specify_vectis_shell_detect::missing_shell_platforms(project_dir, &refs)
        .into_iter()
        .filter_map(|s| s.parse().ok())
        .collect()
}

#[test]
fn detect_missing_no_shells() {
    let dir = tempfile::tempdir().expect("tempdir");
    let platforms = vec![Platform::Core, Platform::Ios, Platform::Android];
    let missing = missing_platforms(dir.path(), &platforms);
    assert_eq!(missing, vec![Platform::Core, Platform::Ios, Platform::Android]);
}

#[test]
fn detect_missing_core_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shared = dir.path().join("shared/src");
    std::fs::create_dir_all(&shared).expect("mkdir");
    std::fs::write(shared.join("app.rs"), "fn main() {}").expect("write");

    let platforms = vec![Platform::Core, Platform::Ios, Platform::Android];
    let missing = missing_platforms(dir.path(), &platforms);
    assert_eq!(missing, vec![Platform::Ios, Platform::Android]);
}

#[test]
fn detect_missing_all_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shared = dir.path().join("shared/src");
    std::fs::create_dir_all(&shared).expect("mkdir");
    std::fs::write(shared.join("app.rs"), "fn main() {}").expect("write");

    let ios = dir.path().join("iOS");
    std::fs::create_dir_all(&ios).expect("mkdir");
    std::fs::write(ios.join("App.swift"), "import SwiftUI").expect("write");

    let android = dir.path().join("Android");
    std::fs::create_dir_all(&android).expect("mkdir");
    std::fs::write(android.join("App.kt"), "package com.example").expect("write");

    let platforms = vec![Platform::Core, Platform::Ios, Platform::Android];
    let missing = missing_platforms(dir.path(), &platforms);
    assert!(missing.is_empty());
}

#[test]
fn detect_missing_skips_web_desktop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let platforms = vec![Platform::Core, Platform::Web, Platform::Desktop];
    let missing = missing_platforms(dir.path(), &platforms);
    assert_eq!(missing, vec![Platform::Core]);
}

#[test]
fn reconcile_never_bootstraps_web_desktop() {
    // Guard: `web` / `desktop` are type-system placeholders with no
    // shell interpretation, so the detect → reconcile pipeline must
    // never insert a bootstrap slice for them — even when they are the
    // only declared non-core platforms and no shell tree exists.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("shared/src")).expect("mkdir");
    std::fs::write(dir.path().join("shared/src/app.rs"), "// core present").expect("write");

    let platforms = vec![Platform::Core, Platform::Web, Platform::Desktop];
    let missing = missing_platforms(dir.path(), &platforms);
    assert!(missing.is_empty(), "web/desktop must never be detected as missing");

    let mut plan = plan_with_sources(Lifecycle::Pending, &["intent"]);
    propose_single_slice(&mut plan);
    let project_missing = vec![ProjectMissingPlatforms {
        project: "my-app".to_string(),
        missing,
    }];
    let names = plan.reconcile_platforms(&project_missing).expect("reconcile succeeds");
    assert!(names.is_empty(), "no bootstrap slice may be inserted for web/desktop");
    assert_eq!(plan.entries.len(), 1);
    assert!(plan.entries[0].depends_on.is_empty());
}
