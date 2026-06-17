//! `specify plan propose` handler — plan-time lead reconciliation.
//!
//! Two mutually exclusive modes wrap the agent-led reconciliation kernel
//! that lives in `crates/workflow/src/change/plan/core/propose.rs`:
//!
//! - `--dry-run` reads the surveyed `discovery.md` lead inventory and
//!   the resolved project topology (a `plan.yaml` is required), and
//!   emits the `kind: request` envelope ([`ProposalRequest`]) for the
//!   agent to group. `--format json` prints the schema-valid envelope
//!   verbatim. It writes no plan state and fires no journal event; its
//!   only filesystem effect is recreating the plan scratch lane
//!   (`.specify/scratch/plan/`) empty, so the agent has a fresh home
//!   for the response envelope and `--from` can never consume a stale
//!   one from a prior run.
//! - `--from <response.json>` is the only writer. It schema-gates the
//!   raw response bytes, deserialises the agent's grouping, **re-reads**
//!   `discovery.md` and the topology (never trusting a prior dry-run
//!   snapshot), projects the response onto `plan.yaml.slices[]` through
//!   [`Plan::propose_from`] under the atomic [`with_state`] write loop.
//!   When a bound project declares non-empty `project.yaml.platforms`,
//!   a deterministic bootstrap post-pass runs via
//!   [`vectis_missing_platforms`] (Vectis-bound projects only) before
//!   the write commits. Only after the write commits does the handler
//!   emit the single `plan.reconcile.completed` journal event.
//!
//! Passing neither mode fails with `plan-propose-mode-required`
//! (exit 2); the clap layer rejects passing both.

use std::io::Write;
use std::path::Path;

use serde::Serialize;
use specify_diagnostics::Diagnostic;
use specify_error::{Error, Result};
use specify_model::discovery::Discovery;
use specify_workflow::change::{
    Plan, ProjectMissingPlatforms, ProjectRef, ProposalRequest, ProposalResponse, ProposeOutcome,
    apply_greenfield_seed, build_request, resolve_topology,
};
use specify_workflow::config::{ProjectConfig, with_state};
use specify_workflow::journal::{self, Event, EventKind};
use specify_workflow::registry::Registry;
use specify_workflow::schema::validate_proposal_json;
use specify_workflow::vectis_missing_platforms;

use super::{Ref, cli, plan_ref, require_file};
use crate::runtime::context::Ctx;

/// Run `specify plan propose --dry-run | --from <response.json>`.
///
/// # Errors
///
/// - `plan-propose-mode-required` (exit 2) when neither `--dry-run` nor
///   `--from` is set.
/// - propagates every `plan-reconcile-*` projection error, the
///   `proposal-schema` gate failure, response read / parse failures,
///   and topology-resolution errors.
pub(super) fn propose(ctx: &Ctx, args: cli::ProposeArgs) -> Result<()> {
    match (args.dry_run, args.from) {
        (true, None) => dry_run(ctx),
        (false, Some(path)) => from(ctx, &path),
        // The clap `conflicts_with` guard makes `(true, Some(_))`
        // unreachable; return the mode error rather than risk a panic.
        (false, None) | (true, Some(_)) => Err(Error::validation_failed(
            "plan-propose-mode-required",
            "propose requires exactly one of --dry-run or --from",
            "pass exactly one of --dry-run or --from",
        )),
    }
}

/// `--dry-run`: emit the `kind: request` reconciliation envelope. Reads
/// `discovery.md` + topology; writes no plan state. Recreates the plan
/// scratch lane empty so the agent's response envelope
/// (`.specify/scratch/plan/propose-response.json`) is always this
/// run's — mirroring the source-operation `prepare` scratch reset.
fn dry_run(ctx: &Ctx) -> Result<()> {
    require_file(ctx)?;
    let discovery = load_discovery(ctx)?;
    // Greenfield-seed projection feeds the request the agent groups; the
    // shadow finding is operator-facing and surfaces at `--from` instead.
    let (topology, _seed_shadowed) = load_topology(ctx)?;
    let request = build_request(&discovery, &topology)?;
    reset_plan_scratch(ctx)?;
    ctx.write(&request, write_request_text)
}

