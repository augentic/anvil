//! Diagnostic fingerprint and canonical JSON helpers.
//!
//! The fingerprint algorithm pins the wire format at `v1`. The
//! algorithm, exclusion table, and inner / outer SHA-256 framing are
//! normative — any drift in canonicalization breaks dedup across CI
//! history. Touch this module only with a deliberate `v2` bump.
//!
//! # Algorithm
//!
//! ```text
//! fingerprint = "sha256:" + hex(sha256(
//!     "v1\n"
//!   + rule-id-or-empty + "\n"
//!   + canonical(location) + "\n"
//!   + hex(sha256(evidence-payload))
//! ))
//! ```
//!
//! Where:
//!
//! - `rule-id-or-empty` is the literal `rule-id` string when set,
//!   the empty string otherwise.
//! - `canonical(location)` is `"{path}:{line}:{column}"` using the
//!   raw [`FindingLocation::path`] verbatim with `line.unwrap_or(0)`
//!   and `column.unwrap_or(0)`. `end-line` and `end-column` are
//!   excluded. When `location` is `None` this term is empty.
//! - `evidence-payload` is the UTF-8 bytes of `evidence.value` for
//!   `kind: snippet`, the UTF-8 bytes of `evidence.summary` for
//!   `kind: digest`, and `evidence.summary + "\n" +
//!   canonical_json(evidence.data)` for `kind: structured`.
//!
//! Producer-side fields — `id`, `title`, `severity`, `kind`,
//! `confidence`, `status`, `disposition`, `change`, `slice`,
//! `target-adapter`, `source-adapter`, `related-rule-ids` — are
//! excluded so that regrading severity, flipping the kind axis,
//! stamping a triage status, attaching slice/change context, or
//! rephrasing a title cannot duplicate diagnostics for the same
//! underlying issue.

use serde_json::Value;
use specify_schema::digest::sha256_hex;

use crate::diagnostic::{Diagnostic, FindingEvidence, FindingLocation};

/// Wire-format version embedded into every fingerprint preimage.
const FINGERPRINT_VERSION: &str = "v1";

/// Compute the diagnostic fingerprint for `diagnostic`.
///
/// Returns `sha256:` followed by 64 lowercase hex chars. The
/// `diagnostic.fingerprint` field is **not** consulted.
#[must_use]
pub fn fingerprint(diagnostic: &Diagnostic) -> String {
    let rule_id = diagnostic.rule_id.as_deref().unwrap_or("");
    let location = canonical_location(diagnostic.location.as_ref());
    let evidence_hex = sha256_hex(evidence_payload(&diagnostic.evidence).as_bytes());

    let mut input = String::with_capacity(
        FINGERPRINT_VERSION
            .len()
            .saturating_add(rule_id.len())
            .saturating_add(location.len())
            .saturating_add(evidence_hex.len())
            .saturating_add(3),
    );
    input.push_str(FINGERPRINT_VERSION);
    input.push('\n');
    input.push_str(rule_id);
    input.push('\n');
    input.push_str(&location);
    input.push('\n');
    input.push_str(&evidence_hex);

    format!("sha256:{}", sha256_hex(input.as_bytes()))
}

/// Recompute the fingerprint from `diagnostic`'s other fields and
/// compare against the stored [`Diagnostic::fingerprint`].
#[must_use]
pub fn verify_fingerprint(diagnostic: &Diagnostic) -> bool {
    let Some(hex) = diagnostic.fingerprint.strip_prefix("sha256:") else {
        return false;
    };
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    fingerprint(diagnostic) == diagnostic.fingerprint
}

/// Canonical JSON serialisation: sorted object keys, no insignificant
/// whitespace, arrays preserve insertion order.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(&mut out, value);
    out
}

fn write_canonical(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            // Serialising a `String` to JSON cannot fail; `unreachable!` keeps
            // fingerprint stability off the `expect` panic path.
            out.push_str(
                &serde_json::to_string(s)
                    .unwrap_or_else(|_| unreachable!("a JSON string is infallibly serialisable")),
            );
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| {
                    unreachable!("a JSON object key is infallibly serialisable")
                }));
                out.push(':');
                write_canonical(out, &map[*key]);
            }
            out.push('}');
        }
    }
}

