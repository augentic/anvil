//! RFC-86 S16: typed gap inventory + shared-lead presentation rollup
//! (Gaps / D18 / D19 / D24) and the RFC-86a D2 disposition projection
//! (deferral facts joined under the `(slice, digest)` match key).

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{Event, EventKind};
use project::plan::{
    DebtCounts, Deferral, Disposition, Entry, GapRow, Plan, SharedLeadRollup, SliceSourceBinding,
    in_scope, plan_gaps_body,
};
use project::slice::{RequirementBody, SliceMetadata};
use tempfile::TempDir;

fn entry(name: &str, sources: Vec<SliceSourceBinding>) -> Entry {
    Entry {
        name: name.into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources,
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
        allow_composition_replace: false,
    }
}

fn plan(entries: Vec<Entry>) -> Plan {
    Plan {
        name: "test".into(),
        sources: BTreeMap::new(),
        entries,
    }
}

fn write_meta(slice_dir: &Path, dropped: bool) {
    std::fs::create_dir_all(slice_dir).expect("slice dir");
    let mut meta = String::from("target: demo@1.0.0\n");
    if dropped {
        meta.push_str("dropped-at: \"2024-01-01T00:00:00Z\"\n");
    }
    std::fs::write(slice_dir.join("metadata.yaml"), meta).expect("metadata");
}

fn write_model(slice_dir: &Path, body: &str) {
    std::fs::write(slice_dir.join("model.yaml"), body).expect("model.yaml");
}

/// Canonical digest of a title-only body — the shape the fixture
/// models in this suite carry (empty statement, no scenarios/notes).
fn title_digest(title: &str) -> String {
    RequirementBody {
        title,
        statement: "",
        scenarios: &[],
        notes: None,
    }
    .digest()
}

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + second).expect("valid timestamp")
}

fn deferred(second: i64, writer: &str, sequence: u64, slice: &str, digest: &str) -> Event {
    Event {
        timestamp: ts(second),
        writer: writer.into(),
        sequence,
        kind: EventKind::GapDeferred {
            slice: slice.into(),
            req: "REQ-000".into(),
            requirement_digest: digest.into(),
            reason: "deferred to next change".into(),
        },
    }
}

#[test]
fn multi_homed_lead() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");

    let staged = plan(vec![
        entry("auth-login", vec![SliceSourceBinding::structured("docs", "conventions")]),
        entry("payments", vec![SliceSourceBinding::structured("docs", "conventions")]),
    ]);

    let auth = root.join(".emery/change/slices/auth-login");
    write_meta(&auth, false);
    write_model(
        &auth,
        r#"requirements:
  - id: REQ-003
    title: password-reset path not evidenced
    statement: ''
    status: unknown
    sources: [docs]
  - id: REQ-007
    title: "session TTL: docs vs intent (tied)"
    statement: ''
    status: conflict
    sources: [docs, intent]
"#,
    );

    let payments = root.join(".emery/change/slices/payments");
    write_meta(&payments, false);
    write_model(
        &payments,
        r#"requirements:
  - id: REQ-008
    title: reset copy not evidenced
    statement: ''
    status: unknown
    sources: [docs]
  - id: REQ-012
    title: "retry budget: docs beat behaviour"
    statement: ''
    status: divergence
    sources: [docs]
  - id: REQ-001
    title: agreed checkout path
    statement: ''
    status: agreed
    sources: [docs]
"#,
    );

    let body = plan_gaps_body(&staged, Layout::new(root), &[]).expect("gaps");
    assert_eq!(body.plan, "test");
    assert_eq!(body.rows.len(), 4, "agreed excluded; four typed gaps: {body:?}");

    assert_eq!(
        body.rows[0],
        GapRow {
            slice: "auth-login".into(),
            req: "REQ-003".into(),
            status: RequirementStatus::Unknown,
            summary: "password-reset path not evidenced".into(),
            requirement_digest: Some(title_digest("password-reset path not evidenced")),
            disposition: Some(Disposition::Open),
            deferral: None,
            shared_lead: Some("docs:conventions".into()),
        }
    );
    assert_eq!(body.rows[1].req, "REQ-007");
    assert_eq!(body.rows[1].status, RequirementStatus::Conflict);
    assert_eq!(body.rows[1].disposition, Some(Disposition::Open));
    // Conflict contributes docs+intent; docs:conventions is multi-homed
    // across unknowns too, so the shared-lead annotation still applies.
    assert_eq!(body.rows[1].shared_lead.as_deref(), Some("docs:conventions"));

    assert_eq!(
        body.rows[2],
        GapRow {
            slice: "payments".into(),
            req: "REQ-008".into(),
            status: RequirementStatus::Unknown,
            summary: "reset copy not evidenced".into(),
            requirement_digest: Some(title_digest("reset copy not evidenced")),
            disposition: Some(Disposition::Open),
            deferral: None,
            shared_lead: Some("docs:conventions".into()),
        }
    );
    assert_eq!(body.rows[3].req, "REQ-012");
    assert_eq!(body.rows[3].status, RequirementStatus::Divergence);
    assert_eq!(body.rows[3].disposition, None, "[divergence] takes no disposition");

    assert_eq!(
        body.rollups,
        vec![SharedLeadRollup {
            source: "docs".into(),
            lead: "conventions".into(),
            selectors: vec!["auth-login".into(), "payments".into()],
        }]
    );
}

