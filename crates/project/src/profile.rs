//! Closed model-capability profiles: scoring policy, host table, digest.

use std::collections::{BTreeMap, BTreeSet};

use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::plan::ProfileRef;
use crate::snapshot::SnapshotId;

/// Wire version stamped into every profile body.
pub const VERSION: u32 = 1;

/// Model-class key of the compiled default.
pub const FRONTIER_LARGE: &str = "frontier-large";

/// Profile id of the compiled default (class plus calibration version).
pub const FRONTIER_LARGE_V1: &str = "frontier-large-v1";

/// Inclusive upper bound on each assessment dimension.
pub const DIM_MAX: u8 = 10;

/// Host-supplied model-capability profile table.
pub trait Profiles: Send + Sync {
    /// The table this deployment compiled or substituted.
    fn profiles(&self) -> &Table;
}

/// Closed profile body: identity, weights, and operation thresholds.
///
/// The canonical digest covers this body and is independent of YAML
/// formatting. Declared starting values, not calibrated measurements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Profile {
    /// Profile id (`frontier-large-v1` for the compiled default).
    pub id: String,
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Per-dimension weights applied to a closed assessment.
    pub weights: Weights,
    /// Operation thresholds compared against the weighted sum.
    pub thresholds: Thresholds,
}

/// Weights over the five closed assessment dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Weights {
    /// Behavioural-breadth weight.
    pub behavioural_breadth: u32,
    /// Coupling weight.
    pub coupling: u32,
    /// Uncertainty weight.
    pub uncertainty: u32,
    /// Context-volume weight.
    pub context_volume: u32,
    /// Verification-surface weight.
    pub verification_surface: u32,
}

/// Operation-specific thresholds on the weighted sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Thresholds {
    /// Slice-split threshold (RFC-88 leaf-readiness).
    pub slice_split: u32,
    /// Task threshold (recorded for RFC-96).
    pub task: u32,
}

/// Judgment-supplied integers (0–[`DIM_MAX`]) for the five dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Assessment {
    /// Behavioural-breadth score.
    pub behavioural_breadth: u8,
    /// Coupling score.
    pub coupling: u8,
    /// Uncertainty score.
    pub uncertainty: u8,
    /// Context-volume score.
    pub context_volume: u8,
    /// Verification-surface score.
    pub verification_surface: u8,
}

/// Which operation threshold to apply to a weighted sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Slice-split (RFC-88).
    SliceSplit,
    /// Task (RFC-96).
    Task,
}

/// Host-supplied table of profiles, keyed by model class.
///
/// Until a second class exists, [`Self::resolve`] returns the sole
/// entry for every target. A replacement is the whole table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    entries: BTreeMap<String, Profile>,
}

impl Profile {
    /// Compiled `frontier-large-v1` body (RFC worked-example values).
    #[must_use]
    pub fn frontier_v1() -> Self {
        Self {
            id: FRONTIER_LARGE_V1.into(),
            version: VERSION,
            weights: Weights {
                behavioural_breadth: 3,
                coupling: 4,
                uncertainty: 2,
                context_volume: 1,
                verification_surface: 3,
            },
            thresholds: Thresholds {
                slice_split: 80,
                task: 35,
            },
        }
    }

    /// Parse YAML, reject unknown fields, and enforce closed invariants.
    ///
    /// # Errors
    ///
    /// `profile-malformed` on YAML/unknown-field failures;
    /// `profile-version` on a wire-version mismatch;
    /// `profile-malformed` on an empty id.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let profile: Self = serde_saphyr::from_str(text).map_err(|err| Error::Diag {
            code: "profile-malformed",
            detail: err.to_string(),
        })?;
        profile.check()?;
        Ok(profile)
    }

    /// Canonical YAML bytes (trailing newline, stable field order).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`] as a [`SnapshotId`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Id plus canonical digest for `plan.yaml.targets`.
    ///
    /// # Errors
    ///
    /// YAML serialization failures from [`Self::digest`].
    pub fn reference(&self) -> Result<ProfileRef, Error> {
        Ok(ProfileRef {
            id: self.id.clone(),
            digest: self.digest()?,
        })
    }

    /// Weighted sum of `assessment` against this profile's weights.
    ///
    /// # Errors
    ///
    /// `profile-dimension-range` when a dimension exceeds [`DIM_MAX`];
    /// `profile-score-overflow` when the sum overflows `u32`.
    pub fn score(&self, assessment: &Assessment) -> Result<u32, Error> {
        assessment.check()?;
        let terms = [
            term(assessment.behavioural_breadth, self.weights.behavioural_breadth)?,
            term(assessment.coupling, self.weights.coupling)?,
            term(assessment.uncertainty, self.weights.uncertainty)?,
            term(assessment.context_volume, self.weights.context_volume)?,
            term(assessment.verification_surface, self.weights.verification_surface)?,
        ];
        terms.into_iter().try_fold(0_u32, |sum, next| sum.checked_add(next).ok_or_else(overflow))
    }

    /// Whether the weighted sum is strictly above the named threshold.
    ///
    /// # Errors
    ///
    /// The same codes as [`Self::score`].
    pub fn exceeds(&self, assessment: &Assessment, gate: Gate) -> Result<bool, Error> {
        Ok(self.score(assessment)? > self.thresholds.get(gate))
    }

    fn check(&self) -> Result<(), Error> {
        if self.version != VERSION {
            return Err(Error::Diag {
                code: "profile-version",
                detail: format!("profile version `{}` is not `{VERSION}`", self.version),
            });
        }
        if self.id.is_empty() {
            return Err(Error::Diag {
                code: "profile-malformed",
                detail: "profile `id` must be non-empty".into(),
            });
        }
        Ok(())
    }
}