/// Recreate `.specify/scratch/plan/` empty, dropping any response
/// envelope a prior run left behind.
fn reset_plan_scratch(ctx: &Ctx) -> Result<()> {
    let lane = ctx.layout().plan_scratch_dir();
    match std::fs::remove_dir_all(&lane) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Io(err)),
    }
    std::fs::create_dir_all(&lane).map_err(Error::Io)
}

/// `--from`: schema-gate and project the agent response onto
/// `plan.yaml.slices[]`, then emit the paired reconciliation events.
fn from(ctx: &Ctx, response_path: &Path) -> Result<()> {
    let plan_path = require_file(ctx)?;
    let raw = read_response(response_path)?;

    // Schema gate on the raw bytes first: it enforces the kebab
    // patterns, uniqueItems, and kind/version consts the typed DTO does
    // not, so it must run before the structural deserialise.
    validate_proposal_json(&raw)?;
    let response: ProposalResponse = serde_json::from_str(&raw).map_err(|err| {
        Error::validation_failed(
            "plan-propose-response-parse",
            "the --from response deserialises as a reconciliation response",
            format!("failed to parse response envelope: {err}"),
        )
    })?;

    // Re-read the catalog and topology every invocation — `--from`
    // never trusts a prior dry-run snapshot.
    let discovery = load_discovery(ctx)?;
    let (topology, seed_shadowed) = load_topology(ctx)?;

    // Detect missing platforms before entering the write loop so shell
    // scanning happens outside the atomic transaction.
    let project_missing = detect_missing_for_topology(&topology, ctx)?;

    // The projection runs inside the atomic write loop: `propose_from`
    // replaces `plan.entries`, `with_state` writes `plan.yaml` on Ok and
    // rolls back on any Err.
    let projected = with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
        let mut outcome = plan.propose_from(response, &discovery, &topology)?;

        if !project_missing.is_empty() {
            let bootstrap_names = plan.reconcile_platforms(&project_missing)?;
            if !bootstrap_names.is_empty() {
                let mut all_names = bootstrap_names;
                all_names.extend(outcome.slice_names);
                outcome.slice_names = all_names;
            }
        }

        Ok(Projected {
            plan: plan_ref(plan, &plan_path),
            outcome,
        })
    })?;

    // Only after the write commits: emit the reconcile event.
    emit_reconcile_event(ctx, &projected)?;

    ctx.write(&summary(projected, seed_shadowed), write_summary_text)
}

/// Detect missing platforms for each project in the topology.
///
/// Single-project mode checks `ctx.project_dir`; hub mode checks each
/// member's workspace clone directory.
fn detect_missing_for_topology(
    topology: &[ProjectRef], ctx: &Ctx,
) -> Result<Vec<ProjectMissingPlatforms>> {
    topology
        .iter()
        .filter(|p| !p.platforms.is_empty())
        .map(|p| {
            let project_dir = if topology.len() == 1 {
                ctx.project_dir.clone()
            } else {
                ctx.project_dir.join("workspace").join(&p.name)
            };
            let missing = vectis_missing_platforms(&project_dir, &p.platforms)?;
            Ok(ProjectMissingPlatforms {
                project: p.name.clone(),
                missing,
            })
        })
        .collect()
}

/// Successful projection carried out of the [`with_state`] write loop so
/// the journal events and the summary can be built only after the atomic
/// `plan.yaml` write commits.
struct Projected {
    plan: Ref,
    outcome: ProposeOutcome,
}

/// `--from` success summary. `--format json` emits this verbatim.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ProposeSummary {
    plan: Ref,
    slice_names: Vec<String>,
    slice_count: usize,
    /// RFC-46 D3 advisory `lead-decision-topic-overlap` review findings.
    /// Empty stays off the JSON wire.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    topic_overlaps: Vec<Diagnostic>,
    /// RFC-46 D6 advisory `greenfield-seed-shadowed` info findings — a
    /// bound project still declares a `greenfield_seed` after acquiring a
    /// baseline. Empty stays off the JSON wire.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    seed_shadowed: Vec<Diagnostic>,
}

