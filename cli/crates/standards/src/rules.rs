//! Codex DTOs and runtime wire types per the rules contract.
//!
//! Provides the typed Rust shapes that round-trip cleanly through the
//! schemas embedded under `schemas/rules/` and `schemas/diagnostics/`:
//!
//! - [`Rule`] / [`Deprecated`] / [`Applicability`] /
//!   [`RuleHint`] / [`Reference`] are the parsed-frontmatter
//!   shape used by the CH-11 frontmatter parser. Field names are
//!   kebab-case at every nesting level (`lint-mode`,
//!   `rule-hints`, `replaced-by`); the parser performs the
//!   `snake_case -> kebab-case` lift on the raw markdown side so the
//!   in-memory shape matches the wire shape.
//! - [`ResolvedRules`] / [`ResolvedRule`] are the export envelope
//!   emitted by `specify rules export --format json` (CH-17). They
//!   add resolver-only fields ([`Origin`], [`PathRoot`], `path`,
//!   `body`) on top of the codex-rule shape.
//!
//! Structured diagnostic types ([`specify_diagnostics::Diagnostic`],
//! renderers, fingerprint helpers) live in the neutral
//! [`specify_diagnostics`] leaf — import them from there directly.
//!
//! Severity comparator order is `Critical < Important < Suggestion <
//! Optional` and origin order is `Target < Source < Shared <
//! Unknown`, matching `ResolvedRules` export contract
//! §"Ordering". The closed enums are declared in the comparator order
//! so the derived [`Ord`] picks up the contract-defined sort sequence.

#![allow(
    clippy::module_name_repetitions,
    reason = "The public wire contract uses the names Rule and ResolvedRules; renaming to avoid the codex prefix would obscure the schema mapping."
)]

pub mod parse;
pub mod resolve;

pub use parse::{ParseError, parse_rule, parse_rule_file};
pub use resolve::{
    ResolveError, ResolveInputs, ResolvedRuleEntry, build_resolved_rules, filter,
    map_resolve_error, resolve, sort_resolved,
};
use serde::{Deserialize, Serialize};
use specify_diagnostics::Severity;

/// Resolver origin tier per `ResolvedRules` export contract.
///
/// Variants are declared in the documented sort order (`target`,
/// `source`, `shared`, `core`, `unknown`) so the derived [`Ord`]
/// yields the contract-defined comparator. Wire spelling uses
/// kebab-case rendered from the variant identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// Target-adapter overlay (`adapters/targets/<name>/rules/`).
    Target,
    /// Source-adapter overlay (`adapters/sources/<name>/rules/`).
    Source,
    /// Shared rules (`adapters/shared/rules/...`).
    Shared,
    /// Core pack overlay (`adapters/shared/rules/core/`). Excluded
    /// from consumer exports unless `--include-core` is set.
    Core,
    /// Indexer fallback: cache rule files whose path does not match
    /// the closed adapter-shape probe in `infer_origin` under
    /// [`crate::lint::index`].
    Unknown,
}

/// Anchor for the rule `path` field in a [`ResolvedRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathRoot {
    /// Path is relative to the rules root (shared rules and rules-root
    /// fallback overlays).
    RulesRoot,
    /// Path is relative to the project directory (project-local
    /// overlays).
    ProjectDir,
    /// Path is relative to the out-of-tree project cache root
    /// (`<project-cache>/`); cached adapter-rule overlays.
    Cache,
}

/// How a rule is expected to be reviewed. Wire spelling is
/// kebab-case (`deterministic`, `model-assisted`, `hybrid`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LintMode {
    /// Rule is fully expressed as deterministic hints.
    Deterministic,
    /// Rule needs an SLM/LLM scorer.
    ModelAssisted,
    /// Mix of deterministic + model-assisted signals.
    Hybrid,
}

