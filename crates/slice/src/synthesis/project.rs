//! Projection kernel — `project(response) -> SliceModel`.
//!
//! The agent decides which requirements exist and how claims reconcile.
//! The kernel deterministically re-derives ids, authority outcomes,
//! sources, and header fields rather than trusting agent-supplied values.
//!
//! Projection is pure and target-independent. It rejects only invalid
//! evidence anchors, cross-references, and closed-form ids.
//!
//! Requirement ids are slice-local until target-wave commit (RFC-86 D5):
//! synthesize mints `REQ-001..N` in declaration order without consulting
//! baseline numbers. Each `MODIFIED` row keeps its agent-authored
//! `baseline-id` and records a digest of that baseline requirement body.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::evidence::{AuthorityClass, ClaimKind};
use diagnostics::digest::sha256_hex;
use error::{Error, Result};

use crate::model::{ModelClaim, ModelRequirement, SliceModel};
use crate::synthesis::authority::{ClaimRef, resolve};
use crate::synthesis::baseline::{BaselineIndex, DomainKind};

/// Domain used when a requirement has no explicit owner.
const DEFAULT_DOMAIN: &str = "default";

/// Header fields stamped onto persisted `model.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionHeader {
    /// Stored schema version (`const: 1` today).
    pub version: u32,
    /// Slice name (kebab-case).
    pub slice: String,
    /// Bound project name, when one is bound.
    pub project: Option<String>,
}

/// Project an agent synthesis response into a persisted [`SliceModel`].
///
/// Validation and derivation run in this order, returning the first
/// violation:
///
/// 1. Anchor claim ids and kinds to Evidence.
/// 2. Assign slice-local requirement ids (and `MODIFIED` digests).
/// 3. Derive authority outcomes and source order.
/// 4. Stamp the header.
/// 5. Validate cross-references, uniqueness, and id grammar.
///
/// # Errors
///
/// Returns the first validation failure.
pub fn project(
    mut model: SliceModel, header: ProjectionHeader, authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
    evidence_claims: &BTreeMap<(String, String), ClaimKind>, baseline_index: &BaselineIndex,
) -> Result<SliceModel> {
    check_claim_anchors(&model, evidence_claims)?;

    let mut allocator = IdAllocator::new();

    for requirement in &mut model.requirements {
        let domain = requirement_domain(requirement);
        let assigned = assign_requirement_id(requirement, &domain, baseline_index, &mut allocator)?;
        requirement.id = Some(assigned.id);
        requirement.baseline_digest = assigned.baseline_digest;
        if requirement.baseline_digest.is_none() {
            requirement.baseline_id = None;
        }

        let claim_refs: Vec<ClaimRef> = requirement
            .claims
            .iter()
            .map(|claim| ClaimRef {
                source: claim.source.clone(),
                id: claim.id.clone(),
                kind: claim.kind,
            })
            .collect();
        let resolution = resolve(&claim_refs, authority, overrides, requirement.agreement);

        let sources = rendered_sources(&requirement.claims, authority, overrides);
        requirement.status = Some(resolution.status);
        for (claim, winner) in requirement.claims.iter_mut().zip(resolution.winners) {
            claim.winner = winner;
        }
        requirement.sources = sources;
    }

    model.version = Some(header.version);
    model.slice = Some(header.slice);
    model.project = header.project;

    check_cross_refs(&model)?;
    check_unique_ids(&model)?;
    check_id_grammar(&model)?;

    Ok(model)
}

fn requirement_domain(requirement: &ModelRequirement) -> String {
    requirement.domain.clone().unwrap_or_else(|| DEFAULT_DOMAIN.to_string())
}

/// Slice-scoped id allocation — declaration order, ignoring baseline numbers.
struct IdAllocator {
    next: u32,
}

impl IdAllocator {
    const fn new() -> Self {
        Self { next: 1 }
    }

    fn allocate(&mut self) -> String {
        let id = format!("REQ-{:03}", self.next);
        self.next = self.next.saturating_add(1);
        id
    }
}

/// One projected requirement identity.
struct Assigned {
    /// Slice-local `REQ-NNN`.
    id: String,
    /// Digest of the baseline body when this row is `MODIFIED`.
    baseline_digest: Option<String>,
}

fn assign_requirement_id(
    requirement: &ModelRequirement, domain: &str, baseline_index: &BaselineIndex,
    allocator: &mut IdAllocator,
) -> Result<Assigned> {
    if let Some(baseline_id) = requirement.baseline_id.as_deref() {
        if !matches_grammar(baseline_id, "REQ-") {
            return Err(id_grammar_error("baseline_id", baseline_id));
        }
        if baseline_index.domain_kind(domain) != DomainKind::Modified {
            return Err(Error::validation_failed(
                "slice-model-baseline-id-orphan",
                "baseline-id is only valid in a domain with an existing baseline spec",
                format!(
                    "baseline-id '{baseline_id}' requires a modified domain baseline for '{domain}'"
                ),
            ));
        }
        if !baseline_index.is_baseline_req(domain, baseline_id) {
            return Err(Error::validation_failed(
                "slice-model-baseline-id-orphan",
                "baseline_id names an existing baseline requirement in a modified domain",
                format!("baseline_id '{baseline_id}' is not in the baseline for domain '{domain}'"),
            ));
        }
        let body = baseline_index.body(domain, baseline_id).ok_or_else(|| {
            Error::validation_failed(
                "slice-model-baseline-id-orphan",
                "baseline_id names an existing baseline requirement in a modified domain",
                format!("baseline_id '{baseline_id}' has no body in domain '{domain}'"),
            )
        })?;
        return Ok(Assigned {
            id: allocator.allocate(),
            baseline_digest: Some(format!("sha256:{}", sha256_hex(body.as_bytes()))),
        });
    }

    Ok(Assigned {
        id: allocator.allocate(),
        baseline_digest: None,
    })
}

