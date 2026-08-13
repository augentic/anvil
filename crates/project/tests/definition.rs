//! RFC-104 handoff DTO, canonical digest, and fail-closed resolution.

use mock::definition::{Spec, mint};
use project::definition::{Handoff, Home, INTENT, resolve};
use project::journal::{Event, EventKind, append_for};
use project::snapshot::SnapshotId;

fn pin(digit: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{digit:x}").repeat(64))
}

fn yaml_with(scope: &str) -> String {
    format!(
        "\
version: 1
definition: demo
scope-digest: {scope_d}
coverage-digest: {cov_d}
sources-digest: {src_d}
system-model-digest: {model_d}
migration-plan-digest: {plan_d}
wave:
  id: deliver
  digest: {wave_d}
  outcome: Deliver the reviewed intent
  architecture:
    before: {{ id: as-is, digest: {before_d} }}
    after: {{ id: target, digest: {after_d} }}
  evidence-scopes:
    - {scope}
",
        scope_d = pin(0x0),
        cov_d = pin(0xc),
        src_d = pin(0xd),
        model_d = pin(0xe),
        plan_d = pin(0xf),
        wave_d = pin(0x5),
        before_d = pin(0xb),
        after_d = pin(0xa),
    )
}

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

#[test]
fn round_trip_and_digest() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::degenerate("ship the orders api")).expect("mint");
    let loaded = Handoff::load(&Home::new(tmp.path()).handoff_path(&minted.digest)).expect("load");
    assert_eq!(loaded, minted.reviewed.handoff);
    assert_eq!(loaded.digest().expect("digest"), minted.digest);

    let reformatted = format!("# comment\n\n{}", loaded.canonical_yaml().expect("yaml"));
    std::fs::write(Home::new(tmp.path()).handoff_path(&minted.digest), reformatted)
        .expect("rewrite");
    let again = resolve(tmp.path(), "deliver").expect("resolve after reformat");
    assert_eq!(again.digest, minted.digest, "YAML reformatting must not move the digest");
    assert_eq!(again.handoff, loaded);
}

#[test]
fn unknown_field_rejected() {
    let mut yaml = yaml_with(
        "source: intent\n      value: hi\n      adapter: emery:intent@1.0.0\n      lead: intent\n      evidence-digest: sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    yaml.push_str("extra: true\n");
    let err = Handoff::parse(&yaml).expect_err("unknown field");
    assert!(code(&err).contains("definition-handoff-malformed"), "{err}");
}

#[test]
fn scope_xor_and_intent() {
    let neither = yaml_with(
        "source: docs\n      adapter: emery:documentation@1.0.0\n      lead: a\n      evidence-digest: sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    let err = Handoff::parse(&neither).expect_err("neither");
    assert!(code(&err).contains("definition-scope-xor"), "{err}");

    let both = yaml_with(
        "source: docs\n      source-cid: sha256:1111111111111111111111111111111111111111111111111111111111111111\n      value: inline\n      adapter: emery:documentation@1.0.0\n      lead: a\n      evidence-digest: sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    let err = Handoff::parse(&both).expect_err("both");
    assert!(code(&err).contains("definition-scope-xor"), "{err}");

    let intent_cid = yaml_with(
        "source: intent\n      source-cid: sha256:1111111111111111111111111111111111111111111111111111111111111111\n      adapter: emery:intent@1.0.0\n      lead: intent\n      evidence-digest: sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    let err = Handoff::parse(&intent_cid).expect_err("intent cid");
    assert!(code(&err).contains("definition-intent-form"), "{err}");

    let located_value = yaml_with(
        "source: docs\n      value: inline\n      adapter: emery:documentation@1.0.0\n      lead: a\n      evidence-digest: sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    let err = Handoff::parse(&located_value).expect_err("located value");
    assert!(code(&err).contains("definition-intent-form"), "{err}");
}

#[test]
fn resolve_missing() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = resolve(tmp.path(), "deliver").expect_err("missing");
    assert!(code(&err).contains("definition-handoff-missing"), "{err}");
}

#[test]
fn resolve_ambiguous() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    mint(tmp.path(), &Spec::degenerate("brief")).expect("mint");
    let extra = tmp.path().join("handoffs").join(format!("{}.yaml", pin(0x9).digest()));
    std::fs::copy(
        Home::new(tmp.path()).handoff_path(&resolve(tmp.path(), "deliver").expect("once").digest),
        extra,
    )
    .expect("copy");
    let err = resolve(tmp.path(), "deliver").expect_err("ambiguous");
    assert!(code(&err).contains("definition-handoff-ambiguous"), "{err}");
}

#[test]
fn resolve_mismatch() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::degenerate("brief")).expect("mint");
    let home = Home::new(tmp.path());
    let src = home.handoff_path(&minted.digest);
    let dst = home.handoffs_dir().join(format!("{}.yaml", pin(0x9).digest()));
    std::fs::rename(&src, &dst).expect("rename");
    let err = resolve(tmp.path(), "deliver").expect_err("mismatch");
    assert!(code(&err).contains("definition-handoff-mismatch"), "{err}");
}

#[test]
fn resolve_review_missing() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    mint(tmp.path(), &Spec::degenerate("brief")).expect("mint");
    std::fs::remove_dir_all(Home::new(tmp.path()).events_dir()).expect("rm events");
    let err = resolve(tmp.path(), "deliver").expect_err("review");
    assert!(code(&err).contains("definition-review-missing"), "{err}");
}

#[test]
fn resolve_event_malformed() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    mint(tmp.path(), &Spec::degenerate("brief")).expect("mint");
    std::fs::write(Home::new(tmp.path()).events_dir().join("local.jsonl"), "{not-json\n")
        .expect("write");
    let err = resolve(tmp.path(), "deliver").expect_err("malformed");
    assert!(code(&err).contains("definition-event-malformed"), "{err}");
}

#[test]
fn append_refuses_review() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = project::config::Layout::new(tmp.path());
    let event = Event::new(
        jiff::Timestamp::from_second(1_700_000_000).expect("ts"),
        EventKind::SystemWaveReviewed {
            handoff_digest: pin(0xa),
        },
    );
    let err = append_for(layout, "local", &[event]).expect_err("refused");
    assert!(code(&err).contains("journal-event-read-only"), "{err}");
}

#[test]
fn review_event_round_trip() {
    let event = Event {
        timestamp: jiff::Timestamp::from_second(1_700_000_000).expect("ts"),
        writer: "local".into(),
        sequence: 1,
        kind: EventKind::SystemWaveReviewed {
            handoff_digest: pin(0xa),
        },
    };
    let wire = serde_json::to_string(&event).expect("serialize");
    assert!(wire.contains(r#""event":"system.wave.reviewed""#), "{wire}");
    assert!(wire.contains(r#""handoff-digest""#), "{wire}");
    assert_eq!(serde_json::from_str::<Event>(&wire).expect("parse"), event);
    let digest = event.digest().expect("digest");
    SnapshotId::parse(digest.as_str()).expect("sha256");
}

#[test]
fn degenerate_intent() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::degenerate("the brief")).expect("mint");
    let scopes = &minted.reviewed.handoff.wave.evidence_scopes;
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].source, INTENT);
    assert_eq!(scopes[0].value.as_deref(), Some("the brief"));
    assert!(scopes[0].source_cid.is_none());
}
