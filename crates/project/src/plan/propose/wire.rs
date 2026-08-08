//! Lead-reconciliation envelope DTOs.
//!
//! These serde DTOs are the source of truth: the judgment-answer schema
//! is generated from them, and the deterministic tail re-parses through them.

use serde::{Deserialize, Serialize};

use super::super::model::{Disagreement, Divergence};
use crate::registry::topology::{Decision, Surface};

/// Reconciliation envelope kind.
///
/// Serialises to the literal `"request"` / `"response"`.
/// [`ProposalRequest`] always carries
/// [`ProposalKind::Request`]; [`ProposalResponse`] always carries
/// [`ProposalKind::Response`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalKind {
    /// Lead catalog and project topology for the agent to group.
    Request,
    /// Agent-authored `slices[]` grouping.
    Response,
}

/// `kind: request` envelope — the lead-centric catalog the agent groups.
///
/// Assembled by the guest `plan author` orchestration: a flat
/// `leads[]` catalog read 1:1 from `discovery.md`, plus the `projects[]`
/// topology the agent binds slices to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProposalRequest {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: ProposalKind,
    /// Project topology — always at least one entry (schema
    /// `minItems: 1`).
    pub projects: Vec<ProjectRef>,
    /// Flat lead catalog: one row per raw `(source, lead)` lead.
    pub leads: Vec<LeadCatalogEntry>,
}

/// One project the agent may bind a response slice to.
///
/// For a workspace this is projected from the committed
/// `.emery/topology.lock`; for a single regular project the
/// CLI synthesises one entry from `project.yaml` (name + resolved
/// target adapter + description) plus the project's own baseline
/// projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProjectRef {
    /// Project name — the value the kernel writes to
    /// `plan.yaml.slices[].project`.
    pub name: String,
    /// The project's target adapter ref in the
    /// `registry::topology::target_ref` grammar: `name@vN` for a
    /// pinned identity (e.g. `omnia@1.0.0`), bare `name` for an
    /// unpinned cache resolve. Resolved on demand by
    /// [`super::resolve_target`] for a slice bound to this project; it
    /// is not written to `plan.yaml` (a slice stores only its
    /// `project`).
    pub target: String,
    /// Single-sentence domain characterisation used by the agent when
    /// more than one project shares a target. Absent stays off the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deterministic baseline surface: the domains this project
    /// owns and a sample of each domain's requirement titles, projected
    /// from `.emery/specs/` through `.emery/topology.lock`. The
    /// agent binds a slice on actual owned behaviour. Empty stays off
    /// the wire (greenfield routes on `description` alone).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface: Vec<Surface>,
    /// Recent per-merge outcome summaries from the project's journal
    /// ledger, newest activity last. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<String>,
    /// Accepted Decision Records projected from `.emery/decisions/`:
    /// the third routing-identity axis — *why* the project is
    /// shaped the way it is, surfaced so the agent can route a slice on
    /// architectural commitment and flag a lead that contradicts an
    /// accepted decision before the operator reviews the plan. Empty
    /// stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Decision>,
    /// Count of accepted decisions elided past the projection cap.
    /// Absent when the catalogue fits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decisions_more: Option<u64>,
    /// Target platforms this project builds for, projected from
    /// `project.yaml.platforms`. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<crate::Platform>,
}

/// One row in the request's flat lead catalog.
///
/// Identity is the `(source, lead)` pair; `lead` repeats
/// across rows when multiple sources surface the same slug. Mirrors a
/// single `discovery.md` [`artifacts::discovery::Lead`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LeadCatalogEntry {
    /// Plan source binding key matching `plan.yaml.sources.<key>`.
    pub source: String,
    /// Discovery lead id surfaced by this source binding.
    pub lead: String,
    /// Reconciliation-grade per-source headline — the primary signal for
    /// agent cross-source grouping. SHOULD name the operation/surface
    /// and its salient constraint so a same-slug lead from another
    /// source can be matched or distinguished on content.
    pub synopsis: String,
    /// Optional agent-authored topic slugs carried from the discovery
    /// lead. Additional grouping context for the agent and the join key
    /// for the decision-contradiction warning; the CLI computes no
    /// grouping from them. Absent (empty) means unclassified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// `kind: response` envelope — the agent's slice grouping.
