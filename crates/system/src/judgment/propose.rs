//! The initial-plan proposal judgment (RFC-104 D10): when `target` is
//! absent at load, one judgment over the live `as-is`, `scope.yaml`,
//! and declarative Evidence proposes `target` and one wave.

use std::collections::{BTreeMap, BTreeSet};

use error::Error;
use omnia_guest::Model;
use serde::{Deserialize, Serialize};

use super::correlate::EvidenceRef;
use super::{prose, render_json, repaired};
use crate::migration::{Disposition, Migration, Wave};
use crate::model::{State, Status};
use crate::scope::Scope;

/// Wire version stamped on both envelopes.
pub(crate) const PROPOSAL_VERSION: u32 = 1;

/// Proposal input envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InputKind {
    /// Agent proposal inputs.
    Inputs,
}

/// The proposal step's input envelope.
///
/// Deliberately absent: `identities[]`, `decisions/` bodies, and any
/// existing `migration.yaml` (the judgment runs only when `target` is
/// absent, before an operator-owned plan exists).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ProposalInputs {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: InputKind,
    /// The declared boundary (`scope.yaml`).
    pub scope: Scope,
    /// The live recovered `as-is` state (possibly empty).
    pub as_is: State,
    /// Included Evidence documents carrying intent or constraint
    /// claims, for the agent to read from the lent tree.
    pub evidence: Vec<EvidenceRef>,
}

/// Proposal response envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseKind {
    /// The agent's proposal.
    Response,
}

/// `kind: response` envelope — the proposed target architecture,
/// optional transitions, dispositions, and exactly one wave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProposalResponse {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: ResponseKind,
    /// The proposed `target` named state.
    pub target: State,
    /// Proposed intermediate states, keyed `transition-<kebab>`.
    /// Empty for a one-hop migration.
    #[serde(default)]
    pub transitions: BTreeMap<String, State>,
    /// Proposed modernization dispositions.
    #[serde(default)]
    pub dispositions: Vec<Disposition>,
    /// The proposed first migration wave.
    pub wave: Wave,
}

/// The accepted proposal the persist tail applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// The proposed `target` state.
    pub target: State,
    /// Proposed `transition-*` states.
    pub transitions: BTreeMap<String, State>,
    /// Proposed dispositions.
    pub dispositions: Vec<Disposition>,
    /// The proposed first wave.
    pub wave: Wave,
}

/// Assemble the proposal input envelope.
#[must_use]
pub const fn inputs(scope: Scope, as_is: State, evidence: Vec<EvidenceRef>) -> ProposalInputs {
    ProposalInputs {
        version: PROPOSAL_VERSION,
        kind: InputKind::Inputs,
        scope,
        as_is,
        evidence,
    }
}

/// Run the initial-plan proposal judgment.
///
/// `claims` is the `(source, id)` anchor index over the persisted
/// Evidence corpus; `leads` the persisted `(source, lead)` pairs;
/// `decision_ids` the loaded `decisions/` record ids. The
/// deterministic tail closes the answer against all three.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / validation
/// failure once the repair budget is exhausted.
pub async fn propose<P: Model>(
    model: &P, inputs: &ProposalInputs, claims: &BTreeSet<(String, String)>,
    leads: &BTreeSet<(String, String)>, decision_ids: &BTreeSet<String>,
) -> Result<Proposal, Error> {
    let schema = project::answers::render(&crate::answers::proposal());
    let user =
        format!("## Proposal inputs\n\n```json\n{}\n```", render_json(inputs, "proposal inputs")?);
    let as_is = inputs.as_is.clone();
    repaired(
        model,
        prose::propose_system(),
        user,
        "proposal",
        None,
        &schema,
        project::judgment::Lent::default(),
        move |answer| {
            let response: ProposalResponse = serde_saphyr::from_str(answer).map_err(|err| {
                Error::validation_failed(
                    "system-propose-response-parse",
                    "the answer deserialises as a proposal response",
                    format!("failed to parse proposal response: {err}"),
                )
            })?;
            tail(response, &as_is, claims, leads, decision_ids)
        },
    )
    .await
}