impl Thresholds {
    const fn get(self, gate: Gate) -> u32 {
        match gate {
            Gate::SliceSplit => self.slice_split,
            Gate::Task => self.task,
        }
    }
}

impl Assessment {
    fn check(self) -> Result<(), Error> {
        for (name, value) in [
            ("behavioural-breadth", self.behavioural_breadth),
            ("coupling", self.coupling),
            ("uncertainty", self.uncertainty),
            ("context-volume", self.context_volume),
            ("verification-surface", self.verification_surface),
        ] {
            if value > DIM_MAX {
                return Err(Error::Diag {
                    code: "profile-dimension-range",
                    detail: format!("dimension `{name}` `{value}` is outside 0–{DIM_MAX}"),
                });
            }
        }
        Ok(())
    }
}

impl Table {
    /// Compiled table: the single `frontier-large` → `frontier-large-v1` entry.
    ///
    /// # Panics
    ///
    /// Never: the compiled table is statically valid.
    #[must_use]
    pub fn compiled() -> Self {
        Self::new(BTreeMap::from([(FRONTIER_LARGE.into(), Profile::frontier_v1())]))
            .expect("the compiled profile table is statically valid")
    }

    /// Validate non-empty class keys, unique profile ids, and each body.
    ///
    /// # Errors
    ///
    /// `profile-table-invalid` when the table is empty, a class key is
    /// empty, or a profile id repeats; body errors from [`Profile::parse`].
    pub fn new(entries: BTreeMap<String, Profile>) -> Result<Self, Error> {
        if entries.is_empty() {
            return Err(invalid("profile table is empty"));
        }
        let mut ids = BTreeSet::new();
        for (class, profile) in &entries {
            if class.is_empty() {
                return Err(invalid("model-class key is empty"));
            }
            profile.check()?;
            if !ids.insert(profile.id.as_str()) {
                return Err(invalid(format!("duplicate profile id `{}`", profile.id)));
            }
        }
        Ok(Self { entries })
    }

    /// The sole compiled (or host-supplied) profile, for every target.
    ///
    /// # Errors
    ///
    /// `profile-class-required` when the table has more than one entry.
    pub fn resolve(&self) -> Result<&Profile, Error> {
        let mut values = self.entries.values();
        match (values.next(), values.next()) {
            (Some(profile), None) => Ok(profile),
            (None, _) => Err(invalid("profile table is empty")),
            (Some(_), Some(_)) => Err(Error::Diag {
                code: "profile-class-required",
                detail: "multiple model-capability profiles are compiled; a model class is \
                         required to select one"
                    .into(),
            }),
        }
    }

    /// Look up the profile compiled for `class`.
    ///
    /// # Errors
    ///
    /// `profile-class-unknown` when `class` is absent.
    pub fn get(&self, class: &str) -> Result<&Profile, Error> {
        self.entries.get(class).ok_or_else(|| Error::Diag {
            code: "profile-class-unknown",
            detail: format!("no model-capability profile for class `{class}`"),
        })
    }
}

fn term(dim: u8, weight: u32) -> Result<u32, Error> {
    u32::from(dim).checked_mul(weight).ok_or_else(overflow)
}

fn overflow() -> Error {
    Error::Diag {
        code: "profile-score-overflow",
        detail: "weighted sum overflowed u32".into(),
    }
}

fn invalid(detail: impl Into<String>) -> Error {
    Error::Diag {
        code: "profile-table-invalid",
        detail: detail.into(),
    }
}
