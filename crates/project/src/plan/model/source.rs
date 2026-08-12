//! Source concerns: the top-level `plan.yaml.sources` binding and the
//! per-entry `(source, lead)` binding.

use serde::{Deserialize, Serialize};

use crate::adapter::{AdapterSelector, FIRST_PARTY_NAMESPACE};
use crate::snapshot::SnapshotId;

/// One top-level [`super::Plan::sources`] binding.
///
/// Carries the kebab-case source adapter name plus exactly one of
/// `path` or `value` (mutually exclusive; readers treat `path` as
/// taking precedence), and — once plan author closes the source set —
/// the content-addressed tree identity of that input (`cid`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SourceBinding {
    /// Kebab-case source-adapter name (e.g. `intent`, `documentation`,
    /// `typescript`, `screenshots`).
    pub adapter: String,
    /// Optional exact semver pin for the bound source adapter; an
    /// omitted `version` means the single installed identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<semver::Version>,
    /// Filesystem path or repo location the adapter binds against.
    /// Mutually exclusive with `value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Literal value supplied directly to the adapter (e.g. the
    /// operator brief text for `intent`). Mutually exclusive with
    /// `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Content-addressed identity of the bound source tree
    /// (`sha256:…`). Wire field `cid` (RFC-86 D25). Absent until plan
    /// author closes the source set; refinement records these pins in
    /// the manifest's `inputs.sources`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cid: Option<SnapshotId>,
}

impl SourceBinding {
    /// The typed adapter selector this binding resolves through: the
    /// bare development name for an unpinned binding, or the
    /// first-party package pin when `version` is set (the binding
    /// schema carries no namespace field; pins are implicitly
    /// `emery:`).
    #[must_use]
    pub fn selector(&self) -> AdapterSelector {
        self.version.as_ref().map_or_else(
            || AdapterSelector::Bare {
                name: self.adapter.clone(),
            },
            |version| AdapterSelector::Package {
                namespace: FIRST_PARTY_NAMESPACE.to_string(),
                name: self.adapter.clone(),
                version: version.clone(),
            },
        )
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
    /// Lead id from `discovery.md`, resolved within `source`.
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
