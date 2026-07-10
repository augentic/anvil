//! The collapsed orchestrator verbs — `source survey` / `source
//! extract`, `slice build` / `slice refine` / `slice merge run`, and
//! `plan author` / `plan execute`.
//!
//! Each command drives the matching [`crate::orchestrate`] entry point
//! against the provider's seam: the deterministic verbs bound
//! [`Anchor`] alone, the judgment verbs additionally bound the seam
//! traits ([`omnia_guest::Model`], [`SourceSeam`], [`TargetSeam`]) they
//! consume — `ctx.provider` replaces the shim's concrete `&Provider`,
//! so the same `Handler` impl serves the wasm guest, the native dev
//! shim, and tests against a scripted mock.

use std::io::Write;
use std::path::PathBuf;

use error::Error;
use omnia_guest::Model;
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use super::{self as orchestrate, ExecuteOutcome};
use crate::change::{LoopStep, SourceBinding};
use crate::handler::{Anchor, Ctx, Out, Render};
use crate::merge::artifact_classes;
use crate::seam::{SourceSeam, TargetSeam, WorkingTree};
use crate::slice::BuildStatus;

/// The live shared mount every build applies against (deployments
/// share one live tree) — the caller-resolved working tree the native
/// prepare phase used to own.
fn live_tree() -> WorkingTree {
    WorkingTree {
        base: "live".to_string(),
        subpath: None,
    }
}

// ---------------------------------------------------------------------------
// source survey
// ---------------------------------------------------------------------------

/// Wire input for `source survey`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyInput {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Plan name guard; when set, must match `plan.yaml.name`.
    #[serde(default)]
    pub plan: Option<String>,
}

/// `specify source survey <source>` → [`orchestrate::survey`].
#[derive(Debug)]
pub struct Survey {
    input: SurveyInput,
}

impl<P: Anchor + SourceSeam> Handler<P> for Survey {
    type Error = crate::handler::Error;
    type Input = SurveyInput;
    type Output = Out<SurveyBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let outcome = orchestrate::survey(
            ctx.provider,
            cx.layout(),
            cx.now(),
            &self.input.source,
            self.input.plan.as_deref(),
        )
        .await?;
        Ok(Reply::ok(Out(SurveyBody {
            source: outcome.source,
            adapter: outcome.adapter,
            leads: outcome.leads,
        })))
    }
}

/// Success envelope for the collapsed `source survey`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SurveyBody {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Lead ids merged into `discovery.md`.
    pub leads: Vec<String>,
}

impl Render for SurveyBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "surveyed {} via {}", self.source, self.adapter)?;
        for lead in &self.leads {
            writeln!(w, "lead: {lead}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// source extract
// ---------------------------------------------------------------------------

/// Wire input for `source extract`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtractInput {
    /// Source key from `plan.yaml.sources.<key>`.
    pub source: String,
    /// Lead id from `discovery.md`.
    pub lead: String,
    /// Slice the Evidence is extracted into.
    pub slice: String,
}

/// `specify source extract <source> <lead> --slice <slice>` →
/// [`orchestrate::extract`].
#[derive(Debug)]
pub struct Extract {
    input: ExtractInput,
}

impl<P: Anchor + SourceSeam> Handler<P> for Extract {
    type Error = crate::handler::Error;
    type Input = ExtractInput;
    type Output = Out<ExtractBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let outcome = orchestrate::extract(
            ctx.provider,
            cx.layout(),
            cx.now(),
            &self.input.source,
            &self.input.lead,
            &self.input.slice,
        )
        .await?;
        Ok(Reply::ok(Out(ExtractBody {
            source: outcome.source,
            adapter: outcome.adapter,
            evidence: outcome.evidence,
        })))
    }
}

/// Success envelope for the collapsed `source extract`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtractBody {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Path of the persisted Evidence document.
    pub evidence: PathBuf,
}

impl Render for ExtractBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "extracted {} via {}", self.source, self.adapter)?;
        writeln!(w, "evidence: {}", self.evidence.display())
    }
}

// ---------------------------------------------------------------------------
// slice build
// ---------------------------------------------------------------------------

/// Wire input for `slice build`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// Slice name (under `.specify/slices/`).
    pub name: String,
}

/// `specify slice build <name>` → [`orchestrate::build`].
#[derive(Debug)]
pub struct Build {
    input: BuildInput,
}

impl<P: Anchor + TargetSeam> Handler<P> for Build {
    type Error = crate::handler::Error;
    type Input = BuildInput;
    type Output = Out<BuildBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let manifest_inputs = cx.resolve_target_adapter()?.manifest.inputs;
        let outcome = orchestrate::build(
            ctx.provider,
            cx.layout(),
            cx.now(),
            &self.input.name,
            &manifest_inputs,
            live_tree(),
        )
        .await?;
        Ok(Reply::ok(Out(BuildBody {
            slice: outcome.slice,
            target: outcome.target,
            status: outcome.status,
            findings: outcome.findings,
        })))
    }
}

