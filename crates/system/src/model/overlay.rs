//! The `as-is` overlay persist tail (RFC-104 D4): replace only the
//! generated state, reapply `identities[]` then `decisions/`
//! `applies-to`, and canonical-write. A vanished id is an explicit gap.

use std::collections::BTreeMap;
use std::path::Path;

use error::Error;

use super::{ClaimRef, Element, ElementKind, Model, Relationship, State, Status};
use crate::decision::Decision;

/// The gap-note attribute key on placeholder elements.
const GAP: &str = "gap";

/// Persist a freshly correlated `as-is` state into `system.yaml`.
///
/// Loads the live file (first creation mints an empty model), maps
/// alias and superseded names onto their declared identity ids
/// (merging records that become the same id), appends an explicit
/// `status: unknown` gap element for every identity or `applies-to`
/// id the state did not recover, stamps `decisions/` `applies-to` as
/// `status: decided`, validates, and canonical-writes. `identities`,
/// `target`, and `transition-*` are never rewritten.
///
/// # Errors
///
/// Load and validation failures (`system-model-invalid` covers a
/// `decided` status in the incoming state and identity merges whose
/// kinds or endpoints disagree), plus atomic-write failures.
pub fn persist_as_is(path: &Path, state: State, decisions: &[Decision]) -> Result<Model, Error> {
    let mut model = match Model::load(path) {
        Ok(model) => model,
        Err(Error::Diag {
            code: "system-model-missing",
            ..
        }) => Model::empty(),
        Err(err) => return Err(err),
    };
    let mut state = state;
    reject_decided(&state)?;
    rename(&mut state, &alias_map(&model));
    merge(&mut state)?;
    gaps(&mut state, &model, decisions);
    stamp(&mut state, decisions);
    model.as_is = state;
    model.validate()?;
    artifacts::atomic::yaml_write(path, &model)?;
    Ok(model)
}

/// The D4 overlay-freshness gate for `system plan` and `system
/// review`: they validate the overlay, they never stamp `as-is`.
///
/// Fresh means replaying the live `identities[]` and `decisions/`
/// overlay onto the live `as-is` changes nothing, and no record is
/// stamped by a decision that no longer exists.
///
/// # Errors
///
/// `system-overlay-stale` when an identity or decision edit has not
/// been folded by a re-survey; propagates identity-merge validation
/// failures.
pub fn validate(model: &Model, decisions: &[Decision]) -> Result<(), Error> {
    let stale = |detail: String| {
        Err(Error::validation_failed(
            "system-overlay-stale",
            "re-run `emery system survey` to refold the overlay",
            detail,
        ))
    };
    let live: std::collections::BTreeSet<&str> =
        decisions.iter().map(|decision| decision.id.as_str()).collect();
    let stamped = model
        .as_is
        .elements
        .iter()
        .map(|element| (&element.id, element.decision.as_deref()))
        .chain(
            model
                .as_is
                .relationships
                .iter()
                .map(|relationship| (&relationship.id, relationship.decision.as_deref())),
        );
    for (id, decision) in stamped {
        if let Some(decision) = decision
            && !live.contains(decision)
        {
            return stale(format!("`{id}` is stamped by vanished decision `{decision}`"));
        }
    }
    let mut replay = model.as_is.clone();
    rename(&mut replay, &alias_map(model));
    merge(&mut replay)?;
    gaps(&mut replay, model, decisions);
    stamp(&mut replay, decisions);
    if replay != model.as_is {
        return stale("identities or decisions changed since the last survey folded them".into());
    }
    Ok(())
}

/// Correlation cannot emit `status: decided` — only the persist tail
/// stamps it from `decisions/`.
fn reject_decided(state: &State) -> Result<(), Error> {
    let decided = state
        .elements
        .iter()
        .map(|element| (&element.id, element.status))
        .chain(
            state.relationships.iter().map(|relationship| (&relationship.id, relationship.status)),
        )
        .find(|(_, status)| *status == Status::Decided);
    match decided {
        Some((id, _)) => Err(Error::validation_failed(
            "system-model-invalid",
            "correlation cannot decide",
            format!("`{id}` arrived with `status: decided`; only `decisions/` records decide"),
        )),
        None => Ok(()),
    }
}

/// Alias or superseded name → declared identity id.
fn alias_map(model: &Model) -> BTreeMap<&str, &str> {
    let mut map = BTreeMap::new();
    for identity in &model.identities {
        for name in identity.aliases.iter().chain(&identity.supersedes) {
            map.insert(name.as_str(), identity.id.as_str());
        }
    }
    map
}

/// Rewrite record ids and relationship endpoints onto identity ids.
fn rename(state: &mut State, map: &BTreeMap<&str, &str>) {
    let canonical = |id: &mut String| {
        if let Some(target) = map.get(id.as_str()) {
            *id = (*target).to_string();
        }
    };
    for element in &mut state.elements {
        canonical(&mut element.id);
    }
    for relationship in &mut state.relationships {
        canonical(&mut relationship.id);
        canonical(&mut relationship.from);
        canonical(&mut relationship.to);
    }
}

