//! Entity-fact DTOs for the `WorkspaceModel` envelope.
//!
//! Each struct is one fact family the `specify lint` indexer
//! produces. Nested keys are kebab-case (`line-start`, `from-path`,
//! `frontmatter-ref`, …) per
//! `specify-cli/schemas/lint/workspace-model.schema.json`; every
//! struct carries `#[serde(rename_all = "kebab-case")]`.

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};

use super::{AdapterAxis, FileKind};
/// `file` fact per the `WorkspaceModel` entity families — produced
/// by the filesystem walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct File {
    /// Project-relative path with forward slashes per the standards-layer contract
    /// §"Stability".
    pub path: String,
    /// Closed file-kind discriminant.
    pub kind: FileKind,
    /// Optional language token inferred from the extension or
    /// supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Optional content digest; populated when an extractor needs
    /// cross-file identity (e.g. canonical SHA checks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// `frontmatter` fact per the `WorkspaceModel` entity families —
/// markdown `---` block extracted then YAML-parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Frontmatter {
    /// Project-relative path of the markdown file the frontmatter
    /// came from.
    pub path: String,
    /// Optional schema id the frontmatter declares (matches the
    /// registered-schema token shape from the registered-schema token shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    /// Parsed YAML field map. Modelled as a [`serde_json::Map`] so
    /// key order round-trips byte-stably; per-key shape is
    /// rule-specific.
    pub fields: JsonMap<String, JsonValue>,
}

/// `markdown_section` fact per the `WorkspaceModel` entity families
/// — markdown structure pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MarkdownSection {
    /// Project-relative path of the markdown file.
    pub path: String,
    /// Markdown heading level (1–6).
    pub level: u8,
    /// Heading text after the leading `#`s, with surrounding
    /// whitespace trimmed.
    pub title: String,
    /// 1-based line of the heading line itself.
    pub line_start: u32,
    /// 1-based last line that belongs to this section (the line
    /// before the next same-or-higher-level heading, or the file's
    /// last line).
    pub line_end: u32,
    /// Number of non-heading body lines under the section.
    pub body_line_count: u32,
}

/// `markdown_link` fact per the `WorkspaceModel` entity families —
/// link scan with fence/comment stripping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MarkdownLink {
    /// Project-relative path of the markdown file containing the
    /// link.
    pub from_path: String,
    /// Verbatim link target as authored (relative path, URL, or
    /// anchor).
    pub to_raw: String,
    /// 1-based line of the link occurrence.
    pub line: u32,
    /// `true` when the target resolves on disk, `false` for broken
    /// references. Absent for off-tree URLs the indexer did not
    /// attempt to resolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolves: Option<bool>,
    /// `true` when the link is an image embed (`![alt](src)`) rather
    /// than a plain `[label](target)` link. Omitted from the wire when
    /// `false` so plain-link facts keep their existing shape.
    #[serde(default, skip_serializing_if = "is_false")]
    pub image: bool,
}

/// `skip_serializing_if` predicate: omit a `bool` field when `false`.
#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde skip predicates take `&T`")]
const fn is_false(value: &bool) -> bool {
    !*value
}

/// `fenced_block` fact — closed fence body extracted for fence-aware evaluators
/// (CORE-037, CORE-017).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FencedBlock {
    /// Project-relative path of the markdown file.
    pub path: String,
    /// 1-based line of the first body line inside the fence.
    pub line_start: u32,
    /// 1-based line of the closing fence delimiter.
    pub line_end: u32,
    /// Info string from the opening fence (`json`, `text`, …); empty when absent.
    pub lang: String,
    /// Fence body lines joined with `\n` (excludes opening/closing delimiters).
    pub body: String,
}