/// Emit the single `plan.reconcile.completed` event — reached only after
/// the `plan.yaml` write has committed.
fn emit_reconcile_event(ctx: &Ctx, projected: &Projected) -> Result<()> {
    let event = Event::new(
        ctx.now(),
        EventKind::PlanReconcileCompleted {
            plan_name: projected.plan.name.clone().into(),
            slice_count: projected.outcome.slice_names.len(),
            slice_names: projected
                .outcome
                .slice_names
                .iter()
                .map(specify_workflow::name::SliceName::from)
                .collect(),
        },
    );
    journal::append_batch(ctx.layout(), std::slice::from_ref(&event))
}

/// Build the `--from` response summary from a committed projection.
fn summary(projected: Projected, seed_shadowed: Vec<Diagnostic>) -> ProposeSummary {
    ProposeSummary {
        slice_count: projected.outcome.slice_names.len(),
        slice_names: projected.outcome.slice_names,
        topic_overlaps: projected.outcome.topic_overlaps,
        seed_shadowed,
        plan: projected.plan,
    }
}

/// Load `discovery.md`, or an empty inventory when the file is absent so
/// the catalog assembly raises `plan-reconcile-empty-catalog` rather than
/// an I/O error on a never-surveyed plan.
fn load_discovery(ctx: &Ctx) -> Result<Discovery> {
    let path = ctx.layout().discovery_path();
    if path.exists() { Discovery::load(&path) } else { Discovery::parse("") }
}

/// Resolve the project topology the request embeds and the response binds
/// to — the committed `.specify/topology.lock` projection for a workspace,
/// or the sole project synthesised from `project.yaml` — then apply the
/// RFC-46 D6 greenfield-seed projection.
///
/// Returns the topology (with any greenfield seed projected into an empty
/// `surface[]`) alongside the advisory `greenfield-seed-shadowed` findings
/// for seeds a baseline already supersedes. A missing `registry.yaml` is a
/// no-op (no seed, no findings).
fn load_topology(ctx: &Ctx) -> Result<(Vec<ProjectRef>, Vec<Diagnostic>)> {
    let config = ProjectConfig::load(&ctx.project_dir)?;
    let mut topology = resolve_topology(&config, &ctx.project_dir)?;
    let shadowed = match Registry::load(&ctx.project_dir)? {
        Some(registry) => {
            apply_greenfield_seed(&mut topology, &registry, &ctx.project_dir, config.workspace)
        }
        None => Vec::new(),
    };
    Ok((topology, shadowed))
}

/// Read the `--from` response file, mapping a missing file to an exit-2
/// validation error rather than a generic I/O failure.
fn read_response(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::validation_failed(
                "plan-propose-response-not-found",
                "the --from response file must exist",
                format!("no response file at {}", path.display()),
            )
        } else {
            Error::Io(err)
        }
    })
}

fn write_request_text(w: &mut dyn Write, body: &ProposalRequest) -> std::io::Result<()> {
    writeln!(w, "projects:")?;
    for project in &body.projects {
        writeln!(w, "  - {} ({})", project.name, project.target)?;
    }
    writeln!(w, "leads:")?;
    for lead in &body.leads {
        writeln!(w, "  - {}/{}: {}", lead.source, lead.lead, lead.synopsis)?;
    }
    Ok(())
}

fn write_summary_text(w: &mut dyn Write, body: &ProposeSummary) -> std::io::Result<()> {
    writeln!(w, "plan: {}", body.plan.name)?;
    writeln!(w, "path: {}", body.plan.path)?;
    writeln!(w, "slice-count: {}", body.slice_count)?;
    if body.slice_names.is_empty() {
        writeln!(w, "slices: (none)")?;
    } else {
        writeln!(w, "slices: {}", body.slice_names.join(", "))?;
    }
    for finding in &body.topic_overlaps {
        writeln!(w, "advisory[lead-decision-topic-overlap]: {}", finding.impact)?;
    }
    for finding in &body.seed_shadowed {
        writeln!(w, "advisory[greenfield-seed-shadowed]: {}", finding.impact)?;
    }
    Ok(())
}
