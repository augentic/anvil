//! `plan.yaml.slices[].authority-override` mutation engine (workflow
//! `kind: schema` evaluator contract). The CLI handlers ([`crate::change::Plan::amend`] siblings in
//! the `specify` runtime binary) parse `--authority-override` /
//! `--clear-authority-override` / `--clear-authority-overrides`
//! flags into the typed `(slice, kind, source)` tuples this
//! module consumes, then drive the in-memory plan through
//! [`mutate_authority_overrides`] and the post-mutation orphan gate
//! [`reject_orphan_overrides`].
//!
//! Set-then-clear on the same `(slice, kind)` resolves to the cleared
//! state, and the journal records the clear (not the set) to match
//! the on-disk outcome. The whole-map clear emits one
//! `Clear` event per kind that was present before the wipe.
//! Sets sort before clears for the same `(slice, kind)` (see
//! [`crate::journal::AuthorityOverrideAction`]'s `Ord` invariant).

use std::collections::{BTreeMap, BTreeSet};

use specify_error::{Error, Result};
use specify_model::evidence::ClaimKind;

use super::model::{Entry, Plan};
use super::validate::orphan_authority_override_keys;
use crate::journal::{self, AuthorityOverrideAction};

/// Build the batched `plan.amend.authority-override` event list
/// matching the on-disk outcome of the mutation walk in
/// [`mutate_authority_overrides`]. Set events are emitted only for
/// survivors (sets not subsequently cleared); per-kind Clear events
/// are deduplicated across the `--clear-authority-override` and
/// `--clear-authority-overrides` surfaces.
type AuthorityOverrideSortKey = (String, Option<String>, AuthorityOverrideAction);

fn emit_override_events(
    plan_name: &str, set_map: &BTreeMap<(String, ClaimKind), String>,
    clear_set: &BTreeSet<(String, ClaimKind)>, clear_all_set: &BTreeSet<String>,
    clear_all_emitted: &BTreeMap<String, Vec<ClaimKind>>, now: jiff::Timestamp,
) -> Vec<journal::Event> {
    let mut pending: Vec<(AuthorityOverrideSortKey, journal::Event)> = Vec::new();
    let mut record = |slice: &str,
                      action: AuthorityOverrideAction,
                      claim_kind: ClaimKind,
                      source: Option<&str>| {
        let claim_kind = Some(claim_kind.to_string());
        pending.push((
            (slice.to_string(), claim_kind.clone(), action),
            journal::Event::new(
                now,
                journal::EventKind::PlanAmendAuthorityOverride {
                    plan_name: plan_name.into(),
                    slice_name: slice.into(),
                    action,
                    claim_kind,
                    source: source.map(str::to_owned),
                },
            ),
        ));
    };
    for ((slice, kind), key) in set_map {
        if clear_set.contains(&(slice.clone(), *kind)) || clear_all_set.contains(slice) {
            continue;
        }
        record(slice, AuthorityOverrideAction::Set, *kind, Some(key.as_str()));
    }
    for (slice, kind) in clear_set {
        if clear_all_set.contains(slice)
            && clear_all_emitted.get(slice).is_some_and(|kinds| kinds.contains(kind))
        {
            continue;
        }
        record(slice, AuthorityOverrideAction::Clear, *kind, None);
    }
    for (slice, kinds) in clear_all_emitted {
        for kind in kinds {
            record(slice, AuthorityOverrideAction::Clear, *kind, None);
        }
    }
    // Final sort gives a byte-stable batched append regardless of
    // operator-issued flag order: `(slice, kind, action)`.
    pending.sort_by(|(left, _), (right, _)| left.cmp(right));
    pending.into_iter().map(|(_, event)| event).collect()
}

/// Apply the full authority-override mutation set on `plan` and
/// return the matching journal events.
///
/// Workflow `kind: schema` evaluator contract — applies `--authority-override` /
/// `--clear-authority-override` / `--clear-authority-overrides` in a
/// deterministic order:
///
/// 1. Sets — collapse duplicate `(slice, kind)` pairs to the last
///    value.
/// 2. Single-kind clears — remove the entry if present (no-op if
///    absent).
/// 3. Whole-map clears — wipe the slice's entire map; emit one
///    `Clear` event per kind that was present before the wipe.
///
/// Set-then-clear on the same `(slice, kind)` resolves to the
/// cleared state, and the journal records the clear (not the set)
/// to match the on-disk outcome.
///
/// # Errors
///
/// Returns `Error::Validation` /
/// `plan-authority-override-unknown-slice` (exit 2) when any flag
/// references a slice not present on `plan`. Shared by `plan create`,
/// `plan add` (with empty clears), and `plan amend`, so all three
/// paths emit byte-identical journal events for the same
/// `(slice, kind)` set.
pub fn mutate_authority_overrides(
    plan: &mut Plan, plan_name: &str, sets: &[(String, ClaimKind, String)],
    clears: &[(String, ClaimKind)], clear_all: &[String], now: jiff::Timestamp,
) -> Result<Vec<journal::Event>> {
    let set_map: BTreeMap<(String, ClaimKind), String> =
        sets.iter().cloned().map(|(s, k, v)| ((s, k), v)).collect();
    let clear_set: BTreeSet<(String, ClaimKind)> = clears.iter().cloned().collect();
    let clear_all_set: BTreeSet<String> = clear_all.iter().cloned().collect();

    refuse_unknown_slices(plan, plan_name, &set_map, &clear_set, &clear_all_set)?;
    for ((slice, kind), key) in &set_map {
        entry_mut(plan, plan_name, slice)?.authority_override.by_kind.insert(*kind, key.clone());
    }
    for (slice, kind) in &clear_set {
        entry_mut(plan, plan_name, slice)?.authority_override.by_kind.remove(kind);
    }
    let mut clear_all_emitted: BTreeMap<String, Vec<ClaimKind>> = BTreeMap::new();
    for slice in &clear_all_set {
        let entry = entry_mut(plan, plan_name, slice)?;
        let kinds: Vec<ClaimKind> = entry.authority_override.by_kind.keys().copied().collect();
        entry.authority_override.by_kind.clear();
        clear_all_emitted.insert(slice.clone(), kinds);
    }

    Ok(emit_override_events(
        plan_name,
        &set_map,
        &clear_set,
        &clear_all_set,
        &clear_all_emitted,
        now,
    ))
}