/// Success envelope for the collapsed `slice build`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildBody {
    /// Slice name.
    pub slice: String,
    /// Target adapter identifier.
    pub target: String,
    /// Report status.
    pub status: BuildStatus,
    /// Finding count on the report.
    pub findings: usize,
}

impl Render for BuildBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "built {} against {} ({} finding(s))", self.slice, self.target, self.findings)
    }
}

// ---------------------------------------------------------------------------
// slice merge run
// ---------------------------------------------------------------------------

/// Wire input for `slice merge run`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergeRunInput {
    /// Slice name (under `.specify/slices/`).
    pub name: String,
    /// Authorise a whole-document composition overwrite.
    #[serde(default)]
    pub allow_composition_replace: bool,
}

/// `specify slice merge run <name>` → [`orchestrate::merge`]
/// (deterministic-only; grouped with the orchestrators so every
/// behavioural divergence between shims lives in one place).
#[derive(Debug)]
pub struct MergeRun {
    input: MergeRunInput,
}

impl<P: Anchor> Handler<P> for MergeRun {
    type Error = crate::handler::Error;
    type Input = MergeRunInput;
    type Output = Out<MergeBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let MergeRunInput {
            name,
            allow_composition_replace,
        } = self.input;
        let classes = artifact_classes(&cx.project_dir, &cx.slices_dir().join(&name));
        let outcome =
            orchestrate::merge(cx.layout(), cx.now(), &name, &classes, allow_composition_replace)?;
        Ok(Reply::ok(Out(MergeBody {
            slice: name,
            merged: outcome.merged.into_iter().map(|entry| entry.name).collect(),
            decisions: outcome.decisions,
            archive_path: outcome.archive_path,
        })))
    }
}

/// Success envelope for `slice merge run`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergeBody {
    /// Slice name.
    pub slice: String,
    /// Merged baseline spec names.
    pub merged: Vec<String>,
    /// Merge decisions recorded.
    pub decisions: Vec<String>,
    /// Path of the archived slice directory.
    pub archive_path: PathBuf,
}

impl Render for MergeBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "merged {}", self.slice)?;
        for name in &self.merged {
            writeln!(w, "spec: {name}")?;
        }
        for decision in &self.decisions {
            writeln!(w, "decision: {decision}")?;
        }
        writeln!(w, "archived: {}", self.archive_path.display())
    }
}

// ---------------------------------------------------------------------------
// plan author
// ---------------------------------------------------------------------------

/// Wire input for `plan author`. `--source` / `--intent` desugar to
/// the structured `sources` map at the CLI boundary — the same
/// `plan.yaml.sources` wire form `plan create` takes.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorInput {
    /// Kebab-case change name.
    pub name: String,
    /// Structured source bindings (`plan.yaml.sources` shape).
    #[serde(default)]
    pub sources: std::collections::BTreeMap<String, SourceBinding>,
}

/// `specify plan author <name> [--source ...]` →
/// [`orchestrate::author`] — the collapsed `/spec:plan` flow exiting
/// at `lifecycle: pending`.
#[derive(Debug)]
pub struct Author {
    input: AuthorInput,
}

impl<P: Anchor + Model + SourceSeam> Handler<P> for Author {
    type Error = crate::handler::Error;
    type Input = AuthorInput;
    type Output = Out<AuthorBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let outcome = orchestrate::author(
            ctx.provider,
            ctx.provider,
            cx.layout(),
            cx.now(),
            &self.input.name,
            self.input.sources,
        )
        .await?;
        Ok(Reply::ok(Out(AuthorBody {
            plan: outcome.plan,
            lifecycle: "pending",
            surveyed: outcome
                .surveyed
                .into_iter()
                .map(|surveyed| AuthorSurvey {
                    source: surveyed.source,
                    adapter: surveyed.adapter,
                    leads: surveyed.leads,
                })
                .collect(),
            slices: outcome.slices,
            hint: outcome.hint,
        })))
    }
}

/// Success envelope for the `plan author` exit at `pending`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorBody {
    /// Change name.
    pub plan: String,
    /// Always `pending` — Gate 1 stays operator-owned.
    pub lifecycle: &'static str,
    /// Surveyed sources in plan-binding order.
    pub surveyed: Vec<AuthorSurvey>,
    /// Authored slice names.
    pub slices: Vec<String>,
    /// The literal Gate 1 transition command.
    pub hint: String,
}

/// One surveyed source in the authoring fan-out.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthorSurvey {
    /// Source key.
    pub source: String,
    /// The bound adapter that answered.
    pub adapter: String,
    /// Lead ids the survey produced.
    pub leads: Vec<String>,
}