///
/// Consumed by the guest `plan author` orchestration's judgment tail.
/// The DTO is shape-only; the partition, fan-out, project-binding, and
/// name-derivation invariants are enforced by the projection kernel
/// (`Plan::propose_from`), not by serde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProposalResponse {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: ProposalKind,
    /// The agent's slices, in response order — the kernel writes
    /// `plan.yaml.slices[]` in this order.
    pub slices: Vec<ResponseSlice>,
    /// Plan review prose authored alongside the grouping.
    /// Canonically optional; the generated judgment-answer schema
    /// requires it, and the collapsed `plan author` orchestration
    /// persists it into `change.md` / `discovery.md`. The projection
    /// kernel ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateProse>,
}

/// Plan review prose riding a [`ProposalResponse`]: section bodies
/// only — the orchestrator owns every deterministic frame (`# Change —
/// <name>`, `# Discovery — <name>`, the `##` headings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GateProse {
    /// The `change.md` operator brief body rendered below the
    /// deterministic `# Change — <name>` heading: intent, scope, and
    /// the plan review sections the skill flow authored by hand
    /// (`## Tentative merges`, `## Cross-cutting leads`, `## Likely
    /// divergences` when applicable).
    pub change: String,
    /// The `discovery.md` `## Summary` section body — one-line counts
    /// (`Sources: N. Leads: M.`). Body only, no heading.
    pub discovery_summary: String,
    /// The `discovery.md` `## Source inventory` section body — one row
    /// per bound source under `plan.yaml.sources.<key>`: key, adapter,
    /// path or value. Body only, no heading.
    pub discovery_source_inventory: String,
}

/// One `slices[]` row in a [`ProposalResponse`]: one slice of work
/// carrying its matched `sources[]` inline and its explicit `name`.
///
/// There is no `scope` noun and no kernel fan-out grouping. A body of
/// work that targets more than one project
/// is expressed as multiple ordinary slices (which may legally reference
/// the same lead) joined by `depends-on`; the agent's explicit `name`
/// disambiguates cross-source matches that carry differing slugs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ResponseSlice {
    /// Explicit plan slice name (kebab-case). Required — with `scope`
    /// gone the agent names every slice directly, and the kernel writes
    /// it verbatim to `plan.yaml.slices[].name`.
    pub name: String,
    /// Matched catalog rows, each referenced by `{ source, lead }`
    /// (at most one per source). A lead may appear in more than one
    /// slice — that is fan-out.
    pub sources: Vec<ResponseMember>,
    /// Optional cross-source-match rationale the agent renders into
    /// `change.md` for plan review. Agent-authored and kernel-ignored —
    /// it is not echoed into the journal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Slice names this row depends on. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Project this slice binds to, chosen from the request's
    /// `projects[]`. Optional only when exactly one project exists, in
    /// which case the kernel auto-binds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Agent-flagged slice-level divergence: `likely` when the matched
    /// leads materially disagree. Absent means no divergence. The kernel
    /// carries it onto `plan.yaml.slices[].divergence`; the operator
    /// adjudicates `accepted` / `rejected` later via `emery plan amend
    /// --divergence`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divergence: Option<Divergence>,
    /// The per-field disagreeing values backing a `divergence` flag.
    /// Recommended non-empty (with ≥2 distinct source values each) when
    /// `divergence` is `likely`; the CLI advises on structural
    /// consistency but never decides materiality and never blocks. Empty
    /// stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disagreements: Vec<Disagreement>,
}

/// One matched catalog row referenced by a [`ResponseSlice`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ResponseMember {
    /// Plan source binding key; must match a request catalog row.
    pub source: String,
    /// Discovery lead id; with `source`, must match a request
    /// catalog row.
    pub lead: String,
}