/// Closed v1 deterministic-hint kind enum.
///
/// After C17 every kind is executable: `path-pattern`, `regex`,
/// `schema`, `tool`, `reference-resolves`, `unique`, `set-coverage`,
/// `cardinality`, `constant-eq`,
/// `fenced-block`, `presence`, `field-grammar`, `cross-reference`,
/// and `cli-contract`. (`content-digest-eq` was removed when its last
/// rule consumers retired — agent-teams overlays are symlink-only and
/// README restatements became links.)
/// No kind is reserved. (Whole-tree namespace-ownership runs through the
/// `rules` WASI tool via `kind: tool`, not a dedicated hint kind.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HintKind {
    /// Glob pattern matched against artifact paths.
    PathPattern,
    /// Regular expression matched against artifact bytes.
    Regex,
    /// Validate against an embedded JSON Schema.
    Schema,
    /// Invoke a declared WASI tool.
    Tool,
    /// Assert that some field across a candidate set is unique
    /// (v1 fact-family selector: `skill`; field in `config: { field }`).
    Unique,
    /// Every reference resolves (v1 source discriminator: `markdown-link`).
    ReferenceResolves,
    /// Assert that the values some candidate file declares cover a
    /// closed expected set (v1 source discriminators: `adapter-briefs`,
    /// `skill-allowed-tools`; the expected set rides `config`). The
    /// `adapter-briefs` source reads `config: { mode }` to choose the
    /// one-sided `subset` (default) or two-sided `exact` comparison.
    SetCoverage,
    /// Assert that some countable property of a candidate is within
    /// configured bounds (v1 metric selectors: `skill-body-line-count`,
    /// `markdown-h2-section-body-line-count`, `brief-*-body-line-count`;
    /// the cap rides `config: { max }`).
    Cardinality,
    /// Assert that an extracted field on a candidate fact equals a
    /// configured constant (v1 source discriminators:
    /// `adapter-manifest-field`, `skill-name-plugin-prefix`; the value
    /// rides `config`).
    ConstantEq,
    /// Fence-aware body predicate over [`crate::lint::FencedBlock`] facts
    /// (`skill-envelope-json-in-body`, …).
    FencedBlock,
    /// Assert that a required artifact is present (v1 mechanism
    /// selectors: `frontmatter` — candidate files absent from the
    /// frontmatter fact family; `file` — a single required path in
    /// `config: { path }`; `markdown-section` — skills over a
    /// `config: { when: { metric, min } }` threshold lacking the
    /// `config: { title, level }` section; `directory-index` —
    /// directories matching a `config: { roots }` glob with at least
    /// `config: { min-files }` files beneath them but no
    /// `config: { index }` file inside them).
    Presence,
    /// Assert that a candidate's named frontmatter field obeys a
    /// token / first-word grammar (v1 mechanism modes: `field-tokens` —
    /// every whitespace token of `config: { field }` matches the
    /// `config: { token-pattern }` regex; `field-first-word` — the first
    /// alphabetic word of `config: { field }` is in the
    /// `config: { allowed }` list).
    FieldGrammar,
    /// Assert that every element of a source fact family has a
    /// corresponding element in a target fact family (a relational
    /// set-difference join); flag the unmatched source items. `value`
    /// selects the source family, `config: { target }` the target
    /// family; the join key is the per-family mechanism (v1 source
    /// selector: `adapter-dir` joined against the `adapter-manifest`
    /// target on the manifest's containing directory).
    CrossReference,
    /// Check documentation against the binary-injected
    /// [`crate::lint::contract::CliContract`] (v1 mechanism selectors:
    /// `invocations` — `specify …` command lines in fenced blocks /
    /// inline code walk the contract's verb tree; `event-ids` —
    /// dotted-kebab inline code spans and configured JSON fields,
    /// gated to the contract's own event-id families, must be known
    /// journal event ids; `error-codes` — configured JSON fields must
    /// be known error discriminants; `test-citations` — `tests/…`
    /// inline code spans and configured link-prefix targets must exist
    /// in the contract's build-time tests inventory. Exclusions ride
    /// `config`).
    CliContract,
}

/// Inclusive narrowing filter — all populated dimensions match (AND).
///
/// Per the wire schema at least one dimension must be populated; the
/// Rust shape represents every dimension as `Option` and the
/// resolver enforces the at-least-one rule when it loads the rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Applicability {
    /// Adapter names this rule applies to (with optional `@v<major>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapters: Option<Vec<String>>,
    /// Language tokens this rule applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,
    /// Artifact categories this rule applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<String>>,
    /// Project-relative path globs this rule applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
}

