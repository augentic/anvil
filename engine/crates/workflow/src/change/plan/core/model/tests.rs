use super::*;

/// Verbatim §The Plan reference fixture, post-2.0 collapse.
/// All entries use the simplified per-entry `Status` enum
/// (`pending | in-progress | done`); v1 has no per-entry
/// `blocked`, `failed`, or `skipped` state.
const PLAN_EXAMPLE_YAML: &str = r"name: platform-v2
sources:
  monolith:
    adapter: typescript
    path: /path/to/legacy-codebase
  orders:
    adapter: typescript
    path: git@github.com:org/orders-service.git
  payments:
    adapter: typescript
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: typescript
    path: git@github.com:org/web-app.git
slices:
  - name: user-registration
    project: platform
    sources: [monolith]
    status: done
  - name: email-verification
    project: platform
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress
  - name: registration-duplicate-email-crash
    project: platform
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending
";

#[test]
fn round_trips_plan_fixture() {
    let original: Plan = serde_saphyr::from_str(PLAN_EXAMPLE_YAML).expect("parse plan fixture");
    let yaml = serde_saphyr::to_string(&original).expect("serialize plan");
    let reparsed: Plan = serde_saphyr::from_str(&yaml).expect("reparse plan");
    assert_eq!(original, reparsed, "plan should survive a serialize/parse round-trip");

    assert_eq!(original.name, "platform-v2");
    assert_eq!(original.sources.len(), 4);
    assert_eq!(original.entries.len(), 3);
    assert_eq!(original.entries[0].status, Status::Done);
    assert_eq!(original.entries[1].status, Status::InProgress);
    assert_eq!(original.entries[2].status, Status::Pending);
}

#[test]
fn lifecycle_round_trips() {
    let yaml = "name: foo\nlifecycle: approved\nslices: []\n";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse approved");
    assert_eq!(plan.lifecycle, Lifecycle::Approved);

    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(
        rendered.contains("lifecycle: approved"),
        "serialised plan must carry kebab-case lifecycle: approved, got:\n{rendered}"
    );
}

#[test]
fn serializes_kebab_case() {
    let plan = Plan {
        name: "demo".into(),
        lifecycle: Lifecycle::Pending,
        sources: BTreeMap::new(),
        entries: vec![Entry {
            name: "entry-one".into(),
            project: Some("default".into()),
            status: Status::InProgress,
            depends_on: vec!["entry-zero".into()],
            sources: vec![],
            context: vec![],
            description: None,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: SliceAuthorityOverride::default(),
        }],
    };
    let yaml = serde_saphyr::to_string(&plan).expect("serialize plan");
    assert!(yaml.contains("depends-on:"), "expected kebab-case depends-on in:\n{yaml}");
    assert!(
        yaml.contains("status: in-progress"),
        "expected kebab-case enum value in-progress in:\n{yaml}"
    );
    assert!(!yaml.contains("depends_on"), "snake_case depends_on leaked into output:\n{yaml}");
}

#[test]
fn missing_fields_default() {
    let yaml = "name: foo\nslices: []\n";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse minimal plan");
    assert_eq!(plan.name, "foo");
    assert_eq!(plan.lifecycle, Lifecycle::Pending);
    assert!(plan.sources.is_empty(), "sources should default to empty map");
    assert!(plan.entries.is_empty(), "slices should be empty");
}

#[test]
fn project_defaults_to_none() {
    let yaml = "\
name: foo
status: pending
";
    let parsed: Entry = serde_saphyr::from_str(yaml).expect("parses without project");
    assert_eq!(parsed.project, None);
}

#[test]
fn project_only_round_trips() {
    // The target adapter is not stored on the slice; a slice
    // carries only its `project` (or omits it to resolve the sole
    // topology project). The bound project survives a YAML round-trip.
    let yaml = r"name: test
slices:
  - name: define-contracts
    project: identity-contracts
    status: pending
  - name: impl-auth
    project: auth-service
    status: pending
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(plan.entries[0].project.as_deref(), Some("identity-contracts"));
    assert_eq!(plan.entries[1].project.as_deref(), Some("auth-service"));

    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    let reparsed: Plan = serde_saphyr::from_str(&rendered).expect("reparse");
    assert_eq!(plan, reparsed, "plan must survive a YAML round-trip");
    assert!(
        rendered.contains("project: identity-contracts")
            && rendered.contains("project: auth-service"),
        "the bound project must serialise back, got:\n{rendered}",
    );
}

#[test]
fn context_round_trips() {
    let yaml = r"
name: ctx-test
slices:
  - name: with-ctx
    project: default
    status: pending
    context:
      - contracts/http/user-api.yaml
      - specs/user-registration/spec.md
  - name: without-ctx
    project: default
    status: pending
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse yaml");
    assert_eq!(
        plan.entries[0].context,
        vec!["contracts/http/user-api.yaml", "specs/user-registration/spec.md"],
    );
    assert!(plan.entries[1].context.is_empty(), "missing context defaults to empty");

    let serialized = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(
        serialized.contains("contracts/http/user-api.yaml"),
        "populated context must appear in serialized output"
    );
    assert!(
        !serialized.contains("without-ctx")
            || !serialized.split("without-ctx").nth(1).unwrap_or("").contains("context"),
        "empty context must be omitted from serialized output"
    );
}

