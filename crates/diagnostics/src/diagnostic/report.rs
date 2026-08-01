//! The [`DiagnosticReport`] envelope: the version pin, the
//! per-severity [`DiagnosticSummary`] tally, and the report shape
//! every check producer emits.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Diagnostic, Severity};

/// Type-level pin of the [`DiagnosticReport`] envelope version.
///
/// Serialises to the integer `1` and refuses to deserialise any other
/// value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticReportVersion;

impl Serialize for DiagnosticReportVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for DiagnosticReportVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported DiagnosticReport version: {value} (only v1 is supported)"
            )))
        }
    }
}

/// Diagnostic tally by severity for the [`DiagnosticReport`] envelope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSummary {
    /// Count of diagnostics with `severity: critical`.
    pub critical: u32,
    /// Count of diagnostics with `severity: important`.
    pub important: u32,
    /// Count of diagnostics with `severity: suggestion`.
    pub suggestion: u32,
    /// Count of diagnostics with `severity: optional`.
    pub optional: u32,
}

impl DiagnosticSummary {
    /// Tally `diagnostics` by severity.
    #[must_use]
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut summary = Self::default();
        for diagnostic in diagnostics {
            match diagnostic.severity {
                Severity::Critical => summary.critical += 1,
                Severity::Important => summary.important += 1,
                Severity::Suggestion => summary.suggestion += 1,
                Severity::Optional => summary.optional += 1,
            }
        }
        summary
    }
}

/// Diagnostic report envelope — `{ version, summary, diagnostics }` —
/// emitted by every check producer.
///
/// ```
/// use diagnostics::{Artifact, Diagnostic, DiagnosticReport, DiagnosticSummary};
///
/// let findings = vec![Diagnostic::violation(
///     "spec.requirement-id-missing",
///     "Every requirement carries an `ID:` line",
///     "`### Requirement: Login` has no `ID:` line",
///     Artifact::Specs,
///     None,
/// )];
/// let report = DiagnosticReport {
///     version: Default::default(),
///     summary: DiagnosticSummary::from_diagnostics(&findings),
///     findings,
/// };
/// let wire = serde_json::to_string(&report).unwrap();
/// assert!(wire.contains("spec.requirement-id-missing"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReport {
    /// Envelope version discriminant pinned to `1`.
    pub version: DiagnosticReportVersion,
    /// Diagnostic tally by severity.
    pub summary: DiagnosticSummary,
    /// Byte-stable list of structured diagnostics. Ordering is the
    /// producer's responsibility and is preserved on the wire.
    pub findings: Vec<Diagnostic>,
}