/// One rule-hint entry on a rule (executable by the deterministic hint interpreter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RuleHint {
    /// Hint kind discriminator.
    pub kind: HintKind,
    /// Hint payload, interpreted by a future validator or review tool.
    pub value: String,
    /// Optional human explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional per-kind configuration (schema-validated; interpreted by the matching eval arm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// One reference entry on a rule. Schema requires `label` plus
/// at least one of `url` / `path`; the resolver enforces the
/// `anyOf` rule when it loads the rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Reference {
    /// Short display label.
    pub label: String,
    /// HTTP(S) reference URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Repository-relative reference path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Deprecation metadata for a rule.
///
/// Marks a rule as deprecated while preserving the stable id for
/// historical citations. Wire key for `replaced_by` is the
/// kebab-case `replaced-by` per `ResolvedRules` export contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Deprecated {
    /// Human-readable reason the rule is deprecated.
    pub reason: String,
    /// Replacement rule id when there is a direct successor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
}

/// Parsed rule (frontmatter + body) on the wire-kebab shape.
///
/// The CH-11 frontmatter parser owns the snake-to-kebab lift on the
/// markdown authoring side; this struct represents the post-lift
/// shape so it round-trips cleanly through the resolved-export
/// schema (the resolver-only fields — `origin`, `path-root`, `path`
/// — live on [`ResolvedRule`], not here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Rule {
    /// Stable rule identifier (e.g. `UNI-014`).
    pub id: String,
    /// Short human-readable rule title.
    pub title: String,
    /// Default review severity for findings citing this rule.
    pub severity: Severity,
    /// One-sentence trigger condition.
    pub trigger: String,
    /// How the rule is expected to be reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint_mode: Option<LintMode>,
    /// Inclusive narrowing filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<Applicability>,
    /// Optional deterministic-hint list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_hints: Option<Vec<RuleHint>>,
    /// Optional reference list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,
    /// Optional deprecation metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<Deprecated>,
    /// Verbatim markdown body (everything after the closing
    /// frontmatter delimiter), including section headings such as
    /// `## Rule`.
    #[serde(default)]
    pub body: String,
}

/// Read-only resolved view of shared, source-adapter, and
/// target-adapter rules. Wire envelope emitted by
/// `specify rules export --format json` per the rules contract
/// §"Resolved rules export".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ResolvedRules {
    /// Envelope version. v1 pins this to 1.
    pub version: u32,
    /// Resolved target-adapter name, optionally `@v<major>`.
    pub target_adapter: String,
    /// Source-adapter names bound to the export context.
    pub source_adapters: Vec<String>,
    /// Ordered rule list, `(non-deprecated, severity, origin,
    /// rule-id)` per `ResolvedRules` export contract §"Ordering".
    pub rules: Vec<ResolvedRule>,
}

/// One resolved rule entry inside a [`ResolvedRules`]. Carries every
/// codex-rule field plus resolver-only fields (`origin`,
/// `path-root`, `path`, `body`).
///
/// The `rule_id` field is named distinctly from [`Rule::id`] so
/// the wire shape stabilises on `rule-id` per the wire contract; semantically
/// the value is the same identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ResolvedRule {
    /// Rule id (e.g. `UNI-014`).
    pub rule_id: String,
    /// Short human-readable rule title.
    pub title: String,
    /// Default review severity for findings citing this rule.
    pub severity: Severity,
    /// One-sentence trigger condition.
    pub trigger: String,
    /// How the rule is expected to be reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint_mode: Option<LintMode>,
    /// Inclusive narrowing filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<Applicability>,
    /// Optional deterministic-hint list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_hints: Option<Vec<RuleHint>>,
    /// Optional reference list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,
    /// Resolver origin tier.
    pub origin: Origin,
    /// Anchor for the rule `path` field.
    pub path_root: PathRoot,
    /// Path to the rule markdown file, relative to `path-root`.
    pub path: String,
    /// Verbatim markdown body (everything after the closing `---`
    /// frontmatter delimiter), including section headings such as
    /// `## Rule`.
    pub body: String,
    /// Deprecation metadata, or `null` when the rule is active.
    #[serde(default)]
    pub deprecated: Option<Deprecated>,
}

#[cfg(test)]
mod tests;