#[test]
fn dropped_slice() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");

    let staged = plan(vec![
        entry("live", vec![SliceSourceBinding::structured("docs", "conventions")]),
        entry("abandoned", vec![SliceSourceBinding::structured("docs", "conventions")]),
    ]);

    let live = root.join(".emery/change/slices/live");
    write_meta(&live, false);
    write_model(
        &live,
        r"requirements:
  - id: REQ-001
    title: thin path
    statement: ''
    status: unknown
    sources: [docs]
",
    );

    let abandoned = root.join(".emery/change/slices/abandoned");
    write_meta(&abandoned, true);
    write_model(
        &abandoned,
        r"requirements:
  - id: REQ-009
    title: also thin
    statement: ''
    status: unknown
    sources: [docs]
",
    );

    let meta = SliceMetadata::load(&abandoned).expect("load dropped meta");
    assert!(!in_scope(&staged, &staged.entries[1], Some(&meta)));

    let body = plan_gaps_body(&staged, Layout::new(root), &[]).expect("gaps");
    assert_eq!(body.rows.len(), 1);
    assert_eq!(body.rows[0].slice, "live");
    assert_eq!(body.rows[0].req, "REQ-001");
    // Dropped sibling removed the multi-home — no shared-lead rollup.
    assert!(body.rows[0].shared_lead.is_none());
    assert!(body.rollups.is_empty());
}

#[test]
fn unrefined_scope_slice() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");

    let staged = plan(vec![entry("pending-work", vec![])]);
    write_meta(&root.join(".emery/change/slices/pending-work"), false);

    let body = plan_gaps_body(&staged, Layout::new(root), &[]).expect("gaps");
    assert!(body.is_empty());
}

/// Fixture for the disposition tests: one slice (`auth-login`) whose
/// model carries an `[unknown]`, a `[conflict]`, and a `[divergence]`
/// row.
fn disposition_fixture(root: &Path) -> Plan {
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");
    let staged = plan(vec![entry("auth-login", vec![])]);
    let slice_dir = root.join(".emery/change/slices/auth-login");
    write_meta(&slice_dir, false);
    write_model(
        &slice_dir,
        r"requirements:
  - id: REQ-001
    title: reset path not evidenced
    statement: ''
    status: unknown
  - id: REQ-002
    title: session TTL tied
    statement: ''
    status: conflict
  - id: REQ-003
    title: retry budget divergence
    statement: ''
    status: divergence
",
    );
    staged
}

fn dispositions(body: &project::plan::GapsBody) -> Vec<(&str, Option<Disposition>)> {
    body.rows.iter().map(|row| (row.req.as_str(), row.disposition)).collect()
}

#[test]
fn covers_gap_rows() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);

    let events = vec![
        deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced")),
        deferred(2, "local", 2, "auth-login", &title_digest("session TTL tied")),
        // A fact naming the divergence row's digest must not
        // disposition it — divergence takes none.
        deferred(3, "local", 3, "auth-login", &title_digest("retry budget divergence")),
    ];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        dispositions(&body),
        vec![
            ("REQ-001", Some(Disposition::Deferred)),
            ("REQ-002", Some(Disposition::Deferred)),
            ("REQ-003", None),
        ]
    );
}

#[test]
fn digest_lapse_revive() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);
    let slice_dir = root.join(".emery/change/slices/auth-login");

    // Deferral minted against the original body.
    let events =
        vec![deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced"))];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred));

    // Re-refine reshapes the body — the digest disappears from the
    // live model, so the fact lapses and the new row is open again.
    write_model(
        &slice_dir,
        r"requirements:
  - id: REQ-001
    title: reset path reshaped by new evidence
    statement: ''
    status: unknown
",
    );
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Open), "lapsed on digest change");

    // A later refine restores the exact body (under a renumbered id):
    // liveness is recomputed from the union, so the old fact revives
    // the disposition without re-assertion.
    write_model(
        &slice_dir,
        r"requirements:
  - id: REQ-009
    title: reset path not evidenced
    statement: ''
    status: unknown
",
    );
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].req, "REQ-009");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred), "exact body revives");
}