/// The deterministic tail: state validation, claim closure, migration
/// coherence, and named-state resolution over the composed model.
fn tail(
    response: ProposalResponse, as_is: &State, claims: &BTreeSet<(String, String)>,
    leads: &BTreeSet<(String, String)>, decision_ids: &BTreeSet<String>,
) -> Result<Proposal, Error> {
    let invalid = |rule: &str, detail: String| {
        Err(Error::validation_failed("system-propose-invalid", rule, detail))
    };
    response.target.validate("target")?;
    close_claims("target", &response.target, claims)?;
    for (name, state) in &response.transitions {
        let stem = name.strip_prefix("transition-").unwrap_or_default();
        if stem.is_empty() || !artifacts::evidence::is_kebab(stem) {
            return invalid(
                "transition name grammar",
                format!("proposed state `{name}` is not `transition-<kebab>`"),
            );
        }
        state.validate(name)?;
        close_claims(name, state, claims)?;
    }

    // Migration coherence over the file shape the persist tail writes.
    let migration = Migration {
        version: 1,
        dispositions: response.dispositions,
        waves: vec![response.wave],
    };
    migration.validate()?;
    let wave = &migration.waves[0];

    let mut states: BTreeSet<&str> = BTreeSet::from(["as-is", "target"]);
    states.extend(response.transitions.keys().map(String::as_str));
    for name in [&wave.architecture.before, &wave.architecture.after] {
        if !states.contains(name.as_str()) {
            return invalid(
                "unresolved state",
                format!("wave `{}` names `{name}`, which is not a proposed or live state", wave.id),
            );
        }
    }
    let mut elements: BTreeSet<&str> =
        as_is.elements.iter().map(|element| element.id.as_str()).collect();
    elements.extend(response.target.elements.iter().map(|element| element.id.as_str()));
    for state in response.transitions.values() {
        elements.extend(state.elements.iter().map(|element| element.id.as_str()));
    }
    for id in
        wave.affected_elements.iter().chain(&wave.touched_elements).chain(&wave.context_elements)
    {
        if !elements.contains(id.as_str()) {
            return invalid(
                "unresolved element",
                format!("wave `{}` names element `{id}`, which no proposed state carries", wave.id),
            );
        }
    }
    for scope in &wave.evidence_scopes {
        if !leads.contains(&(scope.source.clone(), scope.lead.clone())) {
            return invalid(
                "unresolved evidence scope",
                format!(
                    "wave `{}` selects `{}#{}`, which no persisted Evidence document carries",
                    wave.id, scope.source, scope.lead
                ),
            );
        }
    }
    for mapping in &wave.delivery_mappings {
        if !leads.contains(&(mapping.source.clone(), mapping.lead.clone())) {
            return invalid(
                "unresolved mapping lead",
                format!(
                    "wave `{}` maps `{}#{}`, which no persisted Evidence document carries",
                    wave.id, mapping.source, mapping.lead
                ),
            );
        }
    }
    for id in &wave.decisions {
        if !decision_ids.contains(id) {
            return invalid(
                "unresolved decision",
                format!("wave `{}` cites decision `{id}`, which has no decisions/ record", wave.id),
            );
        }
    }

    let Migration {
        dispositions,
        mut waves,
        ..
    } = migration;
    let wave = waves.remove(0);
    Ok(Proposal {
        target: response.target,
        transitions: response.transitions,
        dispositions,
        wave,
    })
}

/// A proposed state cannot decide, and every cited claim must exist.
fn close_claims(
    name: &str, state: &State, claims: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let cited =
        state.elements.iter().map(|element| (&element.id, element.status, &element.claims)).chain(
            state
                .relationships
                .iter()
                .map(|relationship| (&relationship.id, relationship.status, &relationship.claims)),
        );
    for (id, status, refs) in cited {
        if status == Status::Decided {
            return Err(Error::validation_failed(
                "system-propose-decided",
                "the proposal cannot decide",
                format!(
                    "state `{name}`: `{id}` answered with `status: decided`; only `decisions/` \
                     records decide"
                ),
            ));
        }
        for claim in refs {
            if !claims.contains(&(claim.source.clone(), claim.id.clone())) {
                return Err(Error::validation_failed(
                    "system-propose-claims",
                    "every cited claim exists in the Evidence corpus",
                    format!(
                        "state `{name}`: `{id}` cites `{}#{}`, which no persisted Evidence \
                         document carries",
                        claim.source, claim.id
                    ),
                ));
            }
        }
    }
    Ok(())
}