#[test]
fn patch_omits_status() {
    let patch = EntryPatch::default();
    assert!(patch.depends_on.is_none());
    assert!(patch.sources.is_none());
    assert_eq!(patch.project, Patch::Keep);
    assert_eq!(patch.description, Patch::Keep);
    assert!(patch.context.is_none());
}

#[test]
fn binding_round_trips_both_shapes() {
    let yaml = r"
name: bindings
slices:
  - name: pure-intent
    project: app
    sources: [intent]
    status: pending
  - name: combined
    project: app
    sources:
      - source: docs
        lead: account-pwd-reset
      - source: legacy
        lead: account-pwd-reset
    status: pending
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse");
    let bare = &plan.entries[0].sources[0];
    assert!(bare.lead.is_none(), "expected bare shorthand, got {bare:?}");
    assert_eq!(bare.source(), "intent");
    let structured = &plan.entries[1].sources[0];
    assert!(structured.lead.is_some(), "expected structured form, got {structured:?}");
    assert_eq!(structured.source(), "docs");
    assert_eq!(structured.lead("ignored-slice-name"), "account-pwd-reset");
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    let reparsed: Plan = serde_saphyr::from_str(&rendered).expect("reparse");
    assert_eq!(plan, reparsed, "both binding shapes must survive a round-trip");
}

#[test]
fn binding_normalises_shorthand() {
    let bare = SliceSourceBinding::bare("intent");
    assert_eq!(bare.source(), "intent");
    assert_eq!(bare.lead("add-search-filter"), "add-search-filter");
    assert!(bare.lead.is_none());

    let structured = SliceSourceBinding::structured("docs", "user-reg");
    assert_eq!(structured.source(), "docs");
    assert_eq!(structured.lead("ignored-slice-name"), "user-reg");
    assert!(structured.lead.is_some());
}

#[test]
fn source_binding_version_round_trips() {
    // RFC-47 D2: a `sources.<key>.version` semver pin survives a
    // parse → serialize → reparse round-trip, while a binding that
    // omits `version` stays `None` (existing plans parse unchanged).
    let yaml = r"name: pinned
sources:
  pinned-src:
    adapter: typescript
    version: 2.3.1
    path: /repo
  bare-src:
    adapter: intent
    value: do the thing
slices: []
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(plan.sources["pinned-src"].version, Some(semver::Version::new(2, 3, 1)));
    assert_eq!(plan.sources["bare-src"].version, None, "an omitted version stays None");

    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    let reparsed: Plan = serde_saphyr::from_str(&rendered).expect("reparse");
    assert_eq!(plan, reparsed, "the optional version pin must survive a round-trip");
    assert_eq!(
        rendered.matches("version:").count(),
        1,
        "only the pinned binding emits a `version:` key, got:\n{rendered}"
    );
}

#[test]
fn target_ref_parses_semver() {
    // RFC-47 D1: a `name@<semver>` target parses and renders verbatim;
    // the legacy `name@vN` git-ref form is no longer a valid target.
    let parsed = TargetRef::parse("omnia@1.0.0").expect("semver target parses");
    assert_eq!(parsed.to_string(), "omnia@1.0.0");
    TargetRef::parse("omnia@v1").expect_err("the legacy @vN form must be rejected");
}

#[test]
fn is_drained_only_when_all_done() {
    let plan = Plan {
        name: "demo".into(),
        lifecycle: Lifecycle::Approved,
        sources: BTreeMap::new(),
        entries: vec![
            Entry {
                name: "a".into(),
                project: Some("default".into()),
                status: Status::Done,
                depends_on: vec![],
                sources: vec![],
                context: vec![],
                description: None,
                divergence: None,
                disagreements: Vec::new(),
                authority_override: SliceAuthorityOverride::default(),
            },
            Entry {
                name: "b".into(),
                project: Some("default".into()),
                status: Status::Done,
                depends_on: vec![],
                sources: vec![],
                context: vec![],
                description: None,
                divergence: None,
                disagreements: Vec::new(),
                authority_override: SliceAuthorityOverride::default(),
            },
        ],
    };
    assert!(plan.is_drained(), "all-done plan must report drained");
    assert!(!plan.is_executing(), "no in-progress entry => not executing");
}

