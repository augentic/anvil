//! `coverage.yaml` — one row per declared candidate (RFC-104 D2).
//! Declared fields are operator-owned; `observed-cid`,
//! `observed-revision`, and `survey-error` are survey-written.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// The one supported `coverage.yaml` schema version.
const VERSION: u32 = 1;

/// The coverage record at `<system>/coverage.yaml`.
///
/// Every declared candidate stays durable: no row disappears because
/// access was denied, an adapter could not run, or a source was
/// excluded after review.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Coverage {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// One row per declared candidate, in operator order.
    pub candidates: Vec<Row>,
}

/// One coverage candidate.
///
/// `key`, `location`, `adapter`, `disposition`, and `reason` are
/// operator-declared; survey never rewrites them. The remaining
/// fields are survey-written provenance for this row's source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Row {
    /// Stable source key, unique across the file.
    pub key: String,
    /// Exact origin locator: a URL or a path (joined to the definition
    /// home when relative). Mutable — never a pin.
    pub location: String,
    /// Operator-declared adapter identity (a bare name or an exact
    /// package pin). Present if and only if `disposition` is
    /// `included`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Operator-declared coverage disposition.
    pub disposition: Disposition,
    /// Operator-declared reason for the disposition.
    pub reason: String,
    /// RFC-87 tree identity of the snapshot the last successful survey
    /// of this source materialized. Survey-written; never cleared on a
    /// later failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_cid: Option<SnapshotId>,
    /// Git commit reported by the last successful fetch, when the
    /// origin is Git. Survey-written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_revision: Option<String>,
    /// This-run failure of an included source's access or adapter leg.
    /// Survey-written; removed by the next success of this source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survey_error: Option<SurveyError>,
}

/// Closed coverage disposition set (operator-declared, never an
/// engine auto-promotion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    /// Emery surveys the source at its recorded location.
    Included,
    /// Deliberately outside the boundary.
    Excluded,
    /// Evidence exists but cannot be reached (operator accounting).
    Inaccessible,
    /// No adapter can read this evidence class (operator accounting).
    Unsupported,
    /// Identity or membership needs human judgment.
    Unresolved,
}

/// One included source's this-run failure record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SurveyError {
    /// Which leg failed.
    pub kind: SurveyErrorKind,
    /// Human-readable failure detail.
    pub detail: String,
}

/// Which leg of an included source's run failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurveyErrorKind {
    /// Materialization or fetch failure — no tree was prepared.
    Access,
    /// `survey` or `extract` failure after a tree was prepared.
    Adapter,
}

/// One row's survey-owned mutation for this run. Declared fields are
/// never touched by a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowPatch {
    /// The included source completed: record the observed tree and
    /// clear any prior `survey-error`.
    Observed {
        /// RFC-87 tree identity of the materialized snapshot.
        cid: SnapshotId,
        /// Git commit when the origin reports one.
        revision: Option<String>,
    },
    /// The included source failed this run: record the failure and
    /// leave the prior observed tree in place.
    Failed(SurveyError),
}

impl Coverage {
    /// Load and validate `coverage.yaml` from `path`.
    ///
    /// # Errors
    ///
    /// - `system-coverage-missing` when the file is absent.
    /// - `Error::YamlDe` for malformed YAML or unknown fields.
    /// - `system-coverage-invalid` for an unsupported `version`,
    ///   duplicate or empty keys, an empty `location` / `reason`, or
    ///   an `adapter` that violates the required-iff-`included` rule.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Diag {
                    code: "system-coverage-missing",
                    detail: format!("coverage.yaml not found at {}", path.display()),
                });
            }
            Err(err) => return Err(Error::Io(err)),
        };
        let coverage: Self = serde_saphyr::from_str(&text)?;
        coverage.validate()?;
        Ok(coverage)
    }

    /// Content digest of the canonical YAML encoding, independent of
    /// on-disk formatting (the D10 `coverage-digest`).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    /// The rows whose disposition is `included`, in operator order.
    pub fn included(&self) -> impl Iterator<Item = &Row> {
        self.candidates.iter().filter(|row| row.disposition == Disposition::Included)
    }

    /// Look up one row by its source key.
    #[must_use]
    pub fn row(&self, key: &str) -> Option<&Row> {
        self.candidates.iter().find(|row| row.key == key)
    }

    fn validate(&self) -> Result<(), Error> {
        let invalid = |rule: &str, detail: String| {
            Err(Error::validation_failed("system-coverage-invalid", rule, detail))
        };
        if self.version != VERSION {
            return invalid(
                "unsupported version",
                format!("coverage.yaml version {} is not {VERSION}", self.version),
            );
        }
        let mut seen = BTreeSet::new();
        for row in &self.candidates {
            if row.key.trim().is_empty() {
                return invalid("key required", "a candidate row has an empty `key`".to_string());
            }
            if !seen.insert(row.key.as_str()) {
                return invalid(
                    "duplicate key",
                    format!("candidate key `{}` appears more than once", row.key),
                );
            }
            if row.location.trim().is_empty() {
                return invalid(
                    "location required",
                    format!("candidate `{}` has an empty `location`", row.key),
                );
            }
            if row.reason.trim().is_empty() {
                return invalid(
                    "reason required",
                    format!("candidate `{}` has an empty `reason`", row.key),
                );
            }
            let included = row.disposition == Disposition::Included;
            if included && row.adapter.as_deref().is_none_or(|a| a.trim().is_empty()) {
                return invalid(
                    "adapter required",
                    format!("included candidate `{}` must declare an `adapter`", row.key),
                );
            }
            if !included && row.adapter.is_some() {
                return invalid(
                    "adapter forbidden",
                    format!(
                        "candidate `{}` declares an `adapter` but its disposition is not `included`",
                        row.key
                    ),
                );
            }
        }
        Ok(())
    }
}

/// Surgically persist this run's survey-owned mutations.
///
/// Re-loads the live file (comments and key order are not preserved;
/// git is v1 history), applies `patches` by source key to
/// survey-owned fields only, and canonically rewrites the file.
/// Declared fields (`key`, `location`, `adapter`, `disposition`,
/// `reason`) are never rewritten, a failure never clears a prior
/// `observed-cid`, and a patch for a key the operator has since
/// removed is skipped. Returns the persisted state.
///
/// # Errors
///
/// Load failures per [`Coverage::load`], plus atomic-write failures.
pub fn persist(path: &Path, patches: &BTreeMap<String, RowPatch>) -> Result<Coverage, Error> {
    let mut coverage = Coverage::load(path)?;
    for row in &mut coverage.candidates {
        match patches.get(&row.key) {
            Some(RowPatch::Observed { cid, revision }) => {
                row.observed_cid = Some(cid.clone());
                row.observed_revision = revision.clone();
                row.survey_error = None;
            }
            Some(RowPatch::Failed(error)) => {
                row.survey_error = Some(error.clone());
            }
            None => {}
        }
    }
    artifacts::atomic::yaml_write(path, &coverage)?;
    Ok(coverage)
}
