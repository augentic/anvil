//! Projection kernel — `project(response) -> SliceModel`.
//!
//! The agent owns cross-modal reconciliation — which requirements exist
//! and how claims merge or split. Everything around that judgment that
//! can be made deterministic is a pure projection over the structure the
//! agent returned: id assignment, authority resolution, status / winner
//! derivation, the rendered source list, and the stamped header. The
//! kernel **normalizes, never rejects** — any kernel-owned field the
//! agent happened to set (`id`, `status`, claim `winner`, rendered
//! `sources`) and any header field it supplied are ignored and
//! re-derived / re-stamped.
//!
//! Four conditions the kernel cannot project around are hard aborts,
//! mirroring the drift findings `specify slice validate` re-checks:
//! a claim that anchors no on-disk
//! Evidence (`slice-model-source-orphan`), a claim whose `kind`
//! disagrees with Evidence (`slice-model-claim-kind-mismatch`), a
//! `satisfies[]` `REQ` ref with no projected target
//! (`slice-model-cross-ref-orphan`), and an id outside its closed
//! three-digit grammar (`slice-model-id-grammar`).
//!
//! [`project`] is pure: it performs no I/O and reads no clock. The
//! caller (the `specify slice synthesize` handler) reads Evidence and
//! the plan to build the [`ProjectionHeader`], the per-source
//! `authority` map, the per-slice `overrides` map, and the
//! `evidence_claims` anchor index, then hands them in. **Kernel
//! determinism**: given fixed
//! inputs the output is byte-identical, and target-independent by
//! construction — no `target` or shape-brief input reaches this
//! function.

use std::collections::{BTreeMap, BTreeSet};

use specify_error::{Error, Result};
use specify_model::evidence::{AuthorityClass, ClaimKind};

use crate::slice::model::{ModelClaim, SliceModel};
use crate::slice::synthesis::authority::{ClaimRef, resolve};

/// The header fields the kernel stamps onto the persisted `model.yaml`.
///
/// Built by the caller from the slice's name and its bound project;
/// `target` is **not** a header field — it is resolved on demand from
/// `project`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionHeader {
    /// Stored schema version (`const: 1` today).
    pub version: u32,
    /// Slice name (kebab-case).
    pub slice: String,
    /// Bound project name, when one is bound.
    pub project: Option<String>,
}

/// Project the agent's synthesis-response `model` into a fully-derived
/// persisted [`SliceModel`].
///
/// `model` is the structure the agent returned; any kernel-owned or
/// header field it set is ignored and re-derived. `header` is the
/// kernel-stamped header. `authority` maps each source key to its
/// document-level [`AuthorityClass`], `overrides` is the per-slice
/// `authority-override` map (claim kind → winning source), and
/// `evidence_claims` is the `(source, id) → kind` anchor index the
/// caller distilled from the on-disk Evidence documents.
///
/// The projection runs in this order, returning the first violation:
///
/// 1. **Claim anchoring** — every `(source, id, kind)` claim must anchor
///    an Evidence claim: an absent `(source, id)` aborts
///    `slice-model-source-orphan`; a recorded kind that differs aborts
///    `slice-model-claim-kind-mismatch`.
/// 2. **Id assignment** — `requirements[].id` is re-derived as
///    `REQ-NNN` in declaration order (zero-padded, no holes), ignoring
///    any agent-supplied id.
/// 3. **Resolve + derive** — per requirement, [`resolve`] derives
///    `status` and per-claim `winner` markers, and the rendered
///    `sources` list is ordered highest-effective-authority first
///    (ties broken by declaration order via a stable sort).
/// 4. **Header** — `version` / `slice` / `project` are stamped from
///    `header`.
/// 5. **Cross-refs + grammar** — every `tasks[].satisfies[]` `REQ` ref
///    must name a projected requirement (`slice-model-cross-ref-orphan`)
///    and every `tasks[].id` / `requirements[].id` must match its closed
///    three-digit grammar (`slice-model-id-grammar`).
///
/// # Errors
///
/// Returns [`Error::Validation`] (exit 2) carrying the first of the four
/// abort codes above that the response violates.
pub fn project(
    mut model: SliceModel, header: ProjectionHeader, authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
    evidence_claims: &BTreeMap<(String, String), ClaimKind>,
) -> Result<SliceModel> {
    // Step 1 — claim anchoring runs before projection: the kernel
    // cannot project an unanchored claim.
    check_claim_anchors(&model, evidence_claims)?;

    // Steps 2–3 — re-derive ids, status, winners, and rendered sources.
    for (index, requirement) in model.requirements.iter_mut().enumerate() {
        requirement.id = Some(format!("REQ-{:03}", index + 1));

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

    // Step 4 — stamp the header, ignoring any agent-supplied values.
    model.version = Some(header.version);
    model.slice = Some(header.slice);
    model.project = header.project;

    // Step 5 — cross-ref then grammar over the now-projected ids.
    check_cross_refs(&model)?;
    check_id_grammar(&model)?;

    Ok(model)
}

/// Reject any claim that does not anchor an on-disk Evidence claim
/// (`slice-model-source-orphan`) or whose `kind` disagrees with the kind
/// Evidence records for that `(source, id)` (`slice-model-claim-kind-mismatch`).
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

/// Reject any `tasks[].satisfies[]` `REQ` reference that does not name a
/// projected `requirements[].id` (`slice-model-cross-ref-orphan`).
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

/// Reject any `REQ` or `TASK` id outside its closed three-digit grammar
/// (`slice-model-id-grammar`). `REQ` ids are
/// kernel-assigned and always pass; `TASK` ids are agent-authored, so
/// the gate is load-bearing for them.
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

/// `true` when `id` is `<prefix>NNN` with exactly three ASCII digits —
/// the closed `^REQ-[0-9]{3}$` / `^TASK-[0-9]{3}$` grammars.
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

/// Render the distinct source keys of `claims`, highest effective
/// authority first. Each source's
/// standing is the strongest effective level among its contributing
/// claims; a stable sort over the declaration-order list breaks ties by
/// first appearance.
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

/// The strongest effective [`Level`] one source reaches across its
/// contributing claims in a requirement.
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

/// Effective authority level of one `(source, kind)`, mirroring the
/// authority kernel's resolution order: a per-slice override outranks
/// every document class, otherwise the document-level class rank.
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

/// Effective authority level for source ordering. The derived `Ord`
/// places every [`Level::Class`] below [`Level::Override`] (variant
/// declaration order), matching the authority kernel's `Override`
/// outranks every class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Document-level class carried as its fixed rank
    /// (`behaviour < documentation < intent`).
    Class(u8),
    /// A per-slice `authority-override` forced this source to win.
    Override,
}

/// Default class ordering `intent > documentation > behaviour`,
/// independent of the `AuthorityClass` enum declaration order.
const fn class_rank(class: AuthorityClass) -> u8 {
    match class {
        AuthorityClass::Behaviour => 0,
        AuthorityClass::Documentation => 1,
        AuthorityClass::Intent => 2,
    }
}

#[cfg(test)]
mod tests;