#[test]
fn is_executing_when_any_in_progress() {
    let plan = Plan {
        name: "demo".into(),
        lifecycle: Lifecycle::Approved,
        sources: BTreeMap::new(),
        entries: vec![
            Entry {
                name: "a".into(),
                project: Some("default".into()),
                status: Status::Done,
                depends_on: vec![],
                sources: vec![],
                context: vec![],
                description: None,
                divergence: None,
                disagreements: Vec::new(),
                authority_override: SliceAuthorityOverride::default(),
            },
            Entry {
                name: "b".into(),
                project: Some("default".into()),
                status: Status::InProgress,
                depends_on: vec![],
                sources: vec![],
                context: vec![],
                description: None,
                divergence: None,
                disagreements: Vec::new(),
                authority_override: SliceAuthorityOverride::default(),
            },
        ],
    };
    assert!(plan.is_executing(), "any in-progress => executing");
    assert!(!plan.is_drained(), "in-progress entry => not drained");
}

#[test]
fn authority_override_round_trips() {
    let yaml = r"name: synthesis-plan
slices:
  - name: identity-user-registration
    project: identity-svc
    status: pending
    sources:
      - source: runtime
        lead: user-registration
      - source: legacy-monolith
        lead: user-registration
    authority-override:
      requirement: runtime
      criterion: legacy-monolith
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse");
    let entry = &plan.entries[0];
    assert_eq!(
        entry.authority_override.by_kind.get(&ClaimKind::Requirement).map(String::as_str),
        Some("runtime")
    );
    assert_eq!(
        entry.authority_override.by_kind.get(&ClaimKind::Criterion).map(String::as_str),
        Some("legacy-monolith")
    );

    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(rendered.contains("authority-override:"));
    assert!(rendered.contains("requirement: runtime"));
    let reparsed: Plan = serde_saphyr::from_str(&rendered).expect("reparse");
    assert_eq!(plan, reparsed);
}

#[test]
fn empty_authority_override_elides() {
    let yaml = r"name: tiny
slices:
  - name: x
    project: app
    status: pending
";
    let plan: Plan = serde_saphyr::from_str(yaml).expect("parse");
    assert!(plan.entries[0].authority_override.by_kind.is_empty());
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(
        !rendered.contains("authority-override"),
        "empty override map must elide on write, got:\n{rendered}"
    );
}

#[test]
fn divergence_likely_round_trips_yaml() {
    // divergence and writer-ownership contract: the CLI is the single writer of every variant
    // of `slices[].divergence`. The on-disk shape for `Likely`
    // is one kebab-case line on the slice entry, byte-identical
    // to the legacy skill-written output we are retiring.
    let reference = r"name: demo
slices:
  - name: checkout
    project: default
    status: pending
    divergence: likely
";
    let plan: Plan = serde_saphyr::from_str(reference).expect("parse reference yaml");
    assert_eq!(plan.entries[0].divergence, Some(Divergence::Likely));
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(
        rendered.contains("divergence: likely"),
        "Divergence::Likely must serialise as kebab-case `divergence: likely`, got:\n{rendered}"
    );
    let reparsed: Plan = serde_saphyr::from_str(&rendered).expect("reparse");
    assert_eq!(plan, reparsed, "plan with divergence: likely must round-trip");
}

#[test]
fn empty_plan_is_drained_vacuously() {
    let plan = Plan {
        name: "demo".into(),
        lifecycle: Lifecycle::Pending,
        sources: BTreeMap::new(),
        entries: vec![],
    };
    assert!(plan.is_drained(), "empty plan reports drained vacuously");
    assert!(!plan.is_executing(), "empty plan is not executing");
}

/// A2/A13: plan validation findings are built directly on the neutral
/// [`specify_diagnostics::Diagnostic`] currency via `plan_finding`. The
/// stable check code becomes the `rule_id`, the offending entry is
/// carried as `slice`, the artifact is `Plan`, and the fingerprint
/// validates.
#[test]
fn plan_finding_builds_canonical_diagnostic() {
    let diagnostic = crate::change::plan::core::plan_finding(
        "plan.cycle",
        specify_diagnostics::Severity::Important,
        "dependency cycle: a -> b -> a",
        Some("checkout".to_string()),
    );

    assert_eq!(diagnostic.rule_id.as_deref(), Some("plan.cycle"));
    assert_eq!(diagnostic.severity, specify_diagnostics::Severity::Important);
    assert_eq!(diagnostic.slice.as_deref(), Some("checkout"));
    assert_eq!(diagnostic.artifact, specify_diagnostics::Artifact::Plan);
    assert_eq!(diagnostic.impact, "dependency cycle: a -> b -> a");
    specify_diagnostics::validate_diagnostic(&diagnostic).expect("plan finding is valid");
    assert!(specify_diagnostics::verify_fingerprint(&diagnostic), "fingerprint covers slice");
}

/// A non-blocking `Suggestion` finding never gates per
/// [`specify_diagnostics::blocking`].
#[test]
fn plan_finding_suggestion_is_non_blocking() {
    let diagnostic = crate::change::plan::core::plan_finding(
        "plan.orphan-source",
        specify_diagnostics::Severity::Suggestion,
        "source `docs` is unreferenced",
        None,
    );
    assert_eq!(diagnostic.severity, specify_diagnostics::Severity::Suggestion);
    assert!(diagnostic.slice.is_none());
    assert!(!specify_diagnostics::blocking(&diagnostic));
}
