//! The judgment-answer deserializers: schema pins, envelope shapes, and
//! the report projection onto the compact seam types.

use adapter::answers::{
    EVIDENCE_ANSWER_SCHEMA, LEADS_ANSWER_SCHEMA, REPORT_ANSWER_SCHEMA, ReportAnswer,
    parse_evidence, parse_leads, validate_evidence, validate_leads,
};
use adapter::seam::{Authority, Backing, ClaimKind, Error, Severity, Status};

#[test]
fn schema_pins() {
    for (pin, file) in [
        (LEADS_ANSWER_SCHEMA, "leads.schema.json"),
        (EVIDENCE_ANSWER_SCHEMA, "evidence.schema.json"),
        (REPORT_ANSWER_SCHEMA, "report.schema.json"),
    ] {
        let on_disk = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/answers").join(file),
        )
        .expect("vendored schema file");
        assert_eq!(pin, on_disk, "pin matches {file}");
    }
}

#[test]
fn leads_deserialize() {
    let leads = parse_leads(
        r#"{"leads":[
            {"lead":"password-reset","synopsis":"Reset flow with expiry.","topics":["auth","email"]},
            {"lead":"session-timeout","synopsis":"Sessions expire after 30 minutes."}
        ]}"#,
    )
    .expect("leads envelope parses");

    assert_eq!(leads.len(), 2);
    assert_eq!(leads[0].lead, "password-reset");
    assert_eq!(leads[0].topics, vec!["auth", "email"]);
    assert_eq!(leads[1].synopsis, "Sessions expire after 30 minutes.");
    assert!(leads[1].topics.is_empty(), "omitted topics defaults to empty");

    assert!(parse_leads(r#"[{"lead":"bare"}]"#).is_err(), "a bare array is not the envelope");
}

// Covers both backing variants and tolerated open per-kind body fields
// (`replay-digest`, `input`, …).
#[test]
fn evidence_deserializes() {
    let evidence = parse_evidence(
        r#"{
            "authority": "behaviour",
            "claims": [
                {
                    "kind": "example",
                    "id": "password-reset.expiry",
                    "path": "captures/reset.json#L3-L9",
                    "synopsis": "Expired token is rejected.",
                    "backing": {"path": "captures/reset.json"},
                    "replay-digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    "input": {"token": "stale"},
                    "output": {"status": 410}
                },
                {"kind": "type", "backing": {"payload": "struct ResetToken { expiry: Instant }"}}
            ]
        }"#,
    )
    .expect("evidence body parses");

    assert_eq!(evidence.authority, Authority::Behaviour);
    assert_eq!(evidence.claims.len(), 2);
    let example = &evidence.claims[0];
    assert_eq!(example.kind, ClaimKind::Example);
    assert_eq!(example.id.as_deref(), Some("password-reset.expiry"));
    assert_eq!(example.path.as_deref(), Some("captures/reset.json#L3-L9"));
    assert_eq!(example.backing, Some(Backing::Path("captures/reset.json".to_string())));
    let claim = &evidence.claims[1];
    assert_eq!(claim.kind, ClaimKind::Type, "`type` deserializes despite being a keyword");
    assert_eq!(
        claim.backing,
        Some(Backing::Payload("struct ResetToken { expiry: Instant }".to_string()))
    );
    assert!(claim.id.is_none() && claim.path.is_none() && claim.synopsis.is_none());
}

// The answer schema does not pin the shape of `synopsis` / `backing`, so
// an unexpected shape drops the field instead of failing the extract.
#[test]
fn open_body_fields_lenient() {
    let evidence = parse_evidence(
        r#"{
            "authority": "documentation",
            "claims": [
                {"kind": "section", "synopsis": {"headline": "structured"}, "backing": "bare string"},
                {"kind": "decision", "synopsis": "kept", "backing": {"payload": "ADR-7"}}
            ]
        }"#,
    )
    .expect("unpinned body shapes never fail the answer");

    let odd = &evidence.claims[0];
    assert!(odd.synopsis.is_none(), "non-string synopsis is dropped");
    assert!(odd.backing.is_none(), "non-variant backing is dropped");
    let clean = &evidence.claims[1];
    assert_eq!(clean.synopsis.as_deref(), Some("kept"), "modeled shapes still parse");
    assert_eq!(clean.backing, Some(Backing::Payload("ADR-7".to_string())));
}

#[test]
fn leads_tail() {
    let clean = parse_leads(
        r#"{"leads":[{"lead":"password-reset","synopsis":"Reset flow with expiry."}]}"#,
    )
    .expect("clean leads parse");
    validate_leads(&clean).expect("clean leads pass the tail");

    let malformed = parse_leads(
        r#"{"leads":[
            {"lead":"Bad_Id","synopsis":"Casing and underscore violate the pattern."},
            {"lead":"blank-synopsis","synopsis":"   "}
        ]}"#,
    )
    .expect("the tail, not the parser, rejects malformed ids");
    let Err(Error::Internal(detail)) = validate_leads(&malformed) else {
        panic!("malformed leads must fail the tail");
    };
    assert!(detail.contains("lead `Bad_Id`"), "finding names the malformed id: {detail}");
    assert!(detail.contains("synopsis is empty"), "finding names the empty synopsis: {detail}");
}

#[test]
fn evidence_tail() {
    let clean = parse_evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request"},
            {"kind":"decision"}
        ]}"#,
    )
    .expect("clean evidence parses");
    validate_evidence(&clean).expect("clean evidence passes the tail");

    let malformed = parse_evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement"},
            {"kind":"criterion","id":"Not.Valid"}
        ]}"#,
    )
    .expect("the tail, not the parser, rejects malformed claims");
    let Err(Error::Internal(detail)) = validate_evidence(&malformed) else {
        panic!("malformed evidence must fail the tail");
    };
    assert!(detail.contains("claims require an id"), "finding names the missing id: {detail}");
    assert!(detail.contains("`Not.Valid`"), "finding names the malformed id: {detail}");
}

#[test]
fn report_projection() {
    let answer = ReportAnswer::parse(
        r#"{
            "status": "failure",
            "findings": [{
                "rule-id": "UNI-014",
                "title": "Duplicate id",
                "severity": "critical",
                "impact": "Baseline is ambiguous.",
                "remediation": "Rename one contract."
            }],
            "outputs": [{"platform": "ios", "path": "ios/App.swift"}],
            "ui-surface": {"screens": 2}
        }"#,
    )
    .expect("report body parses");
    let report = answer.into_report();

    assert_eq!(report.status, Status::Failure);
    let finding = &report.findings[0];
    assert_eq!(finding.rule_id.as_deref(), Some("UNI-014"));
    assert_eq!(finding.severity, Severity::Critical);
    assert_eq!(
        finding.detail,
        "Duplicate id — Baseline is ambiguous.; remediation: Rename one contract."
    );
    assert_eq!(report.outputs[0].path, "ios/App.swift");
    assert_eq!(report.ui_surface.map(|surface| surface.screens), Some(2));

    let minimal = ReportAnswer::parse(r#"{"status":"success","findings":[]}"#)
        .expect("minimal report parses")
        .into_report();
    assert_eq!(minimal.status, Status::Success);
    assert!(minimal.findings.is_empty() && minimal.outputs.is_empty());
    assert!(minimal.ui_surface.is_none());
}
