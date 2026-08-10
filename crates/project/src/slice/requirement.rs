//! Canonical requirement-body digest — the deferral match key (RFC-86a
//! D2), shared by the typed `model.yaml` view (`slice` crate) and the
//! gap-inventory fact join (`plan::gaps`).

use diagnostics::digest::sha256_hex;

/// Borrowed view of one requirement's body content — the fields
/// `model.yaml` carries as agent-authored prose.
///
/// Kernel-owned fields (`REQ-NNN` id, status, sources, claims)
/// deliberately take no part, so a re-refine that renumbers ids keeps
/// the digest while any body edit changes it.
#[derive(Debug, Clone, Copy)]
pub struct RequirementBody<'a> {
    /// Requirement title.
    pub title: &'a str,
    /// Behavioral statement.
    pub statement: &'a str,
    /// Scenario lines, in declaration order.
    pub scenarios: &'a [String],
    /// Free-form notes, when present.
    pub notes: Option<&'a str>,
}

impl RequirementBody<'_> {
    /// Canonical `sha256:<hex>` digest of this body.
    ///
    /// Format-independent: the digest is over a length-framed record
    /// encoding of the fields, not over any YAML/Markdown rendering, so
    /// serialization details never perturb it. The byte-length prefix
    /// makes framing unambiguous — a value embedding newlines or text
    /// that mimics a record header cannot collide with a field
    /// boundary.
    #[must_use]
    pub fn digest(&self) -> String {
        let mut encoded = Vec::new();
        record(&mut encoded, "title", self.title);
        record(&mut encoded, "statement", self.statement);
        for scenario in self.scenarios {
            record(&mut encoded, "scenario", scenario);
        }
        if let Some(notes) = self.notes {
            record(&mut encoded, "notes", notes);
        }
        format!("sha256:{}", sha256_hex(&encoded))
    }
}

/// Append one `<field> <byte-len>\n<value>\n` record.
fn record(out: &mut Vec<u8>, field: &str, value: &str) {
    out.extend_from_slice(field.as_bytes());
    out.push(b' ');
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(value.as_bytes());
    out.push(b'\n');
}