fn canonical_location(location: Option<&FindingLocation>) -> String {
    location.map_or_else(String::new, |loc| {
        format!(
            "{path}:{line}:{column}",
            path = loc.path,
            line = loc.line.unwrap_or(0),
            column = loc.column.unwrap_or(0),
        )
    })
}

fn evidence_payload(evidence: &FindingEvidence) -> String {
    match evidence {
        FindingEvidence::Snippet { value } => value.clone(),
        FindingEvidence::Digest { summary, .. } => summary.clone(),
        FindingEvidence::Structured { summary, data, .. } => {
            let canonical = canonical_json(data);
            let mut payload = String::with_capacity(summary.len() + 1 + canonical.len());
            payload.push_str(summary);
            payload.push('\n');
            payload.push_str(&canonical);
            payload
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::{canonical_json, fingerprint, verify_fingerprint};
    use crate::diagnostic::{
        Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticSource, DirectiveDisposition,
        DispositionSource, FindingDisposition, FindingEvidence, FindingLocation, FindingStatus,
        Severity,
    };
    use crate::test_support::sample_diagnostic;

    /// (1) Identical inputs produce identical fingerprints.
    #[test]
    fn fingerprint_is_deterministic() {
        let diagnostic = sample_diagnostic();
        let a = fingerprint(&diagnostic);
        let b = fingerprint(&diagnostic);
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
        assert_eq!(a.len(), "sha256:".len() + 64);
    }

    /// (2) Mutating any producer-only field leaves the fingerprint
    /// unchanged — including the new `kind` axis.
    #[test]
    fn excluded_producer_fields_stable_fp() {
        type Mutation = fn(&mut Diagnostic);
        let baseline = fingerprint(&sample_diagnostic());

        let cases: &[Mutation] = &[
            |f| f.id = "FIND-9999".into(),
            |f| f.title = "totally different title".into(),
            |f| f.severity = Severity::Critical,
            |f| f.kind = DiagnosticKind::Review,
            |f| f.confidence = Some(Confidence::Low),
            |f| f.status = Some(FindingStatus::Accepted),
            |f| f.status = Some(FindingStatus::Ignored),
            |f| {
                f.disposition = Some(FindingDisposition {
                    source: DispositionSource::Directive,
                    directive: Some(DirectiveDisposition {
                        path: "crates/invoice_export/src/config.rs".into(),
                        line: 17,
                        rationale: "internal deploy only".into(),
                    }),
                    since: None,
                });
            },
            |f| f.change = Some("other-change".into()),
            |f| f.slice = Some("other-slice".into()),
            |f| f.target_adapter = Some("vectis".into()),
            |f| f.source_adapter = Some("captures".into()),
            |f| f.related_rule_ids = Some(vec!["SEC-001".into()]),
        ];

        for mutate in cases {
            let mut diagnostic = sample_diagnostic();
            mutate(&mut diagnostic);
            assert_eq!(
                fingerprint(&diagnostic),
                baseline,
                "mutating an excluded field must not change the fingerprint"
            );
        }
    }

    /// (3) Changing `rule-id` MUST change the fingerprint.
    #[test]
    fn rule_id_change_changes_fingerprint() {
        let baseline = fingerprint(&sample_diagnostic());

        let mut other = sample_diagnostic();
        other.rule_id = Some("SEC-001".into());
        assert_ne!(fingerprint(&other), baseline);

        let mut absent = sample_diagnostic();
        absent.rule_id = None;
        assert_ne!(fingerprint(&absent), baseline);
    }

    /// (4) Location dimensions enter the hash; end fields do not.
    #[test]
    fn location_dimensions() {
        let baseline = fingerprint(&sample_diagnostic());

        let mut path = sample_diagnostic();
        path.location.as_mut().expect("location present").path = "src/other.rs".into();
        assert_ne!(fingerprint(&path), baseline);

        let mut line = sample_diagnostic();
        line.location.as_mut().expect("location present").line = Some(42);
        assert_ne!(fingerprint(&line), baseline);

        let mut column = sample_diagnostic();
        column.location.as_mut().expect("location present").column = Some(42);
        assert_ne!(fingerprint(&column), baseline);

        let mut ends = sample_diagnostic();
        let loc = ends.location.as_mut().expect("location present");
        loc.end_line = Some(99);
        loc.end_column = Some(99);
        assert_eq!(fingerprint(&ends), baseline);
    }

    /// (5) Snippet value changes the fingerprint.
    #[test]
    fn snippet_value_changes() {
        let baseline = fingerprint(&sample_diagnostic());
        let mut mutated = sample_diagnostic();
        mutated.evidence = FindingEvidence::Snippet {
            value: "different snippet".into(),
        };
        assert_ne!(fingerprint(&mutated), baseline);
    }

    /// (6) Digest summary enters the hash; sha256 and locations do not.
    #[test]
    fn digest_summary_and_sha256_differ() {
        let mut original = sample_diagnostic();
        original.evidence = FindingEvidence::Digest {
            sha256: "a".repeat(64),
            summary: "original summary".into(),
            locations: None,
        };
        let baseline = fingerprint(&original);

        let mut summary_changed = original.clone();
        summary_changed.evidence = FindingEvidence::Digest {
            sha256: "a".repeat(64),
            summary: "different summary".into(),
            locations: None,
        };
        assert_ne!(fingerprint(&summary_changed), baseline);

        let mut sha_changed = original.clone();
        sha_changed.evidence = FindingEvidence::Digest {
            sha256: "b".repeat(64),
            summary: "original summary".into(),
            locations: None,
        };
        assert_eq!(fingerprint(&sha_changed), baseline);

        let mut locations_changed = original;
        locations_changed.evidence = FindingEvidence::Digest {
            sha256: "a".repeat(64),
            summary: "original summary".into(),
            locations: Some(vec![FindingLocation {
                path: "irrelevant.rs".into(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            }]),
        };
        assert_eq!(fingerprint(&locations_changed), baseline);
    }

    /// (7) Structured summary and data enter the hash; key order does
    /// not.
    #[test]
    fn summary_and_data_change() {
        let mut original = sample_diagnostic();
        original.evidence = FindingEvidence::Structured {
            summary: "contract compat".into(),
            data: json!({"breaking": true, "removed": ["GET /v1/foo"]}),
            locations: None,
        };
        let baseline = fingerprint(&original);

        let mut summary_changed = original.clone();
        summary_changed.evidence = FindingEvidence::Structured {
            summary: "different summary".into(),
            data: json!({"breaking": true, "removed": ["GET /v1/foo"]}),
            locations: None,
        };
        assert_ne!(fingerprint(&summary_changed), baseline);

        let mut reordered = original;
        reordered.evidence = FindingEvidence::Structured {
            summary: "contract compat".into(),
            data: json!({"removed": ["GET /v1/foo"], "breaking": true}),
            locations: None,
        };
        assert_eq!(fingerprint(&reordered), baseline, "canonical_json must sort object keys");
    }

    /// (8) `canonical_json` correctness.
    #[test]
    fn canonical_json_correctness() {
        assert_eq!(canonical_json(&json!({"b": 2, "a": 1})), r#"{"a":1,"b":2}"#);
        assert_eq!(canonical_json(&json!([3, 2, 1])), "[3,2,1]");
        assert_eq!(
            canonical_json(&json!({"z": {"y": 1, "x": 2}, "a": [3, 2, 1]})),
            r#"{"a":[3,2,1],"z":{"x":2,"y":1}}"#
        );
        assert_eq!(canonical_json(&json!(null)), "null");
        assert_eq!(
            canonical_json(&json!("hello\nworld\t\"quoted\"")),
            r#""hello\nworld\t\"quoted\"""#
        );
    }

    /// (9) `verify_fingerprint` round-trips and rejects malformed
    /// variants.
    #[test]
    fn verify_round_trip_and_rejection() {
        let mut diagnostic = sample_diagnostic();
        diagnostic.fingerprint = fingerprint(&diagnostic);
        assert!(verify_fingerprint(&diagnostic));

        let mut tampered = diagnostic.clone();
        tampered.rule_id = Some("SEC-001".into());
        assert!(!verify_fingerprint(&tampered));

        let mut missing_prefix = diagnostic.clone();
        missing_prefix.fingerprint =
            diagnostic.fingerprint.trim_start_matches("sha256:").to_owned();
        assert!(!verify_fingerprint(&missing_prefix));

        let mut uppercase = diagnostic.clone();
        let stored_hex =
            diagnostic.fingerprint.strip_prefix("sha256:").expect("baseline has prefix");
        uppercase.fingerprint = format!("sha256:{}", stored_hex.to_ascii_uppercase());
        assert!(!verify_fingerprint(&uppercase));
    }

    /// Golden fingerprint canary — pins the exact wire bytes. The
    /// preimage is unchanged from the pre-relocation algorithm (the
    /// new `kind` axis is excluded from the hash) so the constant
    /// stays identical.
    #[test]
    fn golden_fingerprint_pins_algorithm() {
        let diagnostic = Diagnostic {
            id: "FIND-0001".into(),
            rule_id: Some("UNI-014".into()),
            related_rule_ids: None,
            title: "golden".into(),
            severity: Severity::Important,
            source: DiagnosticSource::Deterministic,
            kind: DiagnosticKind::Violation,
            target_adapter: None,
            source_adapter: None,
            slice: None,
            change: None,
            artifact: Artifact::Code,
            location: Some(FindingLocation {
                path: "src/lib.rs".into(),
                line: Some(18),
                column: Some(5),
                end_line: None,
                end_column: None,
            }),
            evidence: FindingEvidence::Snippet {
                value: "let x = 1;".into(),
            },
            impact: "n/a".into(),
            remediation: "n/a".into(),
            confidence: None,
            fingerprint: String::new(),
            status: None,
            disposition: None,
        };
        assert_eq!(
            fingerprint(&diagnostic),
            "sha256:f3fee654d173694494b18a4b73a5b7d4be0460896457d2f41ad0c7d752beff72",
            "if this fails, the algorithm has drifted; do not re-pin without a v2 bump"
        );
    }

    proptest! {
        // Producer-only fields and the order/content of the excluded
        // `related_rule_ids` Vec never enter the preimage, so the
        // fingerprint stays pinned to the baseline regardless.
        #[test]
        fn excluded_fields_stable(
            title in ".{0,40}",
            ids in prop::collection::vec("[A-Z]{3}-[0-9]{1,4}", 0..6),
            crit in any::<bool>(),
            review in any::<bool>(),
        ) {
            let baseline = fingerprint(&sample_diagnostic());

            let mut d = sample_diagnostic();
            d.title = title;
            d.severity = if crit { Severity::Critical } else { Severity::Important };
            d.kind = if review { DiagnosticKind::Review } else { DiagnosticKind::Violation };

            let mut ascending = ids.clone();
            ascending.sort();
            d.related_rule_ids = Some(ascending);
            prop_assert_eq!(&fingerprint(&d), &baseline);

            let mut descending = ids;
            descending.reverse();
            d.related_rule_ids = Some(descending);
            prop_assert_eq!(&fingerprint(&d), &baseline);
        }

        // `rule-id` is part of the preimage, so two distinct ids over an
        // otherwise-identical diagnostic must fingerprint differently.
        #[test]
        fn rule_id_distinguishes(
            a in "[A-Z]{3}-[0-9]{1,4}",
            b in "[A-Z]{3}-[0-9]{1,4}",
        ) {
            prop_assume!(a != b);
            let mut da = sample_diagnostic();
            da.rule_id = Some(a);
            let mut db = sample_diagnostic();
            db.rule_id = Some(b);
            prop_assert_ne!(fingerprint(&da), fingerprint(&db));
        }
    }
}