/// Fold records that renamed onto the same id into one, first
/// occurrence anchoring order and attributes.
fn merge(state: &mut State) -> Result<(), Error> {
    let kind_conflict = |id: &str| {
        Err(Error::validation_failed(
            "system-model-invalid",
            "identity merge conflict",
            format!("records renamed onto `{id}` disagree on their shape"),
        ))
    };

    let mut elements: Vec<Element> = Vec::with_capacity(state.elements.len());
    for element in state.elements.drain(..) {
        match elements.iter_mut().find(|kept| kept.id == element.id) {
            None => elements.push(element),
            Some(kept) => {
                if kept.kind != element.kind {
                    return kind_conflict(&element.id);
                }
                fold(&mut kept.claims, element.claims);
                kept.status = wider(kept.status, element.status);
                kept.context_only &= element.context_only;
                for (key, value) in element.attributes {
                    kept.attributes.entry(key).or_insert(value);
                }
            }
        }
    }
    state.elements = elements;

    let mut relationships: Vec<Relationship> = Vec::with_capacity(state.relationships.len());
    for relationship in state.relationships.drain(..) {
        match relationships.iter_mut().find(|kept| kept.id == relationship.id) {
            None => relationships.push(relationship),
            Some(kept) => {
                if kept.kind != relationship.kind
                    || kept.from != relationship.from
                    || kept.to != relationship.to
                {
                    return kind_conflict(&relationship.id);
                }
                fold(&mut kept.claims, relationship.claims);
                kept.status = wider(kept.status, relationship.status);
                kept.context_only &= relationship.context_only;
                for (key, value) in relationship.attributes {
                    kept.attributes.entry(key).or_insert(value);
                }
            }
        }
    }
    state.relationships = relationships;
    Ok(())
}

/// Union `extra` into `claims`, preserving order and dropping
/// duplicates.
fn fold(claims: &mut Vec<ClaimRef>, extra: Vec<ClaimRef>) {
    for claim in extra {
        if !claims.contains(&claim) {
            claims.push(claim);
        }
    }
}

/// The epistemically wider of two merged statuses: a retained
/// disagreement outranks evidence, evidence outranks inference.
fn wider(a: Status, b: Status) -> Status {
    let rank = |status: Status| match status {
        Status::Conflict => 3,
        Status::Evidenced => 2,
        Status::Inferred => 1,
        Status::Unknown | Status::Decided => 0,
    };
    if rank(b) > rank(a) { b } else { a }
}

/// Append an explicit gap element for every identity or `applies-to`
/// id the new state did not recover — never a silent drop.
fn gaps(state: &mut State, model: &Model, decisions: &[Decision]) {
    let identity_ids = model.identities.iter().map(|identity| identity.id.as_str());
    let decided_ids =
        decisions.iter().flat_map(|decision| decision.applies_to.iter().map(String::as_str));
    let missing: Vec<&str> =
        identity_ids.chain(decided_ids).filter(|id| !state.contains(id)).collect();
    let note = || {
        BTreeMap::from([(GAP.to_string(), "declared id not recovered by this survey".to_string())])
    };
    for id in missing {
        if state.contains(id) {
            continue; // A prior iteration already appended this gap.
        }
        // Carry the prior survey's shape when the id was modeled
        // before: a vanished relationship stays a relationship as long
        // as its endpoints survived; otherwise the gap is an element.
        let prior = model.as_is.relationships.iter().find(|relationship| relationship.id == id);
        if let Some(prior) = prior
            && [&prior.from, &prior.to]
                .iter()
                .all(|end| state.elements.iter().any(|element| element.id == **end))
        {
            state.relationships.push(Relationship {
                id: id.to_string(),
                kind: prior.kind,
                from: prior.from.clone(),
                to: prior.to.clone(),
                status: Status::Unknown,
                claims: Vec::new(),
                decision: None,
                context_only: false,
                attributes: note(),
            });
            continue;
        }
        let kind = model
            .as_is
            .elements
            .iter()
            .find(|element| element.id == id)
            .map_or(ElementKind::System, |element| element.kind);
        state.elements.push(Element {
            id: id.to_string(),
            kind,
            status: Status::Unknown,
            claims: Vec::new(),
            decision: None,
            context_only: false,
            attributes: note(),
        });
    }
}

/// Reapply `decisions/` `applies-to` as `status: decided` plus
/// `decision: <id>`; claims stay as retained provenance.
fn stamp(state: &mut State, decisions: &[Decision]) {
    for decision in decisions {
        for target in &decision.applies_to {
            for element in &mut state.elements {
                if element.id == *target {
                    element.status = Status::Decided;
                    element.decision = Some(decision.id.clone());
                }
            }
            for relationship in &mut state.relationships {
                if relationship.id == *target {
                    relationship.status = Status::Decided;
                    relationship.decision = Some(decision.id.clone());
                }
            }
        }
    }
}
