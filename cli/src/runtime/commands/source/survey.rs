//! `specify source survey` handler — plan-time lead discovery.
//!
//! Resolves `<source>` against `plan.yaml.sources.<key>`, runs the
//! shared [`prep`] seam ([`prep::SourceOp::Survey`]) for adapter
//! resolution, brief directory, and the four-root sandbox (scratch at
//! `.specify/scratch/<adapter>/survey/`). Source extraction is
//! agent-only and two-phase; the CLI never blocks on agent work:
//!
//! - `--phase prepare` (default): build scratch, emit
//!   `source.execution.agent`, and print the survey handoff envelope
//!   (`{ adapter, version, briefs-dir, source-dir?, scratch-dir,
//!   leads[], execution }` — no `evidence-dir`; survey produces a lead
//!   set, not Evidence). Control returns to the agent.
//! - `--phase finalize`: validate the agent-produced `leads.md`
//!   against `schemas/discovery/lead.schema.json` *before* it becomes
//!   visible, merge the lead set via `Discovery::merge_survey`, and
//!   emit `source.survey.completed`.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_model::discovery::Discovery;
use specify_workflow::adapter::SourceOperation;
use specify_workflow::change::Plan;
use specify_workflow::journal::EventKind;
use specify_workflow::schema;

use crate::runtime::commands::source::cli::Phase;
use crate::runtime::commands::source::{op, prep};
use crate::runtime::context::Ctx;

/// Survey handoff envelope printed by the agent `prepare` phase.
/// No `evidence-dir`: survey merges a lead set via
/// `merge_survey`, it does not persist Evidence.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SurveyHandoff {
    adapter: String,
    version: semver::Version,
    briefs_dir: PathBuf,
    /// `$SOURCE_DIR` — absent for value-bound sources (e.g. `intent`).
    #[serde(skip_serializing_if = "Option::is_none")]
    source_dir: Option<PathBuf>,
    scratch_dir: PathBuf,
    /// Existing leads for this source — the re-survey baseline. Always
    /// present (empty on a fresh survey).
    leads: Vec<String>,
    execution: &'static str,
}

/// Result of a completed survey (agent `finalize`): the merged lead
/// ids and the `discovery.md` they landed in.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SurveyResult {
    adapter: String,
    source: String,
    /// Lead ids merged into `discovery.md`.
    leads: Vec<String>,
    discovery: PathBuf,
}

/// Run `specify source survey <source> [--plan <name>]
/// [--phase prepare|finalize]`.
///
/// # Errors
///
/// - `source-unknown` when `<source>` is not a
///   `plan.yaml.sources` key.
/// - propagates adapter-resolution, schema-validation, and merge
///   failures.
pub fn run(ctx: &Ctx, source: &str, plan_name: Option<&str>, phase: Phase) -> Result<()> {
    let plan = load_plan(ctx, plan_name)?;
    let binding = plan.sources.get(source).ok_or_else(|| Error::Diag {
        code: "source-unknown",
        detail: format!(
            "no source `{source}` in plan.yaml.sources; `specify source survey` resolves \
             its argument against the plan's source keys, not the adapter name"
        ),
    })?;

    let source_path =
        binding.path.as_deref().map(|raw| prep::resolve_source_path(ctx.layout().plan_dir(), raw));

    let prepared = prep::prepare(&prep::PrepRequest {
        adapter: &binding.adapter,
        version: binding.version.clone(),
        project_dir: &ctx.project_dir,
        op: prep::SourceOp::Survey,
        source: source_path.as_deref(),
        leads: &[],
        evidence_root: None,
    })?;

    let flow = SurveyFlow {
        common: op::Common {
            ctx,
            source,
            prepared: &prepared,
            source_path: source_path.as_deref(),
            binding,
            operation: SourceOperation::Survey,
        },
    };
    op::run(&flow, phase)
}

/// Survey's operation-specific seam onto the shared [`op::run`] flow:
/// the handoff omits `evidence-dir`, the commit merges the lead set
/// into `discovery.md`, and the completion event is
/// `source.survey.completed`.
struct SurveyFlow<'a> {
    common: op::Common<'a>,
}

