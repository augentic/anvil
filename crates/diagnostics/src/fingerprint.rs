//! Diagnostic fingerprints and canonical JSON.
//!
//! The algorithm is the normative `v1` wire format — drift breaks dedup
//! across CI history; touch it only with a deliberate `v2` bump:
//!
//! ```text
//! fingerprint = "sha256:" + hex(sha256(
//!     "v1\n"
//!   + rule-id-or-empty + "\n"
//!   + canonical(location) + "\n"        // "{path}:{line}:{column}"
//!   + hex(sha256(evidence-payload))
//! ))
//! ```
//!
//! For digest evidence, `evidence-payload` is the summary rather than the
//! referenced digest. Fingerprints identify a diagnostic for deduplication;
//! the digest separately identifies and verifies the underlying evidence bytes.
//! Producers must therefore keep digest summaries stable and discriminating.

use serde_json::Value;

use crate::diagnostic::{Diagnostic, FindingEvidence, FindingLocation};
use crate::digest::sha256_hex;

const FINGERPRINT_VERSION: &str = "v1";

/// Compute a `sha256:<64 lowercase hex>` fingerprint.
///
/// The stored fingerprint field is not consulted.
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

/// Verify the stored [`Diagnostic::fingerprint`].
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

/// Serialize JSON with sorted keys, no extra whitespace, and stable array order.
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
            // JSON strings serialize infallibly; avoid an `expect` panic path.
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
        // Normative v1 behavior: summary is the diagnostic identity while
        // `sha256` is the identity of the separately stored evidence bytes.
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
