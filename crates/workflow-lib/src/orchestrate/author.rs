//! The plan-authoring orchestrator: the guest collapse of the
//! `/spec:plan` critical path.
//!
//! One call composes what the skill drives as a CLI sequence: `plan
//! create` scaffolding (same name gate, same overwrite refusal), the
//! per-source survey fan-out (via [`super::survey_all`]), the
//! reconciliation judgment leg ([`crate::judgment::propose::reconcile`]
//! with the kernel-projection check inside the repair loop), the
//! reconcile persist tail (`Plan::propose_from` under the
//! atomic write loop plus the `plan.reconcile.completed` event), the
//! Gate 1 prose persistence into `change.md` / `discovery.md`, and the
//! `plan validate` doctor sweep. The run exits with the plan at
//! `pending`; the outcome carries the literal Gate 1 transition hint —
//! the orchestrator never writes `approved` (Gate 1 stays
//! operator-only).
//!
//! Journal cadence composes the native verbs': the per-source
//! `source.execution.agent` / `source.survey.completed` pairs from the
//! fan-out, then the single `plan.reconcile.completed` after the
//! projection commits. The propose dry-run fires no journal event
//! natively, so the judgment dispatch adds none here either.

use std::collections::BTreeMap;

use artifacts::atomic::bytes_write;
use artifacts::discovery::Discovery;
use diagnostics::blocking_present;
use error::{Error, is_kebab};
use guest_model::Model;
use jiff::Timestamp;

use super::SurveyedSource;
use crate::change::{
    GateProse, Plan, ProjectRef, ProposalResponse, SourceBinding, apply_greenfield_seed,
    build_request, plan_doctor, resolve_topology,
};
use crate::config::{Layout, ProjectConfig, with_state};
use crate::journal::{self, Event, EventKind};
use crate::judgment::propose::{self, GateContext};
use crate::name::SliceName;
use crate::registry::Registry;
use crate::seam::SourceSeam;

/// The result of a completed [`author`]: the plan at `pending` with
/// its proposed slices, plus the literal Gate 1 hint for the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorOutcome {
    /// The authored plan's name.
    pub plan: String,
    /// Per-source survey results, in plan-binding order.
    pub surveyed: Vec<SurveyedSource>,
    /// Proposed slice names written to `plan.yaml.slices[]`, in the
    /// agent's response order.
    pub slices: Vec<String>,
    /// The literal closing hint the `/spec:plan` skill prints — Gate 1
    /// stays operator-only, so the orchestrator relays the command
    /// instead of running it.
    pub hint: String,
}

/// Author one plan end-to-end: scaffold → survey fan-out → reconcile →
/// project slices → persist Gate 1 prose → validate → exit at
/// `pending`.
///
/// `bindings` is the desugared `--source` / `--intent` map (the same
/// shape `plan create` hands `Plan::init`).
///
/// # Errors
///
/// - `plan-author-workspace-unsupported` (exit 2) when the plan root is
///   a workspace — the skill's workspace routing has no in-guest
///   counterpart, mirroring the execute loop's refusal.
/// - `change-name-not-kebab` / `already-exists` from the scaffold (the
///   `plan create` gates verbatim).
/// - survey fan-out failures from [`super::survey_all`] (earlier
///   sources stay merged — the native partial-progress posture).
/// - the judgment leg's model / schema / kernel / gate-prose failures
///   once the repair budget is exhausted.
/// - `plan-structural-errors` when the doctor sweep finds blocking
///   findings after the write.
pub async fn author<P: Model, S: SourceSeam>(
    model: &P, sources: &S, layout: Layout<'_>, now: Timestamp, name: &str,
    bindings: BTreeMap<String, SourceBinding>,
) -> Result<AuthorOutcome, Error> {
    refuse_workspace(layout)?;
    scaffold(layout, name, bindings)?;
    let surveyed = super::survey_all(sources, layout, now).await?;

    let discovery = Discovery::load(&layout.discovery_path())?;
    let topology = load_topology(layout)?;
    let request = build_request(&discovery, &topology)?;

    let plan = Plan::load(&layout.plan_path())?;
    let context = GateContext {
        plan: plan.name.as_str(),
        sources: &plan.sources,
    };
    // The check is the kernel-projection dry run against a throwaway
    // clone plus the gate-prose round-trip, so a grouping the kernel
    // would reject — or prose that would corrupt discovery.md — is
    // repaired in-loop rather than surfacing after the call.
    let mut response = propose::reconcile(model, &request, Some(context), |candidate| {
        let mut throwaway = plan.clone();
        throwaway.propose_from(candidate.clone(), &discovery, &topology)?;
        check_gate(candidate, name, &discovery)
    })
    .await?;
    let Some(gate) = response.gate.take() else {
        // Unreachable — the check refused gate-less answers — but a
        // typed refusal beats a panic if the invariant ever slips.
        return Err(gate_missing());
    };

    // The accepted projection runs inside the atomic write loop:
    // `propose_from` replaces `plan.entries`, `with_state` writes
    // `plan.yaml` on Ok and rolls back on any Err.
    let outcome = with_state::<Plan, _, _>(layout, "plan.yaml", |plan| {
        plan.propose_from(response, &discovery, &topology)
    })?;

    // Only after the write commits: emit the reconcile event.
    let event = Event::new(
        now,
        EventKind::PlanReconcileCompleted {
            plan_name: plan.name.clone(),
            slice_count: outcome.slice_names.len(),
            slice_names: outcome.slice_names.iter().map(SliceName::from).collect(),
        },
    );
    journal::append_batch(layout, std::slice::from_ref(&event))?;

    persist_gate_prose(layout, name, &gate, discovery)?;
    validate(layout)?;

    Ok(AuthorOutcome {
        plan: name.to_string(),
        surveyed,
        slices: outcome.slice_names,
        hint: gate_hint(name),
    })
}

