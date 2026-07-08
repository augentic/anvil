//! `WorkspaceModel` DTOs.
//!
//! The model is the deterministic, versioned snapshot of project facts
//! the `specify lint` indexer produces once per run. Internal
//! execution artifact only — never persisted in the working tree and
//! not operator-facing.
//!
//! The DTOs round-trip through
//! `specify_schema::WORKSPACE_MODEL_JSON_SCHEMA`; the envelope's
//! `version: 1` discriminant pins the wire shape, and breaking indexer
//! output bumps it. Top-level envelope keys are `snake_case`; nested
//! entity-fact keys are kebab-case (each entity struct carries
//! `#[serde(rename_all = "kebab-case")]`). Every array on the envelope
//! is always serialised, even when empty, so JSON consumers can rely
//! on the full set of fact families existing.

#![allow(
    clippy::module_name_repetitions,
    reason = "The public schema uses the `WorkspaceModel` envelope name; the surrounding `model` module is the navigational home for the DTO layer."
)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod facts;

pub use facts::{
    FencedBlock, File, Frontmatter, IgnoreDirective, MarkdownLink, MarkdownSection, Scenario,
    Skill, Symlink,
};

/// Type-level pin of the `WorkspaceModel` envelope version.
///
/// Serialises to the integer `1` and refuses to deserialise any
/// other value per the standards-layer contract §"`WorkspaceModel`". Modelled as a unit
/// struct so the [`Default`] / [`Eq`] / [`Hash`] / [`Ord`]
/// derivations propagate to [`WorkspaceModel`] without further
/// plumbing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceModelVersion;

impl Serialize for WorkspaceModelVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for WorkspaceModelVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported WorkspaceModel version: {value} (only v1 is supported)"
            )))
        }
    }
}

/// Closed file-kind discriminant per `WorkspaceModel` file scan. `binary` when the
/// first 8 `KiB` of the file contains a `NUL` byte; otherwise `text`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    /// Decoded as UTF-8 text by the indexer (replacement bytes
    /// allowed per `WorkspaceModel` file scan).
    Text,
    /// Treated as opaque bytes; regex hints skip files of this kind.
    Binary,
}

/// Closed scan-profile discriminant per the standards-layer contract §"`WorkspaceModel`"
/// extraction inputs.
///
/// `project` scans a downstream consumer project's files; `framework`
/// scans this framework repo's authoring artifacts. The two profiles
/// select different extractor sets in the indexer.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ScanProfile {
    /// Project scan: the files produced with Specify in a downstream
    /// consumer project.
    #[default]
    Project,
    /// Framework repo scan: this repo's authoring artifacts.
    Framework,
}

/// v1 `WorkspaceModel` envelope per the standards-layer contract §"`WorkspaceModel`" and
/// §"Schema location".
///
/// Important: per the standards-layer contract §"Persistence and query (v1 decision)" the
/// model is **not** persisted in the working tree and is **not** an
/// operator-facing Specify artifact in v1. v1 ships
/// `specify lint --dump-model` only; a `workspace-model.v1.json` in the
/// out-of-tree project cache and
/// `specify model query <selector>` are reserved surfaces.
///
/// Top-level keys are `snake_case` to match the schema under
/// `specify/schemas/lint/workspace-model.schema.json`; nested
/// entity facts carry their own kebab-case rename.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceModel {
    /// Version discriminant pinned to `1` per the standards-layer contract
    /// §"`WorkspaceModel`".
    pub version: WorkspaceModelVersion,
    /// Scan root; project root for `consumer`, the framework repo root
    /// (`--framework-root`, default cwd) for `framework`.
    pub project_dir: String,
    /// Profile selector that controls which extractors run.
    pub scan_profile: ScanProfile,
    /// Optional narrow path list; empty means the indexer performs
    /// a full profile-specific scan by the file scan contract.
    pub artifact_paths: Vec<String>,
    /// Optional language tokens supplied by the caller or inferred
    /// from paths.
    pub languages: Vec<String>,
    /// `file` facts from the filesystem walk.
    pub files: Vec<File>,
    /// `frontmatter` facts from markdown `---` block extraction
    /// plus YAML parse.
    pub frontmatter: Vec<Frontmatter>,
    /// `markdown_section` facts from the markdown structure pass.
    pub markdown_sections: Vec<MarkdownSection>,
    /// `markdown_link` facts from the fence-aware link scan.
    pub markdown_links: Vec<MarkdownLink>,
    /// `symlink` facts; recorded but not traversed by the file scan contract.
    pub symlinks: Vec<Symlink>,
    /// `skill` facts from `plugins/**/SKILL.md`.
    pub skills: Vec<Skill>,
    /// `ignore_directive` facts from the directive indexer.
    /// Optional in v1 envelopes per the schema; the producer always
    /// serialises the array so consumers see one consistent wire
    /// shape.
    #[serde(default)]
    pub ignore_directives: Vec<IgnoreDirective>,
    /// `fenced_block` facts from the fence-aware markdown pass.
    #[serde(default)]
    pub fenced_blocks: Vec<FencedBlock>,
    /// `scenario` facts from the dedicated scenario discovery pass over
    /// the opt-in scenario roots under the framework scan profile.
    /// Optional in v1 envelopes; producers omit the field when empty so
    /// the project profile's wire shape is unchanged. Kept out of
    /// [`Self::files`] so no other rule's candidate set changes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<Scenario>,
}
