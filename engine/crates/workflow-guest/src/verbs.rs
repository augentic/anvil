//! Drive the collapsed orchestrator verbs against the WIT-backed
//! [`Provider`].
//!
//! `specify_dispatch::guest::route` parses these verbs but cannot run
//! them — the seam lives here, where the WIT bindings are. Each arm
//! loads the shared [`Ctx`] (the `"."` preopen is the project root),
//! reads the clock once at the boundary, drives the matching
//! `specify_workflow::orchestrate` entry point, and renders a compact
//! outcome envelope through the same [`Ctx::write`] / `report`
//! machinery the native verbs use — failures land on stderr with the
//! native error-body shape and the native exit-code mapping.
//!
//! Two caller-resolved build parameters mirror what the native
//! prepare phase owns: the manifest-declared build-inputs list comes
//! from the project's bound target adapter, and the working tree names
//! the live shared mount (`base: "live"` — snapshot identity is the
//! RFC-55 forward hook; today's deployments share one live tree).

use std::path::PathBuf;

use serde::Serialize;
use specify_dispatch::commands::slice::artifact_classes;
use specify_dispatch::context::Ctx;
use specify_dispatch::guest::{Orchestration, Verb};
use specify_dispatch::output::{Exit, Format, report};
use specify_error::Error;
use specify_workflow::orchestrate;
use specify_workflow::seam::WorkingTree;
use specify_workflow::slice::BuildStatus;

use crate::provider::Provider;

/// Run one routed orchestration to its exit code, rendering the
/// success envelope on stdout or the native failure envelope on
/// stderr.
pub async fn drive(orchestration: Orchestration) -> Exit {
    let format = orchestration.format;
    match run(format, orchestration.plan_dir, orchestration.verb).await {
        Ok(()) => Exit::Success,
        Err(err) => report(format, &err),
    }
}

/// Dispatch one orchestrator verb against the loaded context. The
/// project root is the `"."` mount preopen, so `Ctx` anchors there
/// rather than walking from a process CWD WASI does not model.
async fn run(format: Format, plan_dir: Option<PathBuf>, verb: Verb) -> Result<(), Error> {
    let ctx = Ctx::load_at(format, plan_dir, std::path::Path::new("."))?;
    let now = ctx.now();
    match verb {
        Verb::Survey { source, plan } => {
            let outcome =
                orchestrate::survey(&Provider, ctx.layout(), now, &source, plan.as_deref()).await?;
            let body = SurveyBody {
                source: outcome.source,
                adapter: outcome.adapter,
                leads: outcome.leads,
            };
            ctx.write(&body, |w, body| {
                writeln!(w, "surveyed {} via {}", body.source, body.adapter)?;
                for lead in &body.leads {
                    writeln!(w, "lead: {lead}")?;
                }
                Ok(())
            })
        }
        Verb::Extract { source, lead, slice } => {
            let outcome =
                orchestrate::extract(&Provider, ctx.layout(), now, &source, &lead, &slice).await?;
            let body = ExtractBody {
                source: outcome.source,
                adapter: outcome.adapter,
                evidence: outcome.evidence,
            };
            ctx.write(&body, |w, body| {
                writeln!(w, "extracted {} via {}", body.source, body.adapter)?;
                writeln!(w, "evidence: {}", body.evidence.display())
            })
        }
        Verb::Build { slice } => {
            let manifest_inputs = ctx.resolve_target_adapter()?.manifest.inputs;
            let tree = WorkingTree {
                base: "live".to_string(),
                subpath: None,
            };
            let outcome =
                orchestrate::build(&Provider, ctx.layout(), now, &slice, &manifest_inputs, tree)
                    .await?;
            let body = BuildBody {
                slice: outcome.slice,
                target: outcome.target,
                status: outcome.status,
                findings: outcome.findings,
                warnings: outcome.warnings.into_iter().map(|warning| warning.title).collect(),
            };
            ctx.write(&body, |w, body| {
                writeln!(
                    w,
                    "built {} against {} ({} finding(s))",
                    body.slice, body.target, body.findings
                )?;
                for warning in &body.warnings {
                    writeln!(w, "warning: {warning}")?;
                }
                Ok(())
            })
        }
        Verb::Merge {
            slice,
            allow_composition_replace,
        } => {
            let classes = artifact_classes(&ctx.project_dir, &ctx.slices_dir().join(&slice));
            let outcome =
                orchestrate::merge(ctx.layout(), now, &slice, &classes, allow_composition_replace)?;
            let body = MergeBody {
                slice,
                merged: outcome.merged.into_iter().map(|entry| entry.name).collect(),
                decisions: outcome.decisions,
                archive_path: outcome.archive_path,
            };
            ctx.write(&body, |w, body| {
                writeln!(w, "merged {}", body.slice)?;
                for name in &body.merged {
                    writeln!(w, "spec: {name}")?;
                }
                for decision in &body.decisions {
                    writeln!(w, "decision: {decision}")?;
                }
                writeln!(w, "archived: {}", body.archive_path.display())
            })
        }
    }
}

/// Success envelope for the collapsed `source survey`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SurveyBody {
    source: String,
    adapter: String,
    leads: Vec<String>,
}

/// Success envelope for the collapsed `source extract`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ExtractBody {
    source: String,
    adapter: String,
    evidence: PathBuf,
}

/// Success envelope for the collapsed `slice build`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct BuildBody {
    slice: String,
    target: String,
    status: BuildStatus,
    findings: usize,
    warnings: Vec<String>,
}

/// Success envelope for the guest `slice merge run`.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct MergeBody {
    slice: String,
    merged: Vec<String>,
    decisions: Vec<String>,
    archive_path: PathBuf,
}
