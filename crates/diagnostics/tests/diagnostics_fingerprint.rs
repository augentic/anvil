//! `v1` fingerprint algorithm: determinism, the excluded/included
//! field matrix, `verify_fingerprint`, and canonical JSON.

use emery_diagnostics::digest::sha256_hex;
use emery_diagnostics::{
    Confidence, DiagnosticKind, FindingEvidence, FindingLocation, Severity, canonical_json,
    fingerprint, verify_fingerprint,
};
use proptest::prelude::*;
use serde_json::json;

use crate::diagnostics_support::sample_diagnostic;

mod diagnostics_support;

#[test]
fn empty_digest_kat() {
    assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn streamed_digest_parity() {
    use std::io::Cursor;

    use emery_diagnostics::digest::sha256_reader;

    let payload: Vec<u8> = (0_u8..=250).cycle().take(200_000).collect();
    let mut cursor = Cursor::new(&payload);
    assert_eq!(
        sha256_reader(&mut cursor).expect("stream"),
        sha256_hex(&payload),
        "streamed SHA-256 equals hashing the same bytes as a slice"
    );
}

#[test]
fn fp_is_deterministic() {
    let d = sample_diagnostic();
    let fp = fingerprint(&d);
    assert_eq!(fp, fingerprint(&d));
    assert!(fp.starts_with("sha256:"));
    assert_eq!(fp.len(), 71);
}

#[test]
fn fp_excludes_producer() {
    let base = sample_diagnostic();
    let expected = fingerprint(&base);

    let mut regraded = base.clone();
    regraded.severity = Severity::Optional;
    assert_eq!(fingerprint(&regraded), expected, "severity is excluded");

    let mut flipped = base.clone();
    flipped.kind = DiagnosticKind::Review;
    assert_eq!(fingerprint(&flipped), expected, "kind axis is excluded");

    let mut reworded = base;
    reworded.title = "totally different title".into();
    reworded.id = "DIAG-9999".into();
    reworded.slice = Some("other-slice".into());
    reworded.change = Some("other-change".into());
    reworded.target_adapter = Some("vectis".into());
    reworded.confidence = Some(Confidence::Low);
    assert_eq!(fingerprint(&reworded), expected, "context is excluded");
}

#[test]
fn fp_includes_identity() {
    let base = sample_diagnostic();
    let expected = fingerprint(&base);

    let mut ruled = base.clone();
    ruled.rule_id = Some("UNI-999".into());
    assert_ne!(fingerprint(&ruled), expected, "rule-id enters the hash");

    let mut moved = base.clone();
    moved.location = Some(FindingLocation {
        path: "src/other.rs".into(),
        line: Some(1),
        column: Some(1),
        end_line: None,
        end_column: None,
    });
    assert_ne!(fingerprint(&moved), expected, "location enters the hash");

    let mut reworded = base;
    reworded.evidence = FindingEvidence::Snippet {
        value: "different".into(),
    };
    assert_ne!(fingerprint(&reworded), expected, "evidence enters the hash");
}

#[test]
fn fp_ignores_range_end() {
    let base = sample_diagnostic();
    let expected = fingerprint(&base);
    let mut widened = base;
    if let Some(loc) = widened.location.as_mut() {
        loc.end_line = Some(999);
        loc.end_column = Some(999);
    }
    assert_eq!(fingerprint(&widened), expected);
}

#[test]
fn verify_roundtrip_tamper() {
    let mut d = sample_diagnostic();
    d.fingerprint = fingerprint(&d);
    assert!(verify_fingerprint(&d));

    let mut tampered = d;
    tampered.evidence = FindingEvidence::Snippet {
        value: "mutated".into(),
    };
    assert!(!verify_fingerprint(&tampered));
}

#[test]
fn verify_rejects_malformed() {
    let mut d = sample_diagnostic();
    d.fingerprint = "deadbeef".into();
    assert!(!verify_fingerprint(&d), "missing sha256: prefix");
    d.fingerprint = "sha256:zz".into();
    assert!(!verify_fingerprint(&d), "wrong length / non-hex");
}

#[test]
fn canonical_json_sorts_keys() {
    let value = json!({ "b": 1, "a": [3, 2, 1], "c": { "y": 2, "x": 1 } });
    assert_eq!(canonical_json(&value), r#"{"a":[3,2,1],"b":1,"c":{"x":1,"y":2}}"#);
}

proptest! {
    #[test]
    fn fp_stable_over_inputs(rule in "[a-z-]{0,12}", payload in ".{0,32}") {
        let mut d = sample_diagnostic();
        d.rule_id = Some(rule);
        d.evidence = FindingEvidence::Snippet { value: payload };
        let fp = fingerprint(&d);
        prop_assert_eq!(&fp, &fingerprint(&d));
        prop_assert!(fp.starts_with("sha256:"));
        prop_assert_eq!(fp.len(), 71);
    }
}
