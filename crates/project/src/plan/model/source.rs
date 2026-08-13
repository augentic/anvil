//! Source concerns: the top-level `plan.yaml.sources` binding and the
//! per-entry `(source, lead)` binding.

use serde::{Deserialize, Serialize};

use crate::adapter::AdapterSelector;
use crate::adapter::catalog::{INTENT, Pin};
use crate::snapshot::SnapshotId;

/// One top-level [`super::Plan::sources`] binding.
///
/// Exact adapter pin plus exactly one of `locator` or `value`. Location
/// rows carry a tree `cid`; the reserved `intent` row is the `value`
/// arm only (no locator, no CID).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SourceBinding {
    /// Exact package pin (`emery:<name>@<semver>`).
    pub adapter: Pin,
    /// Git, path, or HTTPS locator. Mutually exclusive with `value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// Inline value; required for [`INTENT`], forbidden with `locator`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Content-addressed identity of a location-backed tree.
    /// Absent on the `intent` / inline-value arm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cid: Option<SnapshotId>,
}

impl SourceBinding {
    /// Reserved `intent` row: adapter pin plus inline value, no CID.
    #[must_use]
    pub fn intent(adapter: Pin, value: impl Into<String>) -> Self {
        Self {
            adapter,
            locator: None,
            value: Some(value.into()),
            cid: None,
        }
    }

    /// Location-backed row with a recorded CID.
    #[must_use]
    pub fn located(adapter: Pin, locator: impl Into<String>, cid: SnapshotId) -> Self {
        Self {
            adapter,
            locator: Some(locator.into()),
            value: None,
            cid: Some(cid),
        }
    }

    /// Typed adapter selector this binding resolves through.
    #[must_use]
    pub fn selector(&self) -> AdapterSelector {
        self.adapter.selector()
    }

    /// Whether this row is the reserved inline `intent` arm.
    #[must_use]
    pub fn is_intent(&self) -> bool {
        self.adapter.name == INTENT
    }

    /// Enforce locator xor value, and the `intent` value-only rule.
    ///
    /// # Errors
    ///
    /// `source-binding-xor` when both or neither arm is present;
    /// `source-intent-locator` when `intent` carries a locator or CID.
    pub fn validate(&self, key: &str) -> Result<(), error::Error> {
        let has_locator = self.locator.as_ref().is_some_and(|locator| !locator.is_empty());
        let has_value = self.value.as_ref().is_some_and(|value| !value.is_empty());
        match (has_locator, has_value) {
            (true, false) | (false, true) => {}
            (false, false) => {
                return Err(error::Error::Diag {
                    code: "source-binding-xor",
                    detail: format!("source `{key}` must carry `locator` or `value`"),
                });
            }
            (true, true) => {
                return Err(error::Error::Diag {
                    code: "source-binding-xor",
                    detail: format!("source `{key}` must not carry both `locator` and `value`"),
                });
            }
        }
        if self.adapter.name == INTENT {
            if has_locator || self.cid.is_some() {
                return Err(error::Error::Diag {
                    code: "source-intent-locator",
                    detail: "adapter `intent` is inline `value` only; a locator is refused".into(),
                });
            }
            if key != INTENT {
                return Err(error::Error::Diag {
                    code: "source-intent-key",
                    detail: format!("adapter `intent` is reserved as source key `{INTENT}`"),
                });
            }
        }
        Ok(())
    }
}

/// One `(source, lead)` binding under [`super::Entry::sources`].
///
/// On the wire this is either a bare string `<key>` (shorthand for
/// `{ source: <key>, lead: <slice.name> }`) or the structured
/// `{ source, lead }` object. Both shapes round-trip byte-identically:
/// the bare shorthand parses to `lead == None` and `Serialize` emits
/// the shape the operator authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceSourceBinding {
    /// Source key matching a top-level [`super::Plan::sources`] entry.
    /// Always present, regardless of which wire shape produced this
    /// value.
    pub source: String,
    /// Lead id from the catalog, resolved within `source`.
    /// `None` denotes the bare-string shorthand — the lead falls
    /// back to the owning slice's name via
    /// the binding's internal lead accessor.
    pub lead: Option<String>,
}

impl SliceSourceBinding {
    /// Construct the bare-string shorthand form: lead defaults to
    /// the owning slice's name at lookup time.
    #[must_use]
    pub fn bare(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lead: None,
        }
    }

    /// Construct the structured form with an explicit lead.
    #[must_use]
    pub fn structured(source: impl Into<String>, lead: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lead: Some(lead.into()),
        }
    }

    /// The source key this binding references in
    /// [`super::Plan::sources`].
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The lead this binding pairs with, falling back to the
    /// owning slice's name for the bare-string shorthand per the
    /// workflow contract §`Slice.sources`.
    #[must_use]
    pub fn lead<'a>(&'a self, slice_name: &'a str) -> &'a str {
        self.lead.as_deref().unwrap_or(slice_name)
    }
}

impl Serialize for SliceSourceBinding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.lead {
            None => serializer.serialize_str(&self.source),
            Some(lead) => {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct("SliceSourceBinding", 2)?;
                state.serialize_field("source", &self.source)?;
                state.serialize_field("lead", lead)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for SliceSourceBinding {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Bare(String),
            Structured {
                #[serde(rename = "source")]
                source: String,
                #[serde(rename = "lead")]
                lead: String,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Bare(source) => Self::bare(source),
            Wire::Structured { source, lead } => Self::structured(source, lead),
        })
    }
}
