use super::*;

/// Verbatim §The Plan reference fixture, post-2.0 collapse.
/// All entries use the simplified per-entry `Status` enum
/// (`pending | in-progress | done`); v1 has no per-entry
/// `blocked`, `failed`, or `skipped` state.
const PLAN_EXAMPLE_YAML: &str = r"name: platform-v2
sources:
  monolith:
    adapter: demo-source
    path: /path/to/legacy-codebase
  orders:
    adapter: demo-source
    path: git@github.com:org/orders-service.git
  payments:
    adapter: demo-source
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: demo-source
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

fn entry(name: &str, status: Status) -> Entry {
    Entry {
        name: name.into(),
        project: Some("default".into()),
        status,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: SliceAuthorityOverride::default(),
    }
}

/// Every parse / serialize / round-trip / default case for `Plan`,
/// `Entry`, the source-binding shapes, `TargetRef`, and `EntryPatch`.
/// Each block is one of the former single-purpose serde tests; the
/// inputs are exhaustive so coverage is identical to the split form.
#[expect(clippy::too_many_lines, reason = "collapsed serde matrix: one block per former test")]
#[test]
fn serde_shapes() {
    // The §The Plan reference fixture round-trips and parses every entry.
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

    // `lifecycle: approved` round-trips kebab-case.
    let plan: Plan =
        serde_saphyr::from_str("name: foo\nlifecycle: approved\nslices: []\n").expect("parse");
    assert_eq!(plan.lifecycle, Lifecycle::Approved);
    assert!(
        serde_saphyr::to_string(&plan).expect("serialize").contains("lifecycle: approved"),
        "serialised plan must carry kebab-case lifecycle: approved"
    );

    // A constructed plan serialises kebab-case keys and enum values.
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
    assert!(yaml.contains("status: in-progress"), "expected kebab-case in-progress in:\n{yaml}");
    assert!(!yaml.contains("depends_on"), "snake_case depends_on leaked into output:\n{yaml}");

    // Omitted optionals default: lifecycle pending, empty maps.
    let plan: Plan = serde_saphyr::from_str("name: foo\nslices: []\n").expect("parse minimal plan");
    assert_eq!(plan.name, "foo");
    assert_eq!(plan.lifecycle, Lifecycle::Pending);
    assert!(plan.sources.is_empty(), "sources should default to empty map");
    assert!(plan.entries.is_empty(), "slices should be empty");

    // An entry without `project` parses to `None`.
    let parsed: Entry =
        serde_saphyr::from_str("name: foo\nstatus: pending\n").expect("parses without project");
    assert_eq!(parsed.project, None);

    // The bound `project` survives a YAML round-trip.
    let plan: Plan = serde_saphyr::from_str(
        "name: test\nslices:\n  - name: define-contracts\n    project: identity-contracts\n    status: pending\n  - name: impl-auth\n    project: auth-service\n    status: pending\n",
    )
    .expect("parse");
    assert_eq!(plan.entries[0].project.as_deref(), Some("identity-contracts"));
    assert_eq!(plan.entries[1].project.as_deref(), Some("auth-service"));
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert_eq!(serde_saphyr::from_str::<Plan>(&rendered).expect("reparse"), plan);
    assert!(
        rendered.contains("project: identity-contracts")
            && rendered.contains("project: auth-service"),
        "the bound project must serialise back, got:\n{rendered}",
    );

    // `context` round-trips when populated and is omitted when empty.
    let plan: Plan = serde_saphyr::from_str(
        "\nname: ctx-test\nslices:\n  - name: with-ctx\n    project: default\n    status: pending\n    context:\n      - contracts/http/user-api.yaml\n      - specs/user-registration/spec.md\n  - name: without-ctx\n    project: default\n    status: pending\n",
    )
    .expect("parse yaml");
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

    // A default `EntryPatch` keeps everything and omits status.
    let patch = EntryPatch::default();
    assert!(patch.depends_on.is_none());
    assert!(patch.sources.is_none());
    assert_eq!(patch.project, Patch::Keep);
    assert_eq!(patch.description, Patch::Keep);
    assert!(patch.context.is_none());

    // Both source-binding shapes (bare + structured) round-trip.
    let plan: Plan = serde_saphyr::from_str(
        "\nname: bindings\nslices:\n  - name: bare-binding\n    project: app\n    sources: [demo-source]\n    status: pending\n  - name: combined\n    project: app\n    sources:\n      - source: docs\n        lead: account-pwd-reset\n      - source: legacy\n        lead: account-pwd-reset\n    status: pending\n",
    )
    .expect("parse");
    let bare = &plan.entries[0].sources[0];
    assert!(bare.lead.is_none(), "expected bare shorthand, got {bare:?}");
    assert_eq!(bare.source(), "demo-source");
    let structured = &plan.entries[1].sources[0];
    assert!(structured.lead.is_some(), "expected structured form, got {structured:?}");
    assert_eq!(structured.source(), "docs");
    assert_eq!(structured.lead("ignored-slice-name"), "account-pwd-reset");
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert_eq!(serde_saphyr::from_str::<Plan>(&rendered).expect("reparse"), plan);

    // The binding constructors normalise the shorthand vs structured forms.
    let bare = SliceSourceBinding::bare("demo-source");
    assert_eq!(bare.source(), "demo-source");
    assert_eq!(bare.lead("add-search-filter"), "add-search-filter");
    assert!(bare.lead.is_none());
    let structured = SliceSourceBinding::structured("docs", "user-reg");
    assert_eq!(structured.source(), "docs");
    assert_eq!(structured.lead("ignored-slice-name"), "user-reg");
    assert!(structured.lead.is_some());

    // An optional `sources.<key>.version` pin round-trips.
    let plan: Plan = serde_saphyr::from_str(
        "name: pinned\nsources:\n  pinned-src:\n    adapter: demo-source\n    version: 2.3.1\n    path: /repo\n  bare-src:\n    adapter: demo-source\n    value: do the thing\nslices: []\n",
    )
    .expect("parse");
    assert_eq!(plan.sources["pinned-src"].version, Some(semver::Version::new(2, 3, 1)));
    assert_eq!(plan.sources["bare-src"].version, None, "an omitted version stays None");
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert_eq!(serde_saphyr::from_str::<Plan>(&rendered).expect("reparse"), plan);
    assert_eq!(
        rendered.matches("version:").count(),
        1,
        "only the pinned binding emits a `version:` key, got:\n{rendered}"
    );

    // The `SourceBinding` builders normalise the path- vs value-bound shapes
    // (each is mutually exclusive and leaves `version` unset).
    let path_bound = SourceBinding::path("typescript", "/repo/legacy");
    assert_eq!(path_bound.adapter, "typescript");
    assert_eq!(path_bound.path.as_deref(), Some("/repo/legacy"));
    assert!(path_bound.value.is_none() && path_bound.version.is_none());
    let value_bound = SourceBinding::value("intent", "do the thing");
    assert_eq!(value_bound.adapter, "intent");
    assert_eq!(value_bound.value.as_deref(), Some("do the thing"));
    assert!(value_bound.path.is_none() && value_bound.version.is_none());

    // `name@<semver>` parses; the `name@vN` form is rejected.
    assert_eq!(
        TargetRef::parse("demo-target@1.0.0").expect("semver target").to_string(),
        "demo-target@1.0.0"
    );
    TargetRef::parse("demo-target@v1").expect_err("the legacy @vN form must be rejected");

    // The `authority-override` map round-trips and elides when empty.
    let plan: Plan = serde_saphyr::from_str(
        "name: synthesis-plan\nslices:\n  - name: identity-user-registration\n    project: identity-svc\n    status: pending\n    sources:\n      - source: runtime\n        lead: user-registration\n      - source: legacy-monolith\n        lead: user-registration\n    authority-override:\n      requirement: runtime\n      criterion: legacy-monolith\n",
    )
    .expect("parse");
    let ov = &plan.entries[0].authority_override;
    assert_eq!(ov.by_kind.get(&ClaimKind::Requirement).map(String::as_str), Some("runtime"));
    assert_eq!(ov.by_kind.get(&ClaimKind::Criterion).map(String::as_str), Some("legacy-monolith"));
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(rendered.contains("authority-override:"));
    assert!(rendered.contains("requirement: runtime"));
    assert_eq!(serde_saphyr::from_str::<Plan>(&rendered).expect("reparse"), plan);
    let plan: Plan = serde_saphyr::from_str(
        "name: tiny\nslices:\n  - name: x\n    project: app\n    status: pending\n",
    )
    .expect("parse");
    assert!(plan.entries[0].authority_override.by_kind.is_empty());
    assert!(
        !serde_saphyr::to_string(&plan).expect("serialize").contains("authority-override"),
        "empty override map must elide on write"
    );

    // `divergence: likely` round-trips as one kebab-case line.
    let plan: Plan = serde_saphyr::from_str(
        "name: demo\nslices:\n  - name: checkout\n    project: default\n    status: pending\n    divergence: likely\n",
    )
    .expect("parse reference yaml");
    assert_eq!(plan.entries[0].divergence, Some(Divergence::Likely));
    let rendered = serde_saphyr::to_string(&plan).expect("serialize");
    assert!(rendered.contains("divergence: likely"), "got:\n{rendered}");
    assert_eq!(serde_saphyr::from_str::<Plan>(&rendered).expect("reparse"), plan);
}