impl Render for AuthorBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for surveyed in &self.surveyed {
            writeln!(
                w,
                "surveyed {} via {} ({} lead(s))",
                surveyed.source,
                surveyed.adapter,
                surveyed.leads.len()
            )?;
        }
        for slice in &self.slices {
            writeln!(w, "slice: {slice}")?;
        }
        writeln!(w, "{}", self.hint)
    }
}

// ---------------------------------------------------------------------------
// slice refine
// ---------------------------------------------------------------------------

/// Wire input for `slice refine`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineInput {
    /// Slice name (a `plan.yaml.slices[]` entry).
    pub name: String,
}

/// `specify slice refine <name>` → [`orchestrate::refine_breakout`] —
/// the `/spec:refine` breakout outside the execute loop.
#[derive(Debug)]
pub struct Refine {
    input: RefineInput,
}

impl<P: Anchor + Model + SourceSeam + TargetSeam> Handler<P> for Refine {
    type Error = crate::handler::Error;
    type Input = RefineInput;
    type Output = Out<RefineBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let outcome = orchestrate::refine_breakout(
            ctx.provider,
            ctx.provider,
            ctx.provider,
            cx.layout(),
            cx.now(),
            &self.input.name,
        )
        .await?;
        Ok(Reply::ok(Out(RefineBody {
            slice: outcome.slice,
            artifacts: outcome.artifacts,
            extracted: outcome
                .extracted
                .into_iter()
                .map(|(source, lead)| RefineExtract { source, lead })
                .collect(),
        })))
    }
}

/// Success envelope for the `slice refine` breakout.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineBody {
    /// Slice name.
    pub slice: String,
    /// Written artifact names.
    pub artifacts: Vec<String>,
    /// Extracted `(source, lead)` pairs, in binding order.
    pub extracted: Vec<RefineExtract>,
}

/// One extracted `(source, lead)` pair.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineExtract {
    /// Source key.
    pub source: String,
    /// Lead id.
    pub lead: String,
}

impl Render for RefineBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "refined {}", self.slice)?;
        for extract in &self.extracted {
            writeln!(w, "extracted: {}/{}", extract.source, extract.lead)?;
        }
        for artifact in &self.artifacts {
            writeln!(w, "artifact: {artifact}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// plan execute
// ---------------------------------------------------------------------------

/// Wire input for `plan execute` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ExecuteInput {}

/// `specify plan execute` → [`orchestrate::execute`] — the drained
/// refine → build → merge loop over the approved plan.
#[derive(Clone, Copy, Debug)]
pub struct Execute;

impl<P: Anchor + Model + SourceSeam + TargetSeam> Handler<P> for Execute {
    type Error = crate::handler::Error;
    type Input = ExecuteInput;
    type Output = Out<ExecuteBody>;

    fn from_input(_: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let manifest_inputs = cx.resolve_target_adapter()?.manifest.inputs;
        let tree = live_tree();
        let outcome = orchestrate::execute(
            ctx.provider,
            ctx.provider,
            ctx.provider,
            cx.layout(),
            cx.now(),
            &manifest_inputs,
            &tree,
        )
        .await?;
        match outcome {
            ExecuteOutcome::Drained { phases } => Ok(Reply::ok(Out(ExecuteBody {
                status: "drained",
                phases: phases
                    .into_iter()
                    .map(|run| ExecutePhase {
                        slice: run.slice,
                        step: run.step,
                    })
                    .collect(),
            }))),
            // A stop is the loop's typed halt — surface it on the
            // error envelope (exit 2 / 422) so a driver can tell a
            // parked loop from a drained one without parsing prose.
            ExecuteOutcome::Stopped {
                reason,
                detail,
                hint,
                slice,
                ..
            } => Err(Error::validation_failed(
                "plan-execute-stopped",
                "the execute loop drains the plan",
                match (slice, detail) {
                    (Some(slice), Some(detail)) => {
                        format!("stop {reason} ({slice}): {detail} — {hint}")
                    }
                    (Some(slice), None) => format!("stop {reason} ({slice}) — {hint}"),
                    (None, Some(detail)) => format!("stop {reason}: {detail} — {hint}"),
                    (None, None) => format!("stop {reason} — {hint}"),
                },
            )
            .into()),
        }
    }
}

/// Success envelope for the `plan execute` drained exit.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExecuteBody {
    /// Always `drained` — a stop surfaces on the error envelope.
    pub status: &'static str,
    /// Completed phases in run order.
    pub phases: Vec<ExecutePhase>,
}

/// One completed phase in the drained run.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExecutePhase {
    /// Slice name.
    pub slice: String,
    /// Loop step (refine / build / merge).
    pub step: LoopStep,
}

impl Render for ExecuteBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for phase in &self.phases {
            writeln!(w, "{} {}", phase.step, phase.slice)?;
        }
        writeln!(w, "drained")
    }
}