/// The literal Gate 1 closing hint the `/spec:plan` skill prints.
fn gate_hint(name: &str) -> String {
    format!(
        "Plan `{name}` is at `pending`. Run `specify plan transition {name} approved` to stamp \
         Gate 1, then `/spec:execute` to drive the slices."
    )
}

/// Refuse workspace-routed plan authoring: the `/spec:plan` skill
/// syncs workspace slots before surveying, and the guest collapse has
/// no counterpart yet — mirroring the execute loop's refusal posture.
/// (A fresh plan has no entries, so only the `workspace: true`
/// discriminator applies here.)
fn refuse_workspace(layout: Layout<'_>) -> Result<(), Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    if config.workspace {
        return Err(Error::validation_failed(
            "plan-author-workspace-unsupported",
            "the guest plan-authoring collapse runs single-project plans only",
            "the plan root is a workspace (`workspace: true` in project.yaml); workspace \
             routing (slot sync) has no in-guest counterpart — author workspace plans through \
             the native /spec:plan skill",
        ));
    }
    Ok(())
}

/// The `plan create` scaffold semantics: kebab name gate, overwrite
/// refusal, `Plan::init` + atomic save. No `--auto-approve` and no
/// `--authority-override` surface — Gate 1 stamping stays operator-only
/// and override pre-seeding needs slice rows that do not exist yet.
fn scaffold(
    layout: Layout<'_>, name: &str, bindings: BTreeMap<String, SourceBinding>,
) -> Result<(), Error> {
    if !is_kebab(name) {
        return Err(Error::Diag {
            code: "change-name-not-kebab",
            detail: format!(
                "change: name `{name}` must be kebab-case \
                 (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens)"
            ),
        });
    }
    let plan_path = layout.plan_path();
    if plan_path.exists() {
        return Err(Error::Diag {
            code: "already-exists",
            detail: format!("refusing to overwrite existing plan at {}", plan_path.display()),
        });
    }
    let plan = Plan::init(name, bindings)?;
    plan.save(&plan_path)
}

/// Resolve the project topology the request embeds, minus the
/// operator-facing `greenfield-seed-shadowed` advisories (the seed
/// projection itself still applies).
fn load_topology(layout: Layout<'_>) -> Result<Vec<ProjectRef>, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    let mut topology = resolve_topology(&config, layout.project_dir())?;
    if let Some(registry) = Registry::load(layout.project_dir())? {
        let _shadowed =
            apply_greenfield_seed(&mut topology, &registry, layout.project_dir(), config.workspace);
    }
    Ok(topology)
}

/// The gate-prose leg of the repair-loop check: the answer must carry
/// the `gate` object and its discovery preamble must round-trip
/// through the discovery parser without corrupting the inventory.
fn check_gate(
    candidate: &ProposalResponse, name: &str, discovery: &Discovery,
) -> Result<(), Error> {
    let Some(gate) = &candidate.gate else {
        return Err(gate_missing());
    };
    let mut probe = discovery.clone();
    probe.set_preamble(&discovery_preamble(name, gate))
}

fn gate_missing() -> Error {
    Error::validation_failed(
        "plan-author-gate-missing",
        "the reconciliation answer carries the Gate 1 prose",
        "add the `gate` object (`change`, `discovery-summary`, `discovery-source-inventory`) \
         alongside `slices[]`",
    )
}

/// Persist the Gate 1 prose: `change.md` framed under the
/// deterministic `# Change — <name>` heading, and the `discovery.md`
/// preamble through the validated [`Discovery::set_preamble`] writer
/// (the lead inventory rides through untouched).
fn persist_gate_prose(
    layout: Layout<'_>, name: &str, gate: &GateProse, mut discovery: Discovery,
) -> Result<(), Error> {
    let brief = format!("# Change — {name}\n\n{}\n", gate.change.trim());
    bytes_write(&layout.change_brief_path(), brief.as_bytes())?;

    discovery.set_preamble(&discovery_preamble(name, gate))?;
    discovery.write_atomic(&layout.discovery_path())
}

/// Compose the deterministic `discovery.md` three-section frame around
/// the model-authored section bodies.
fn discovery_preamble(name: &str, gate: &GateProse) -> String {
    format!(
        "# Discovery — {name}\n\n## Summary\n\n{}\n\n## Source inventory\n\n{}",
        gate.discovery_summary.trim(),
        gate.discovery_source_inventory.trim()
    )
}

/// The `plan validate` doctor sweep — minus the stdout report surface
/// (the blocking decision and error code match the native verb).
fn validate(layout: Layout<'_>) -> Result<(), Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let registry = Registry::load(layout.project_dir())?;
    let findings = plan_doctor(
        &plan,
        Some(&layout.slices_dir()),
        registry.as_ref(),
        Some(layout.project_dir()),
    );
    if blocking_present(&findings) {
        return Err(Error::validation_failed(
            "plan-structural-errors",
            "plan must be free of structural errors",
            "run 'specify plan validate' for detail",
        ));
    }
    Ok(())
}