/// `Plan::is_drained` / `is_executing` over the closed status matrix:
/// all-done is drained-not-executing, any-in-progress is
/// executing-not-drained, and an empty plan is vacuously drained.
#[test]
fn drained_and_executing() {
    let cases: &[(Vec<Status>, bool, bool)] = &[
        (vec![Status::Done, Status::Done], true, false),
        (vec![Status::Done, Status::InProgress], false, true),
        (vec![], true, false),
    ];
    for (statuses, drained, executing) in cases {
        let entries: Vec<Entry> = statuses
            .iter()
            .enumerate()
            .map(|(i, status)| entry(&format!("slice-{i}"), *status))
            .collect();
        let plan = Plan {
            name: "demo".into(),
            lifecycle: Lifecycle::Approved,
            sources: BTreeMap::new(),
            entries,
        };
        assert_eq!(plan.is_drained(), *drained, "is_drained for {statuses:?}");
        assert_eq!(plan.is_executing(), *executing, "is_executing for {statuses:?}");
    }
}

/// A2/A13: plan validation findings are built directly on the neutral
/// [`diagnostics::Diagnostic`] currency via `plan_finding`. The
/// stable check code becomes the `rule_id`, the offending entry is
/// carried as `slice`, the artifact is `Plan`, and the fingerprint
/// validates.
#[test]
fn plan_finding_builds_canonical_diagnostic() {
    let diagnostic = crate::change::plan::core::plan_finding(
        "plan.cycle",
        diagnostics::Severity::Important,
        "dependency cycle: a -> b -> a",
        Some("checkout".to_string()),
    );

    assert_eq!(diagnostic.rule_id.as_deref(), Some("plan.cycle"));
    assert_eq!(diagnostic.severity, diagnostics::Severity::Important);
    assert_eq!(diagnostic.slice.as_deref(), Some("checkout"));
    assert_eq!(diagnostic.artifact, diagnostics::Artifact::Plan);
    assert_eq!(diagnostic.impact, "dependency cycle: a -> b -> a");
    diagnostics::validate_diagnostic(&diagnostic).expect("plan finding is valid");
    assert!(diagnostics::verify_fingerprint(&diagnostic), "fingerprint covers slice");
}

/// A non-blocking `Suggestion` finding never gates per
/// [`diagnostics::blocking`].
#[test]
fn plan_finding_suggestion_is_non_blocking() {
    let diagnostic = crate::change::plan::core::plan_finding(
        "plan.orphan-source",
        diagnostics::Severity::Suggestion,
        "source `docs` is unreferenced",
        None,
    );
    assert_eq!(diagnostic.severity, diagnostics::Severity::Suggestion);
    assert!(diagnostic.slice.is_none());
    assert!(!diagnostics::blocking(&diagnostic));
}
