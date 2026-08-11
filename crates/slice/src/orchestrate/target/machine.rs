//! The RFC-90 D1 engine-owned build phase machine:
//! [`Machine::execute`] drives one attempt through
//! `build → verify ⇄ repair → review ⇄ repair` and the terminal tail.

use std::path::Path;
use std::time::Instant;

use artifacts::atomic::bytes_write;
use diagnostics::Diagnostic;
use error::Error;
use jiff::Timestamp;
use project::adapter::ArtifactDeclaration;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{self, EventKind};
use project::seam::{
    BuildContext, DeferredRequirement, Input, PhaseReport, PhaseSource, RepairOrigin, Target,
    Workspace, Workspaces, seam_failure,
};
use project::snapshot::{CodePatch, SnapshotId};

use super::{BuildOutcome, workspace_failure};
use crate::build::attempt::{self, Attempt};
use crate::build::brief::repair_brief;
use crate::build::canonical::{self, Stamp};
use crate::build::gate::{self, PhaseOperation};
use crate::build::stage::{self, Stage};
use crate::{BuildReport, BuildStatus, LifecycleStatus, actions as slice_actions};

/// Maximum verification repair dispatches per attempt — an engine
/// constant (RFC-90 D1), never supplied by an adapter or model.
pub const MAX_VERIFICATION_REPAIRS: u32 = 3;

/// Maximum review remediation dispatches per attempt — an engine
/// constant (RFC-90 D1).
pub const MAX_REVIEW_REMEDIATIONS: u32 = 1;

/// Why the machine stopped short of a passing review.
enum Halt {
    /// A phase dispatch returned a seam error.
    Dispatch { operation: PhaseOperation, error: Error },
    /// An engine gate rejected the returned report, its staged
    /// writes, or the attempt-store bookkeeping around it.
    Gate { error: Error },
    /// A blocking terminal: build blocking findings, or an exhausted
    /// verification / review budget.
    Blocking { operation: PhaseOperation },
}

/// The rounds retained for terminal projection: the build report plus
/// the **latest** verify / review reports. Superseded rounds and every
/// repair report stay attempt-local evidence (RFC-90 D2).
#[derive(Default)]
struct Rounds {
    build: Option<PhaseReport>,
    verify: Option<PhaseReport>,
    review: Option<PhaseReport>,
    halt: Option<Halt>,
}

/// One build attempt's phase-machine state, constructed by the
/// orchestrator after the envelope (wave, workspace, attempt
/// directory, artifact stage) is in place.
pub(super) struct Machine<'a, S> {
    pub seam: &'a S,
    pub layout: Layout<'a>,
    pub now: Timestamp,
    /// Routed target id (`target:<name>`).
    pub id: String,
    pub slice: &'a str,
    pub slice_dir: &'a Path,
    /// The envelope wave's content-addressed digest.
    pub wave: SnapshotId,
    /// Engine identity stamped onto every canonicalized finding.
    pub stamp: Stamp<'a>,
    /// The target's declared `writable-artifacts[]` grants.
    pub grants: &'a [ArtifactDeclaration],
    /// The request's `deferred[]` exclusion set (RFC-86a D4): the
    /// coverage-claim gate input and, by digest, the `BuildRecord`'s
    /// consumed set.
    pub deferred: &'a [DeferredRequirement],
    /// The prepared product workspace with the artifact stage
    /// attached.
    pub workspace: Workspace,
    pub attempt: Attempt,
    pub stage: Stage,
    /// Build-dispatch inputs; consumed by the single `build` phase.
    pub inputs: Option<Vec<Input>>,
    /// Build-dispatch context; consumed by the single `build` phase.
    pub context: Option<BuildContext>,
    /// Phase ordinal within the attempt (1-based after the first
    /// dispatch).
    pub ordinal: u32,
}