#[test]
fn dup_facts_idempotent() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);
    let digest = title_digest("reset path not evidenced");

    let events = vec![
        deferred(1, "alpha", 1, "auth-login", &digest),
        deferred(2, "bravo", 1, "auth-login", &digest),
        deferred(3, "alpha", 2, "auth-login", &digest),
    ];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows.len(), 3, "duplicate facts mint no extra rows");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred));
}

#[test]
fn two_writer_one_disp() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);
    let digest = title_digest("reset path not evidenced");
    let with_reason = |writer: &str, reason: &str| Event {
        timestamp: ts(5),
        writer: writer.into(),
        sequence: 1,
        kind: EventKind::GapDeferred {
            slice: "auth-login".into(),
            req: "REQ-000".into(),
            requirement_digest: digest.clone(),
            reason: reason.into(),
        },
    };

    // Same timestamp: `(timestamp, writer, sequence)` breaks the tie,
    // so bravo's fact is the latest and its reason supersedes.
    let events = vec![with_reason("alpha", "first"), with_reason("bravo", "second")];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred));
    assert_eq!(body.rows[0].deferral.as_ref().expect("covering fact").reason, "second");

    // Union order of the input slice does not matter — the fold keys
    // on the envelope, not the position.
    let events = vec![with_reason("bravo", "second"), with_reason("alpha", "first")];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        body.rows[0].deferral.as_ref().expect("covering fact").reason,
        "second",
        "bravo sorts last"
    );
}

fn render(body: &project::plan::GapsBody) -> String {
    let mut out = Vec::new();
    project::handler::Render::render(body, &mut out).expect("render");
    String::from_utf8(out).expect("utf8")
}

#[test]
fn rows_carry_reason() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);

    let events =
        vec![deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced"))];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        body.rows[0].deferral,
        Some(Deferral {
            reason: "deferred to next change".into(),
            deferred_at: ts(1),
        }),
        "deferred row carries the covering fact's reason and timestamp"
    );
    assert_eq!(body.rows[1].deferral, None, "open row carries no deferral detail");
    assert_eq!(body.rows[2].deferral, None, "divergence row carries no deferral detail");

    // Re-deferring supersedes: the latest fact's reason wins.
    let mut events = events;
    events.push(Event {
        timestamp: ts(2),
        writer: "local".into(),
        sequence: 2,
        kind: EventKind::GapDeferred {
            slice: "auth-login".into(),
            req: "REQ-001".into(),
            requirement_digest: title_digest("reset path not evidenced"),
            reason: "deferred at the build gate under epoch 2024".into(),
        },
    });
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        body.rows[0].deferral,
        Some(Deferral {
            reason: "deferred at the build gate under epoch 2024".into(),
            deferred_at: ts(2),
        })
    );
}

#[test]
fn render_disp_sections() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);

    let events = vec![
        deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced")),
        deferred(2, "local", 2, "auth-login", &title_digest("session TTL tied")),
    ];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    let text = render(&body);

    let header = text.lines().next().expect("header");
    assert!(header.contains("disposition"), "disposition column header: {text}");
    let unknown_row = text.lines().find(|l| l.contains("REQ-001")).expect("REQ-001 row");
    assert!(unknown_row.contains("deferred"), "deferred disposition cell: {unknown_row}");
    let divergence_row = text.lines().find(|l| l.contains("REQ-003")).expect("REQ-003 row");
    assert!(divergence_row.contains('—'), "divergence takes no disposition: {divergence_row}");

    // Deferred conflicts render separately from deferred unknowns
    // (D6), each row with the covering fact's reason.
    let unknowns_at = text.find("deferred unknowns:").expect("deferred unknowns section");
    let conflicts_at = text.find("deferred conflicts:").expect("deferred conflicts section");
    assert!(unknowns_at < conflicts_at, "unknowns before conflicts: {text}");
    let unknowns_section = &text[unknowns_at..conflicts_at];
    assert!(unknowns_section.contains("auth-login/REQ-001"), "{text}");
    assert!(unknowns_section.contains("deferred to next change"), "{text}");
    let conflicts_section = &text[conflicts_at..];
    assert!(conflicts_section.contains("auth-login/REQ-002"), "{text}");
    assert!(!conflicts_section.contains("REQ-001"), "conflict section carries conflicts only");

    // An all-open inventory renders no deferred sections.
    let body = plan_gaps_body(&staged, Layout::new(root), &[]).expect("gaps");
    let text = render(&body);
    assert!(!text.contains("deferred unknowns:"), "{text}");
    assert!(!text.contains("deferred conflicts:"), "{text}");
}