/// Emit the seeded `plan.amend.authority-override` Set events for
/// a freshly created entry.
///
/// `plan add --authority-override` populates
/// [`Entry::authority_override`] up-front; this helper turns those
/// per-kind entries into the same shape `plan create` and
/// `plan amend` write so the journal lines up byte-identically
/// across the three handlers. Returns an empty vec when the entry
/// carries no overrides; clears are not modelled (the add path can
/// only set).
#[must_use]
pub fn emit_seed_events(
    plan_name: &str, entry: &Entry, now: jiff::Timestamp,
) -> Vec<journal::Event> {
    let set_map: BTreeMap<(String, ClaimKind), String> = entry
        .authority_override
        .by_kind
        .iter()
        .map(|(kind, key)| ((entry.name.to_string(), *kind), key.clone()))
        .collect();
    emit_override_events(
        plan_name,
        &set_map,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeMap::new(),
        now,
    )
}

/// Post-mutation orphan-source gate.
///
/// Runs [`orphan_authority_override_keys`] on `plan` and
/// short-circuits the CLI write with a single payload-free
/// `Error::Validation` (exit 2) when any finding fires. Findings are
/// joined in the deterministic order the domain helper produces (slice
/// declaration order, then claim-kind iteration order) into the
/// envelope's `detail`.
///
/// This is the post-mutation gate that catches new orphan
/// overrides introduced by `--authority-override` on `plan create`
/// / `plan amend`. The `plan add` path's `Plan::create` already
/// re-runs `Plan::validate` (which folds in the same check) so it
/// doesn't need a separate call.
///
/// # Errors
///
/// Returns `Error::Validation` when at least one orphan-source
/// finding blocks (a `critical`/`important` violation).
pub fn reject_orphan_overrides(plan: &Plan) -> Result<()> {
    let findings: Vec<_> = orphan_authority_override_keys(&plan.entries)
        .into_iter()
        .filter(specify_diagnostics::blocking)
        .collect();
    let Some(first) = findings.first() else {
        return Ok(());
    };
    let detail = findings.iter().map(|f| f.impact.clone()).collect::<Vec<_>>().join("; ");
    Err(Error::Validation {
        code: first.rule_id.clone().unwrap_or_default().into(),
        detail,
    })
}

/// Refuse the whole invocation if any flag references a slice not
/// present on `plan`. Runs before any mutation so callers never
/// observe partial state.
fn refuse_unknown_slices(
    plan: &Plan, plan_name: &str, set_map: &BTreeMap<(String, ClaimKind), String>,
    clear_set: &BTreeSet<(String, ClaimKind)>, clear_all_set: &BTreeSet<String>,
) -> Result<()> {
    let known: BTreeSet<&str> = plan.entries.iter().map(|e| e.name.as_str()).collect();
    let unknown = set_map
        .keys()
        .map(|(s, _)| s.as_str())
        .chain(clear_set.iter().map(|(s, _)| s.as_str()))
        .chain(clear_all_set.iter().map(String::as_str))
        .find(|s| !known.contains(s));
    if let Some(slice) = unknown {
        return Err(unknown_slice_err(plan_name, slice));
    }
    Ok(())
}

/// Borrow the named entry on `plan` by name.
///
/// Returns the same `plan-authority-override-unknown-slice`
/// diagnostic that pre-flight `refuse_unknown_slices` emits inside
/// [`mutate_authority_overrides`] when no entry matches.
///
/// # Errors
///
/// Returns `Error::Validation` /
/// `plan-authority-override-unknown-slice` when no entry on `plan`
/// matches `slice`.
pub fn entry_mut<'a>(plan: &'a mut Plan, plan_name: &str, slice: &str) -> Result<&'a mut Entry> {
    plan.entries
        .iter_mut()
        .find(|e| e.name == slice)
        .ok_or_else(|| unknown_slice_err(plan_name, slice))
}

/// Build the shared `plan-authority-override-unknown-slice`
/// diagnostic (exit 2) so the CLI handlers don't each open-code the
/// same message.
#[must_use]
pub fn unknown_slice_err(plan_name: &str, slice: &str) -> Error {
    Error::validation_failed(
        "plan-authority-override-unknown-slice",
        "--authority-override / --clear-authority-override(s) must reference a slice present in \
         the plan",
        format!(
            "no slice named '{slice}' in plan '{plan_name}'; add the slice (e.g. specify plan add \
             {slice}) before authoring authority-override entries"
        ),
    )
}

#[cfg(test)]
mod tests;
