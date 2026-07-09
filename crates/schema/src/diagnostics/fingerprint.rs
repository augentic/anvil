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

use schema::digest::sha256_hex;
use serde_json::Value;

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
///
/// ```
/// use serde_json::json;
/// use diagnostics::canonical_json;
///
/// let value = json!({"b": 1, "a": [2, 1]});
/// assert_eq!(canonical_json(&value), r#"{"a":[2,1],"b":1}"#);
/// ```
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