impl<'a> op::Flow<'a> for SurveyFlow<'a> {
    type Handoff = SurveyHandoff;
    type Outcome = SurveyResult;

    fn common(&self) -> &op::Common<'a> {
        &self.common
    }

    fn handoff(&self, scratch: PathBuf) -> Result<SurveyHandoff> {
        let c = &self.common;
        Ok(SurveyHandoff {
            adapter: c.prepared.manifest.name.clone(),
            version: c.prepared.manifest.version.clone(),
            briefs_dir: c.prepared.briefs_dir.clone(),
            source_dir: c.source_path.map(Path::to_path_buf),
            scratch_dir: scratch,
            leads: existing_lead_ids(c.ctx, c.source)?,
            execution: "agent",
        })
    }

    fn write_handoff(w: &mut dyn Write, body: &SurveyHandoff) -> std::io::Result<()> {
        write_handoff_text(w, body)
    }

    fn commit(&self, raw: &str, _artifact_source: &Path) -> Result<SurveyResult> {
        let c = &self.common;
        let lead_ids = validate_and_merge(c.ctx, c.source, raw)?;
        Ok(SurveyResult {
            adapter: c.prepared.manifest.name.clone(),
            source: c.source.to_string(),
            leads: lead_ids,
            discovery: c.ctx.layout().discovery_path(),
        })
    }

    fn write_outcome(w: &mut dyn Write, body: &SurveyResult) -> std::io::Result<()> {
        write_result_text(w, body)
    }

    fn completed_event(&self) -> EventKind {
        let c = &self.common;
        EventKind::SourceSurveyCompleted {
            source: c.source.to_string(),
            adapter: c.prepared.manifest.name.clone(),
        }
    }
}

/// Parse, schema-validate, and merge a lead set into `discovery.md`.
/// Returns the merged lead ids. The schema check gates the merge, so an
/// invalid lead set leaves `discovery.md` untouched.
fn validate_and_merge(ctx: &Ctx, source: &str, raw: &str) -> Result<Vec<String>> {
    let mut leads = Discovery::parse_lead_set(raw)?.into_leads();
    if leads.is_empty() && !raw.trim().is_empty() {
        return Err(Error::Diag {
            code: "survey-leads-empty",
            detail: "leads.md contains text but no leads were parsed; each lead must be a \
                     `### <lead>` heading followed by `lead:` and `synopsis:` bullets using \
                     `-` or `*` markers"
                .to_string(),
        });
    }
    // Attribution is CLI-owned: a `survey` for `source` produces
    // `source`'s leads, so stamp every lead before the schema check
    // (which requires `source`) and the merge.
    for lead in &mut leads {
        lead.source = source.to_string();
    }
    schema::validate_leads(&leads)?;
    let lead_ids: Vec<String> = leads.iter().map(|lead| lead.lead.clone()).collect();

    let discovery_path = ctx.layout().discovery_path();
    let mut discovery = load_or_empty_discovery(&discovery_path)?;
    discovery.merge_survey(source, leads, &discovery_path)?;
    Ok(lead_ids)
}

/// Existing lead ids for `source`, read from `discovery.md` when
/// present — the re-survey baseline echoed into the handoff envelope.
fn existing_lead_ids(ctx: &Ctx, source: &str) -> Result<Vec<String>> {
    let discovery_path = ctx.layout().discovery_path();
    if !discovery_path.exists() {
        return Ok(Vec::new());
    }
    let discovery = Discovery::load(&discovery_path)?;
    Ok(discovery
        .leads()
        .iter()
        .filter(|lead| lead.source == source)
        .map(|lead| lead.lead.clone())
        .collect())
}

/// Load `discovery.md`, or start from an empty document when the file
/// is absent so the first survey can author the inventory.
fn load_or_empty_discovery(path: &Path) -> Result<Discovery> {
    if path.exists() { Discovery::load(path) } else { Discovery::parse("") }
}

fn load_plan(ctx: &Ctx, plan_name: Option<&str>) -> Result<Plan> {
    let plan = Plan::load(&ctx.layout().plan_path())?;
    if let Some(name) = plan_name
        && name != plan.name.as_str()
    {
        return Err(Error::Argument {
            flag: "--plan",
            detail: format!(
                "--plan `{name}` does not match the active plan `{}` at plan.yaml",
                plan.name
            ),
        });
    }
    Ok(plan)
}

fn write_handoff_text(w: &mut dyn Write, body: &SurveyHandoff) -> std::io::Result<()> {
    writeln!(w, "adapter: {} {}", body.adapter, body.version)?;
    writeln!(w, "execution: {}", body.execution)?;
    writeln!(w, "briefs-dir: {}", body.briefs_dir.display())?;
    if let Some(source_dir) = &body.source_dir {
        writeln!(w, "source-dir: {}", source_dir.display())?;
    }
    writeln!(w, "scratch-dir: {}", body.scratch_dir.display())?;
    if body.leads.is_empty() {
        writeln!(w, "leads: (none)")?;
    } else {
        writeln!(w, "leads: {}", body.leads.join(", "))?;
    }
    Ok(())
}

fn write_result_text(w: &mut dyn Write, body: &SurveyResult) -> std::io::Result<()> {
    writeln!(w, "adapter: {}", body.adapter)?;
    writeln!(w, "source: {}", body.source)?;
    writeln!(w, "discovery: {}", body.discovery.display())?;
    if body.leads.is_empty() {
        writeln!(w, "leads: (none)")?;
    } else {
        writeln!(w, "leads: {}", body.leads.join(", "))?;
    }
    Ok(())
}
