//! Versioned diagnostic reports.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Diagnostic, Severity};

/// Report version, serialized as `1`; all other values are rejected.
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

/// Diagnostic counts by severity.
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

/// Report envelope emitted by check producers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReport {
    /// Envelope version discriminant pinned to `1`.
    pub version: DiagnosticReportVersion,
    /// Diagnostic tally by severity.
    pub summary: DiagnosticSummary,
    /// Diagnostics in producer-defined, wire-preserved order.
    pub findings: Vec<Diagnostic>,
}