fn check_unique_ids(model: &SliceModel) -> Result<()> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for requirement in &model.requirements {
        let id = requirement.id.as_deref().unwrap_or_default();
        if !seen.insert(id) {
            return Err(Error::validation_failed(
                "slice-model-id-duplicate",
                "projected requirement ids are unique across the slice",
                format!("duplicate projected requirement id '{id}'"),
            ));
        }
    }
    Ok(())
}

fn check_claim_anchors(
    model: &SliceModel, evidence_claims: &BTreeMap<(String, String), ClaimKind>,
) -> Result<()> {
    for requirement in &model.requirements {
        for claim in &requirement.claims {
            let key = (claim.source.clone(), claim.id.clone());
            match evidence_claims.get(&key) {
                None => {
                    return Err(Error::validation_failed(
                        "slice-model-source-orphan",
                        "every claim references an Evidence (source, id)",
                        format!(
                            "claim ({}, {}) anchors no on-disk Evidence claim",
                            claim.source, claim.id
                        ),
                    ));
                }
                Some(recorded) if *recorded != claim.kind => {
                    return Err(Error::validation_failed(
                        "slice-model-claim-kind-mismatch",
                        "a claim kind matches the kind recorded in Evidence",
                        format!(
                            "claim ({}, {}) is kind '{}' but Evidence records '{}'",
                            claim.source, claim.id, claim.kind, recorded
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

fn check_cross_refs(model: &SliceModel) -> Result<()> {
    let projected: BTreeSet<&str> =
        model.requirements.iter().filter_map(|req| req.id.as_deref()).collect();
    for task in &model.tasks {
        for satisfied in &task.satisfies {
            if !projected.contains(satisfied.as_str()) {
                return Err(Error::validation_failed(
                    "slice-model-cross-ref-orphan",
                    "every tasks[].satisfies REQ reference names a projected requirement",
                    format!("task '{}' satisfies unknown requirement '{satisfied}'", task.id),
                ));
            }
        }
    }
    Ok(())
}

/// Task ids require a gate because, unlike requirement ids, they remain
/// agent-authored.
fn check_id_grammar(model: &SliceModel) -> Result<()> {
    for requirement in &model.requirements {
        let id = requirement.id.as_deref().unwrap_or_default();
        if !matches_grammar(id, "REQ-") {
            return Err(id_grammar_error("requirement", id));
        }
    }
    for task in &model.tasks {
        if !matches_grammar(&task.id, "TASK-") {
            return Err(id_grammar_error("task", &task.id));
        }
    }
    Ok(())
}

fn matches_grammar(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix)
        .is_some_and(|digits| digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn id_grammar_error(kind: &str, id: &str) -> Error {
    Error::validation_failed(
        "slice-model-id-grammar",
        "REQ and TASK ids match their closed three-digit grammar",
        format!("{kind} id '{id}' does not match its closed three-digit grammar"),
    )
}

/// Order distinct sources by their strongest effective authority.
///
/// Stable sorting preserves first appearance for ties.
fn rendered_sources(
    claims: &[ModelClaim], authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for claim in claims {
        if !order.iter().any(|seen| seen == &claim.source) {
            order.push(claim.source.clone());
        }
    }
    order.sort_by(|left, right| {
        source_level(right, claims, authority, overrides)
            .cmp(&source_level(left, claims, authority, overrides))
    });
    order
}

fn source_level(
    source: &str, claims: &[ModelClaim], authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
) -> Level {
    claims
        .iter()
        .filter(|claim| claim.source == source)
        .map(|claim| effective_level(source, claim.kind, authority, overrides))
        .max()
        .unwrap_or(Level::Class(0))
}

fn effective_level(
    source: &str, kind: ClaimKind, authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
) -> Level {
    if overrides.get(&kind).map(String::as_str) == Some(source) {
        return Level::Override;
    }
    let class = authority.get(source).copied().unwrap_or(AuthorityClass::Behaviour);
    Level::Class(class_rank(class))
}

/// Variant order makes every override outrank every document class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Ranked document-level authority.
    Class(u8),
    /// A per-slice `authority-override` forced this source to win.
    Override,
}

/// Rank authority independently of enum declaration order.
const fn class_rank(class: AuthorityClass) -> u8 {
    match class {
        AuthorityClass::Behaviour => 0,
        AuthorityClass::Documentation => 1,
        AuthorityClass::Intent => 2,
    }
}
