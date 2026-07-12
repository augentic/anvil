//! Source concerns: the top-level `plan.yaml.sources` binding and the
//! per-entry `(source, lead)` binding.

use serde::{Deserialize, Serialize};

/// One top-level [`super::Plan::sources`] binding.
///
/// Carries the kebab-case source adapter name plus exactly one of
/// `path` (filesystem path or repo location) or `value` (literal
/// payload supplied directly to the adapter, used by the `intent`
/// source).
///
/// On the wire (workflow §Source) the binding is always the structured
/// `{ adapter, path?, value? }` object form. The `oneOf` exclusion
/// between `path` and `value` is enforced by `plan.schema.json` and
/// re-checked at the loader boundary via `crate::schema_gate::validate_plan`.
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
}

/// One `(source, lead)` binding under [`super::Entry::sources`].
///
/// On the wire (workflow §`Slice.sources`) this is either:
///
/// - a bare string `<key>` — shorthand for the structured form
///   `{ source: <key>, lead: <slice.name> }`; used
///   predominantly in the degenerate `intent` case
///   (`sources: [intent]`); or
/// - a structured `{ source, lead }` object.
///
/// Both shapes round-trip byte-identically: the bare shorthand is
/// normalised at parse time into `lead == None`, and `Serialize`
/// emits the same shape the operator authored. Use
/// the internal bare / structured constructors in
/// tests instead of constructing the struct literal directly so the
/// shorthand discipline stays consistent.
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
    pub(crate) fn bare(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lead: None,
        }
    }

    /// Construct the structured form with an explicit lead.
    #[must_use]
    pub(crate) fn structured(source: impl Into<String>, lead: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lead: Some(lead.into()),
        }
    }

    /// The source key this binding references in
    /// [`super::Plan::sources`].
    #[must_use]
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    /// The lead this binding pairs with, falling back to the
    /// owning slice's name for the bare-string shorthand per the
    /// workflow contract §`Slice.sources`.
    #[must_use]
    pub(crate) fn lead<'a>(&'a self, slice_name: &'a str) -> &'a str {
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
