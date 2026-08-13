//! `decisions/<id>.yaml` — operator-authored definition decisions
//! (RFC-104 D4). Not product `DEC-NNNN` Decision Records; the engine
//! never writes the directory.

use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// The one supported decision schema version.
const VERSION: u32 = 1;

/// One definition decision record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Decision {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// Kebab-case id, equal to the filename stem.
    pub id: String,
    /// Element or relationship ids the persist tail stamps as
    /// `status: decided`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applies_to: Vec<String>,
    /// Lineage: decision ids this record replaced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// The situation the decision was taken in.
    pub context: String,
    /// The decision itself.
    pub decision: String,
    /// What follows from it.
    pub consequences: String,
}

impl Decision {
    /// Content digest of the canonical YAML encoding (the handoff's
    /// per-decision `digest`).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }
}

/// Load every `decisions/<id>.yaml` beneath `dir`, sorted by id.
///
/// An absent directory is the valid empty set. Each record's `id`
/// must be a kebab slug equal to its filename stem, and no two
/// records may apply to the same model id.
///
/// # Errors
///
/// - `Error::YamlDe` for malformed YAML or unknown fields.
/// - `system-decision-invalid` for a version, id, or `applies-to`
///   violation.
pub fn load_all(dir: &Path) -> Result<Vec<Decision>, Error> {
    let invalid = |rule: &str, detail: String| {
        Err(Error::validation_failed("system-decision-invalid", rule, detail))
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::Io(err)),
    };
    let mut decisions = Vec::new();
    for entry in entries {
        let path = entry.map_err(Error::Io)?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(Error::Io)?;
        let decision: Decision = serde_saphyr::from_str(&text)?;
        if decision.version != VERSION {
            return invalid(
                "unsupported version",
                format!("{}: version {} is not {VERSION}", path.display(), decision.version),
            );
        }
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        if !artifacts::evidence::is_kebab(&decision.id) || decision.id != stem {
            return invalid(
                "id is the filename stem",
                format!("{}: id `{}` must be the kebab filename stem", path.display(), decision.id),
            );
        }
        decisions.push(decision);
    }
    decisions.sort_by(|a, b| a.id.cmp(&b.id));

    let mut applied = std::collections::BTreeMap::new();
    for decision in &decisions {
        for target in &decision.applies_to {
            if let Some(other) = applied.insert(target.as_str(), decision.id.as_str()) {
                return invalid(
                    "one decision per id",
                    format!("`{other}` and `{}` both apply to `{target}`", decision.id),
                );
            }
        }
    }
    Ok(decisions)
}