/// `symlink` fact per the `WorkspaceModel` entity families.
///
/// Recorded but not traversed under the consumer file scan contract;
/// the framework profile additionally follows the link and records
/// the resolved canonical endpoint in [`Self::resolved_target`] per
/// the standards-layer contract §F1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Symlink {
    /// Project-relative path of the symlink itself.
    pub path: String,
    /// Symlink target as recorded by the filesystem (may be
    /// relative or absolute).
    pub target: String,
    /// `true` when the link target does not exist on disk.
    pub broken: bool,
    /// Project-relative path of the resolved endpoint after
    /// canonicalisation. Populated only by the framework scan
    /// profile per §F1 (`follow` mode); absent under the consumer
    /// profile and absent for broken links the walker could not
    /// resolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_target: Option<String>,
}

/// `skill` fact per the `WorkspaceModel` entity families —
/// extracted from `plugins/**/SKILL.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Skill {
    /// Skill name (matches the `name:` frontmatter field).
    pub name: String,
    /// Project-relative path of the `SKILL.md` file.
    pub path: String,
    /// Owning plugin slug (the directory under `plugins/`).
    pub plugin: String,
    /// Path back to the originating [`Frontmatter`] fact so
    /// consumers can join through the frontmatter table.
    pub frontmatter_ref: String,
    /// Number of non-frontmatter body lines under the skill body.
    /// Populated by the framework profile; absent when the indexer
    /// did not compute it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_line_count: Option<u32>,
}

/// Closed brief-scope discriminant: parent brief vs phase sub-brief.
///
/// The two carry different size budgets (a rule supplies the caps via
/// `config`), so the scope is the selector a `cardinality` brief
/// metric narrows on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BriefScope {
    /// Parent orchestrator brief at
    /// `adapters/<axis>/<adapter>/briefs/<operation>.md`.
    Parent,
    /// Phase sub-brief under
    /// `adapters/<axis>/<adapter>/briefs/{build,extract}/**/*.md`.
    Phase,
}

/// `brief` fact per the `WorkspaceModel` entity families —
/// extracted from `adapters/**/briefs/**/*.md` under the framework
/// profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Brief {
    /// Project-relative path of the brief markdown file.
    pub path: String,
    /// Owning adapter axis (`sources` xor `targets`).
    pub axis: AdapterAxis,
    /// Owning adapter slug (the directory under
    /// `adapters/{sources,targets}/`).
    pub adapter: String,
    /// Operation slug for the brief. For a parent brief this is the
    /// brief stem (`extract`, `survey`, `shape`, `build`, `merge`);
    /// for a phase sub-brief it is the phase directory (`build`,
    /// `extract`).
    pub operation: String,
    /// Parent vs phase classification — the size-budget selector.
    pub scope: BriefScope,
    /// `##` heading titles found in the body, in document order
    /// after fence and HTML-comment stripping.
    pub sections: Vec<String>,
    /// Total non-empty markdown body lines (frontmatter excluded
    /// when present).
    pub body_line_count: u32,
}

/// `adapter_manifest` fact per the `WorkspaceModel` entity families
/// — extracted from `adapters/{sources,targets}/**/adapter.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AdapterManifest {
    /// Closed `sources` / `targets` discriminant.
    pub axis: AdapterAxis,
    /// Adapter name from `adapter.yaml`.
    pub name: String,
    /// Project-relative path of the `adapter.yaml` file.
    pub path: String,
    /// Optional manifest version; absent when the adapter does not
    /// pin one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Operation slugs declared as the `briefs:` map keys in the
    /// manifest body. Empty when the manifest omits the field or
    /// declares an empty map. Consumed by the `kind: set-coverage`
    /// interpreter via the `adapter-briefs` discriminator (subset or
    /// exact mode) to detect manifests whose `briefs.keys()` do
    /// not match the rule-supplied expected operation set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brief_keys: Vec<String>,
}