impl<S: Target + Workspaces> Machine<'_, S> {
    /// Run the D1 transition algorithm and conclude the attempt:
    /// terminal-report projection, gates, capture, promotion,
    /// `BuildRecord`, and the `built` transition on success; a
    /// persisted failed terminal report and typed error on every
    /// other path.
    ///
    /// # Errors
    ///
    /// Dispatch failures return their seam diagnostics; gate
    /// rejections their `target-phase-*` / `target-build-*`
    /// discriminants; blocking terminals `target-build-failed`.
    pub(super) async fn execute(mut self) -> Result<BuildOutcome, Error> {
        let rounds = self.run().await;
        self.conclude(rounds).await
    }

    /// The exact D1 transition algorithm over the four operations.
    async fn run(&mut self) -> Rounds {
        let mut rounds = Rounds::default();
        match self.build_phase().await {
            Ok(report) => {
                // Fail fast on a deferred coverage claim (RFC-86a D4)
                // before spending verify / review dispatches; `commit`
                // re-runs the gate on the terminal report.
                let gated = report.enforce_deferred_not_covered(self.slice, self.deferred);
                let blocking = report.has_blocking();
                rounds.build = Some(report);
                if let Err(error) = gated {
                    rounds.halt = Some(Halt::Gate { error });
                    return rounds;
                }
                if blocking {
                    rounds.halt = Some(Halt::Blocking {
                        operation: PhaseOperation::Build,
                    });
                    return rounds;
                }
            }
            Err(halt) => {
                rounds.halt = Some(halt);
                return rounds;
            }
        }

        let mut verification_repairs = 0;
        let mut review_remediations = 0;
        loop {
            match self.verify_phase().await {
                Ok(report) => {
                    let brief = report.has_blocking().then(|| repair_brief(&report.findings));
                    rounds.verify = Some(report);
                    if let Some(brief) = brief {
                        if verification_repairs == MAX_VERIFICATION_REPAIRS {
                            rounds.halt = Some(Halt::Blocking {
                                operation: PhaseOperation::Verify,
                            });
                            return rounds;
                        }
                        // Blocking findings on the repair report are
                        // persisted evidence; the required verify that
                        // follows supersedes them for routing.
                        if let Err(halt) =
                            self.repair_phase(RepairOrigin::Verification, brief).await
                        {
                            rounds.halt = Some(halt);
                            return rounds;
                        }
                        verification_repairs += 1;
                        continue;
                    }
                }
                Err(halt) => {
                    rounds.halt = Some(halt);
                    return rounds;
                }
            }

            match self.review_phase().await {
                Ok(report) => {
                    let brief = report.has_blocking().then(|| repair_brief(&report.findings));
                    rounds.review = Some(report);
                    let Some(brief) = brief else {
                        return rounds;
                    };
                    if review_remediations == MAX_REVIEW_REMEDIATIONS {
                        rounds.halt = Some(Halt::Blocking {
                            operation: PhaseOperation::Review,
                        });
                        return rounds;
                    }
                    if let Err(halt) = self.repair_phase(RepairOrigin::Review, brief).await {
                        rounds.halt = Some(halt);
                        return rounds;
                    }
                    review_remediations += 1;
                    // A review-origin repair re-enters verification and
                    // consumes the shared verification budget from here.
                }
                Err(halt) => {
                    rounds.halt = Some(halt);
                    return rounds;
                }
            }
        }
    }

    async fn build_phase(&mut self) -> Result<PhaseReport, Halt> {
        let inputs = self.inputs.take().expect("build dispatches once");
        let context = self.context.take().expect("build dispatches once");
        let started = Instant::now();
        let result = self
            .seam
            .build(self.id.clone(), self.slice.to_string(), inputs, context, self.workspace.clone())
            .await;
        self.admit(PhaseOperation::Build, started, result)
    }

    async fn verify_phase(&mut self) -> Result<PhaseReport, Halt> {
        let started = Instant::now();
        let result = self.seam.verify(self.id.clone(), self.workspace.clone()).await;
        self.admit(PhaseOperation::Verify, started, result)
    }

    async fn repair_phase(
        &mut self, origin: RepairOrigin, findings: Vec<Diagnostic>,
    ) -> Result<PhaseReport, Halt> {
        let continuation = self.continuation()?;
        let started = Instant::now();
        let result = self
            .seam
            .repair(
                self.id.clone(),
                self.slice.to_string(),
                origin,
                findings,
                continuation,
                self.workspace.clone(),
            )
            .await;
        self.admit(PhaseOperation::Repair, started, result)
    }

    async fn review_phase(&mut self) -> Result<PhaseReport, Halt> {
        let continuation = self.continuation()?;
        let started = Instant::now();
        let result = self
            .seam
            .review(self.id.clone(), self.slice.to_string(), continuation, self.workspace.clone())
            .await;
        self.admit(PhaseOperation::Review, started, result)
    }

    /// The attempt-scoped continuation echoed to `repair` / `review`.
    fn continuation(&self) -> Result<Option<Vec<u8>>, Halt> {
        attempt::load_continuation(&self.attempt).map_err(|error| Halt::Gate { error })
    }

    /// Admit one returned phase report: run the D2 acceptance gate
    /// over the as-returned findings (before dedupe can collapse an
    /// incoherent source into a twin), then canonicalize, persist, and
    /// journal `slice.build.phase-completed` — a rejected report stays
    /// on disk as evidence — then enforce the staged-artifact grants
    /// after a mutating phase and apply the continuation replacement
    /// rule.
    fn admit(
        &mut self, operation: PhaseOperation, started: Instant,
        result: Result<PhaseReport, project::seam::Error>,
    ) -> Result<PhaseReport, Halt> {
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut report = result.map_err(|err| Halt::Dispatch {
            operation,
            error: seam_failure(operation_name(operation), &self.id, &err),
        })?;
        let gated = gate::accept(operation, &report);
        report.findings =
            canonical::canonicalize(std::mem::take(&mut report.findings), &self.stamp);

        self.ordinal += 1;
        let record = attempt::write_phase(&self.attempt, self.ordinal, operation, &report)
            .map_err(|error| Halt::Gate { error })?;
        journal::emit_best_effort(
            self.layout,
            self.now,
            EventKind::SliceBuildPhaseCompleted {
                slice_name: self.slice.into(),
                attempt: self.attempt.id,
                ordinal: self.ordinal,
                operation: operation.to_string(),
                source: report.source.to_string(),
                report_digest: record.digest,
                elapsed_ms,
            },
            "slice.build",
        );

        gated.map_err(|error| Halt::Gate { error })?;
        if operation != PhaseOperation::Verify {
            let changes = self.stage.diff().map_err(|error| Halt::Gate { error })?;
            stage::enforce_grants(&changes, self.grants).map_err(|error| Halt::Gate { error })?;
        }
        match report.next_continuation.as_deref() {
            None => {}
            Some([]) => {
                attempt::clear_continuation(&self.attempt).map_err(|error| Halt::Gate { error })?;
            }
            Some(bytes) => attempt::store_continuation(&self.attempt, bytes)
                .map_err(|error| Halt::Gate { error })?,
        }
        Ok(report)
    }

    /// Conclude the attempt from the retained rounds: project the
    /// terminal report, then commit (success) or persist the failed
    /// report and discard the stage (every other path).
    async fn conclude(&self, mut rounds: Rounds) -> Result<BuildOutcome, Error> {
        let verification = rounds.verify.as_ref().map(|report| report.source);
        let Some(halt) = rounds.halt.take() else {
            let report = self.assemble(BuildStatus::Success, &rounds, None);
            return match self.commit(&report).await {
                Ok(patch) => match self.promoted(&report, patch, verification) {
                    Ok(outcome) => Ok(outcome),
                    // Pre-record terminal-tail failures only. Once the
                    // record lands, `promoted` never returns `Err` —
                    // rewriting success as failure disagrees with it.
                    Err(error) => {
                        let finding = gate::engine_finding(
                            &error.variant_str(),
                            "post-promotion terminal tail failed",
                            &format!(
                                "the terminal tail failed after artifact promotion — the \
                                 promoted artifacts remain on the slice tree: {error}"
                            ),
                        );
                        self.fail(&rounds, Some(finding), error)
                    }
                },
                Err(error) => {
                    let finding = gate::engine_finding(
                        &error.variant_str(),
                        "terminal build gate failed",
                        &error.to_string(),
                    );
                    self.fail(&rounds, Some(finding), error)
                }
            };
        };
        let (finding, error) = match halt {
            Halt::Dispatch { operation, error } => (
                Some(gate::engine_finding(
                    "target-phase-dispatch-failed",
                    "phase dispatch failed",
                    &format!("the `{operation}` dispatch failed: {error}"),
                )),
                error,
            ),
            Halt::Gate { error } => (
                Some(gate::engine_finding(
                    &error.variant_str(),
                    "phase report rejected",
                    &error.to_string(),
                )),
                error,
            ),
            // The blocking findings themselves are terminal; no
            // engine-authored diagnostic is added.
            Halt::Blocking { operation } => (
                None,
                Error::Diag {
                    code: "target-build-failed",
                    detail: format!(
                        "slice `{}` build failed: blocking findings remain after `{operation}` \
                         (engine budgets: {MAX_VERIFICATION_REPAIRS} verification repairs, \
                         {MAX_REVIEW_REMEDIATIONS} review remediation)",
                        self.slice
                    ),
                },
            ),
        };
        self.fail(&rounds, finding, error)
    }

    /// The deterministic terminal-report projection (RFC-90 D2):
    /// outputs, UI surface, and the coverage claim only from the build
    /// report; findings the canonical union of the build report, the
    /// latest verify and review reports, and any engine-authored
    /// terminal finding.
    fn assemble(
        &self, status: BuildStatus, rounds: &Rounds, extra: Option<Diagnostic>,
    ) -> BuildReport {
        let mut findings = Vec::new();
        for report in [&rounds.build, &rounds.verify, &rounds.review].into_iter().flatten() {
            findings.extend(report.findings.iter().cloned());
        }
        findings.extend(extra);
        let findings = canonical::canonicalize(findings, &self.stamp);
        let (outputs, ui_surface, covered) = rounds
            .build
            .as_ref()
            .map(|report| (report.outputs.clone(), report.ui_surface, report.covered.clone()))
            .unwrap_or_default();
        BuildReport::stamped(
            &self.id,
            self.slice.to_string(),
            status,
            findings,
            outputs,
            ui_surface,
            covered,
        )
    }

    /// The post-promotion success tail: persist the terminal report
    /// (attempt copy + canonical projection), write the
    /// `BuildRecord`, discard the stage, and stamp `completed_at`.
    ///
    /// Once the `BuildRecord` lands, success is committed — stage
    /// discard and the lifecycle stamp are best-effort so a later
    /// failure cannot overwrite the canonical success report (plan
    /// progress and `Built` both project from the record).
    fn promoted(
        &self, report: &BuildReport, patch: CodePatch, verification: Option<PhaseSource>,
    ) -> Result<BuildOutcome, Error> {
        attempt::write_terminal(&self.attempt, report)?;
        self.project_canonical(report)?;
        let consumed = self.deferred.iter().map(|req| req.requirement_digest.clone()).collect();
        BuildRecord::from_capture(patch, self.wave.clone(), report.clone(), consumed)
            .write(self.slice_dir)?;
        stage::discard(&self.attempt.dir);
        if let Err(err) =
            slice_actions::transition(self.slice_dir, LifecycleStatus::Built, self.now)
        {
            tracing::warn!(
                "built timestamp stamp failed after BuildRecord write for slice `{}`: {err}",
                self.slice
            );
        }
        Ok(BuildOutcome {
            slice: self.slice.to_string(),
            target: report.target.clone(),
            status: report.status,
            findings: report.findings.len(),
            verification,
        })
    }

    /// The terminal success gates and irreversible tail head: blocking
    /// / deferred-coverage / output gates, staged-diff grant
    /// validation, workspace capture, and the transactional artifact
    /// promotion.
    async fn commit(&self, report: &BuildReport) -> Result<CodePatch, Error> {
        report.enforce_no_blocking()?;
        report.enforce_deferred_not_covered(self.deferred)?;
        // Declared outputs live in the private workspace until capture.
        report.enforce_outputs_exist(Path::new(&self.workspace.root))?;
        let changes = self.stage.diff()?;
        stage::enforce_grants(&changes, self.grants)?;
        let patch = self
            .seam
            .capture(self.workspace.id.clone())
            .await
            .map_err(|err| workspace_failure("capture", self.slice, &err))?;
        self.stage.promote(&changes, self.slice_dir)?;
        Ok(patch)
    }

    /// The orderly failure path: persist the failed terminal report
    /// (attempt copy + canonical projection, best-effort), discard the
    /// stage, and return the typed error. The product workspace is
    /// discarded by the envelope wrapper.
    fn fail(
        &self, rounds: &Rounds, extra: Option<Diagnostic>, error: Error,
    ) -> Result<BuildOutcome, Error> {
        let report = self.assemble(BuildStatus::Failure, rounds, extra);
        if let Err(err) = attempt::write_terminal(&self.attempt, &report) {
            tracing::warn!("failed terminal report write failed: {err}");
        }
        if let Err(err) = self.project_canonical(&report) {
            tracing::warn!("canonical report projection failed: {err}");
        }
        stage::discard(&self.attempt.dir);
        Err(error)
    }

    /// Atomically project the terminal body to the canonical
    /// `build/report.yaml` (RFC-90 D6).
    fn project_canonical(&self, report: &BuildReport) -> Result<(), Error> {
        let yaml = project::fs::yaml(report)?;
        bytes_write(&self.slice_dir.join("build").join("report.yaml"), yaml.as_bytes())
    }
}

/// The seam-failure operation label for one phase operation.
const fn operation_name(operation: PhaseOperation) -> &'static str {
    match operation {
        PhaseOperation::Build => "build",
        PhaseOperation::Verify => "verify",
        PhaseOperation::Repair => "repair",
        PhaseOperation::Review => "review",
    }
}