#[test]
fn rollup_both_disps() {
    // D19 stays presentation-only over open and deferred rows alike:
    // deferring one of two multi-homed findings keeps the shared-lead
    // annotation and the rollup selectors intact.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");
    let staged = plan(vec![
        entry("auth-login", vec![SliceSourceBinding::structured("docs", "conventions")]),
        entry("payments", vec![SliceSourceBinding::structured("docs", "conventions")]),
    ]);
    for (slice, title) in
        [("auth-login", "reset path not evidenced"), ("payments", "reset copy not evidenced")]
    {
        let dir = root.join(".emery/change/slices").join(slice);
        write_meta(&dir, false);
        write_model(
            &dir,
            &format!(
                "requirements:\n  - id: REQ-001\n    title: {title}\n    statement: ''\n    status: unknown\n    sources: [docs]\n"
            ),
        );
    }

    let events =
        vec![deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced"))];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred));
    assert_eq!(body.rows[0].shared_lead.as_deref(), Some("docs:conventions"));
    assert_eq!(body.rows[1].disposition, Some(Disposition::Open));
    assert_eq!(
        body.rollups,
        vec![SharedLeadRollup {
            source: "docs".into(),
            lead: "conventions".into(),
            selectors: vec!["auth-login".into(), "payments".into()],
        }]
    );
}

#[test]
fn debt_counts_conflict() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let staged = disposition_fixture(root);

    let events = vec![
        deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced")),
        deferred(2, "local", 2, "auth-login", &title_digest("session TTL tied")),
    ];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        dispositions(&body),
        vec![
            ("REQ-001", Some(Disposition::Deferred)),
            ("REQ-002", Some(Disposition::Deferred)),
            ("REQ-003", None),
        ],
        "fully dispositioned — divergence takes no disposition"
    );
    let debt = body.debt();
    assert_eq!(
        debt,
        DebtCounts {
            unknown: 1,
            conflict: 1,
        }
    );
    assert_eq!(debt.to_string(), "2 deferred gaps (1 unknown, 1 conflict)");

    // Only the conflict deferred: the unknown stays open.
    let events = vec![deferred(2, "local", 2, "auth-login", &title_digest("session TTL tied"))];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(
        dispositions(&body),
        vec![
            ("REQ-001", Some(Disposition::Open)),
            ("REQ-002", Some(Disposition::Deferred)),
            ("REQ-003", None),
        ]
    );
    let debt = body.debt();
    assert_eq!(
        debt,
        DebtCounts {
            unknown: 0,
            conflict: 1,
        }
    );
    assert_eq!(debt.to_string(), "1 deferred gap (0 unknown, 1 conflict)");
}

#[test]
fn missing_stmt_rejects() {
    // Parity with the strict typed model in `crates/slice`: a
    // `model.yaml` row without a `statement` is malformed, and the
    // inventory must refuse it rather than mint a deferral match key
    // over an empty statement.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");
    let staged = plan(vec![entry("auth-login", vec![])]);
    let slice_dir = root.join(".emery/change/slices/auth-login");
    write_meta(&slice_dir, false);
    write_model(
        &slice_dir,
        r"requirements:
  - id: REQ-001
    title: reset path not evidenced
    status: unknown
",
    );

    let err = plan_gaps_body(&staged, Layout::new(root), &[]).expect_err("malformed model");
    assert!(err.to_string().contains("statement"), "{err}");
}

#[test]
fn deferral_scoped_by_slice() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change/slices")).expect("slices");
    let staged = plan(vec![entry("auth-login", vec![]), entry("payments", vec![])]);
    // Identical bodies in two slices: the match key is `(slice,
    // digest)`, so a deferral on one slice leaves the other open.
    let model = r"requirements:
  - id: REQ-001
    title: reset path not evidenced
    statement: ''
    status: unknown
";
    for slice in ["auth-login", "payments"] {
        let dir = root.join(".emery/change/slices").join(slice);
        write_meta(&dir, false);
        write_model(&dir, model);
    }

    let events =
        vec![deferred(1, "local", 1, "auth-login", &title_digest("reset path not evidenced"))];
    let body = plan_gaps_body(&staged, Layout::new(root), &events).expect("gaps");
    assert_eq!(body.rows[0].slice, "auth-login");
    assert_eq!(body.rows[0].disposition, Some(Disposition::Deferred));
    assert_eq!(body.rows[1].slice, "payments");
    assert_eq!(body.rows[1].disposition, Some(Disposition::Open));
}