/// `adapter_dir` fact per the `WorkspaceModel` entity families.
///
/// Produced by the framework profile's dedicated adapter-directory pass
/// (see [`crate::lint::index::adapter_dir`]): one fact per immediate
/// child directory under `adapters/sources/` and `adapters/targets/`,
/// regardless of whether the directory carries an `adapter.yaml`
/// manifest. Symlinked entries are skipped. Directories are not files,
/// so this family is kept out of [`crate::lint::WorkspaceModel::files`]
/// and carries zero blast radius to other rules' candidate sets.
///
/// The family is the *source* side of the `kind: cross-reference`
/// relational join: a `cross-reference` hint flags any [`Self::path`]
/// that has no corresponding [`AdapterManifest`] (joined on the
/// manifest's containing directory). Listing every adapter directory —
/// not just the orphans — keeps the join itself in the evaluator rather
/// than baked into this extractor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AdapterDir {
    /// Project-relative path of the adapter directory itself (e.g.
    /// `adapters/targets/vectis`), with forward slashes and no trailing
    /// separator. The join key against the containing directory of each
    /// [`AdapterManifest`] path.
    pub path: String,
    /// Closed `sources` / `targets` discriminant matching the parent
    /// directory under `adapters/`.
    pub axis: AdapterAxis,
    /// Directory name (the final path segment, e.g. `vectis`).
    pub name: String,
}

/// `scenario` fact per the `WorkspaceModel` entity families.
///
/// Produced by the framework profile's dedicated scenario pass (see
/// [`crate::lint::index::scenario`]) over the opt-in scenario roots
/// (`evals/scenarios/*.md`, `adapters/targets/*/tests/**`, plugin
/// skill fixtures). Scenario files live partly under the un-indexed
/// `evals/` tree, so the pass walks the roots itself and emits a
/// dedicated fact family kept OUT of [`crate::lint::WorkspaceModel::files`]
/// — no other rule's candidate set changes. The full [`Self::fields`]
/// map carries the parsed frontmatter so a `kind: schema` hint can
/// validate it whole-tree; the projected `id` / `stages` /
/// `expected_artifacts` / `body_id` views serve the per-field hints
/// (`kind: unique` over `id` today).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scenario {
    /// Project-relative path of the scenario markdown file.
    pub path: String,
    /// Frontmatter `id` value when present and a string; absent when
    /// the frontmatter omits it or it is not a scalar string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Frontmatter `stages` tokens, in declared order; empty when the
    /// field is absent or not a string array.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<String>,
    /// Frontmatter `expected-artifacts` paths; empty when absent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_artifacts: Vec<String>,
    /// Body `Scenario ID:` line value when present; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_id: Option<String>,
    /// Parsed YAML frontmatter field map. Modelled as a
    /// [`serde_json::Map`] so a `kind: schema` hint can validate it
    /// directly; empty when the opted-in frontmatter failed to parse.
    pub fields: JsonMap<String, JsonValue>,
}

/// `ignore_directive` fact per the `WorkspaceModel` entity families.
///
/// Produced by the indexer pass that recognises
/// `specify-ignore: <RULE-ID> — <rationale>` comments across the
/// closed comment-style list (C-family, hash, HTML, SQL/Lua).
/// Malformed directives (missing or empty rationale) are still
/// emitted with `rationale = None` so the directive-validation pass
/// can synthesise `UNI-022` / `UNI-023` findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct IgnoreDirective {
    /// Project-relative path of the file containing the directive
    /// comment.
    pub path: String,
    /// 1-based line of the directive comment itself.
    pub line: u32,
    /// Rule id named by the directive. Not pattern-pinned so
    /// malformed ids surface as `UNI-023` candidates downstream.
    pub rule_id: String,
    /// Verbatim rationale text from the directive. `None` when the
    /// directive lacked a rationale (the directive-validation pass
    /// emits `UNI-022` for that case).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// 1-based line the directive applies to. Inline trailing
    /// directives target their own line; block-leading directives
    /// target the next non-blank, non-comment line; directives at
    /// end-of-file with no following target line use the line
    /// number one past the file's last line so the validation pass
    /// can detect orphan placement.
    pub target_line: u32,
    /// Raw directive comment text as captured by the indexer,
    /// including delimiters (e.g. `// specify-ignore: …`,
    /// `/* specify-ignore: … */`).
    pub raw: String,
}
