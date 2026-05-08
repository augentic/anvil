// Scripted-finalize backend (RM-01 plan, C11).
//
// Purpose: complete the RM-01 cross-repo happy path by composing the
// C09 plan-creation sequence + C10 deterministic loop driver with a
// fixed `workspace push` → fake-`gh` mark-merged → `change finalize`
// → idempotency probe sequence. Like `ScriptedExecuteBackend`, this
// backend is a deterministic stand-in for the post-execute landing
// path the operator would otherwise drive manually (or that a future
// `agent` backend will drive end-to-end).
//
// **Important boundary.** The orchestrate path of `/change:plan`
// would normally drive `workspace push` and (after operator merge)
// `change finalize`. This backend stands in for that orchestration —
// it does NOT prove the orchestrate skill itself does the right thing
// on the same brief. It DOES prove:
//
//   * the CLI substrate the orchestrate skill composes is healthy
//     (`workspace push`, `change finalize`),
//   * the JSON shapes those commands emit haven't drifted from what
//     the assertion handlers (and the Layer 0 substrate test) pin,
//   * the post-finalize archive layout matches what RM-01 expects,
//   * idempotency: a second `change finalize` returns
//     `error: plan-not-found`.
//
// Architecture (composition pattern from C10 amendment §"Composition
// Pattern (For C11 / Future Backends)"):
//
//   1. `prepare(ctx)` reuses `prepareScriptedHub` from
//      `scripted-shared.ts` — same primitive `ScriptedPlanBackend`
//      and `ScriptedExecuteBackend` use. Setup is byte-for-byte
//      identical across all three backends.
//   2. `invoke(ctx)` runs (in order):
//        a. plan creation via `runPlanCreationSequence` (C09 helper),
//        b. loop driver via a private helper extracted from C10 so
//           the backend doesn't subclass `ScriptedExecuteBackend`,
//        c. `specify --format json workspace push`, capture JSON,
//        d. (optional) negative pre-merge `change finalize` probe
//           for the `finalize-runs-before-prs-merged` expectation
//           — captured but never fails the suite,
//        e. mark every fake `gh` PR file as MERGED via
//           `markPrMerged` (C07 helper),
//        f. `specify --format json change finalize`, capture JSON,
//        g. second `specify --format json change finalize` for the
//           idempotency probe (expected non-zero with
//           `error: plan-not-found`).
//      Each captured JSON is also written into the run dir as a
//      file the assertion handlers reference (per the C06 evidence
//      inventory).
//   3. `teardown(ctx)` runs `collectEvidence` from C07. The plan
//      snapshot is captured BEFORE finalize moves the file — we
//      read `plan.yaml` once after the loop driver finishes and pass
//      that string through, so `plan.yaml.before-finalize` is
//      populated even though `plan.yaml` is gone post-finalize.
//
// CLI-authoritative invariant: every Specify state mutation goes
// through `specify`. The backend never hand-edits `.specify/`,
// `plan.yaml`, or the archive tree. The PR-state files under
// `gh-state/` ARE mutated directly (via `markPrMerged`) — that is
// the external-fake boundary, not Specify lifecycle state.

import { join } from "jsr:@std/path@1";

import { collectEvidence } from "../evidence-collectors.ts";
import { appendLog } from "../evidence.ts";
import { markPrMerged, readAllPrStates } from "../fake-gh.ts";
import {
  runSpecify,
  runSpecifyJson,
  SpecifyCommandError,
} from "../specify-cli.ts";
import type { SpecifyBin, SpecifyRun } from "../specify-cli.ts";
import { getWorkspaceStatus } from "../workspace-sync.ts";
import {
  CHANGE_NAME,
  HUB_NAME,
  prepareScriptedHub,
  readIfExists,
  RESIDUE_PATHS,
  runPlanCreationSequence,
  type ScriptedAction,
  type ScriptedHubState,
  SLICE_BACKEND,
  SLICE_CONTRACT,
  SLICE_MOBILE,
  syncAndProbeWorkspace,
} from "./scripted-shared.ts";
import { StubPhaseDriver } from "./phase-driver.ts";
import type { PhaseDriver } from "./phase-driver.ts";
import {
  defaultCapabilityForSlice,
  type PlanEntry,
  type ScriptedExecuteStep,
} from "./scripted-execute.ts";
import type { ScriptedPlanEvidence } from "./scripted-plan.ts";
import type { ScriptedExecuteEvidence } from "./scripted-execute.ts";
import type {
  Backend,
  BackendResult,
  ExecuteState,
  FinalizeState,
  RunContext,
  SetupHubResult,
  SliceInfo,
} from "../types.ts";

/** Constructor options for `ScriptedFinalizeBackend` (C12 + C13 amendments). */
export interface ScriptedFinalizeBackendOptions {
  /**
   * Per-slice phase-outcome producer the loop driver dispatches to
   * when `phaseDriverFor` is unset. Defaults to `new StubPhaseDriver()`
   * so existing C11 smokes keep their byte-for-byte behaviour. The
   * C12 `AgentBackend` plugs in `AgentPhaseDriver` instead.
   */
  phaseDriver?: PhaseDriver;
  /**
   * C13 amendment: per-slice phase-driver dispatch. When supplied,
   * the loop driver invokes this callback once per plan entry to
   * pick the driver to use. Lets the contract slice get a
   * `ContractsBuildPhaseDriver` while implementation slices stay on
   * the deterministic stub. When unset, the loop uses `phaseDriver`
   * for every slice — backwards-compatible with the C11/C12 smokes.
   */
  phaseDriverFor?: (entry: PlanEntry) => PhaseDriver;
  /** Per-slice capability name lookup; defaults to RM-01 fixture map. */
  capabilityForSlice?: (sliceName: string) => string | undefined;
}

const MAX_LOOP_ITERATIONS = 32;

/** Strict-typed slice of `change plan next` JSON we consume. */
interface PlanNextJson {
  next: string | null;
  reason: string | null;
  project?: string | null;
  schema?: string | null;
  description?: string | null;
}

/** Per-step record persisted into `BackendResult.evidence.extras.scriptedFinalize`. */
export interface ScriptedFinalizeStep {
  step: number;
  action: string;
  argv: string[];
  cwd: string;
  exitCode: number;
}

/** Evidence shape stored on `BackendResult.evidence.extras.scriptedFinalize`. */
export interface ScriptedFinalizeEvidence {
  changeName: string;
  hubName: string;
  hubDir: string;
  briefHubPath: string | null;
  briefSourcePath: string | null;
  slices: { contract: string; backend: string; mobile: string };
  /** Ordered CLI invocations specific to the C11 push/finalize phase. */
  finalizeSteps: ScriptedFinalizeStep[];
  /** PR numbers captured from the push-output JSON. */
  prNumbers: Record<string, number>;
  /** Repo keys (slug-encoded) of the fake `gh` PR files at end-of-push. */
  prRepoKeys: string[];
  /** Whether the optional pre-merge negative probe ran. */
  preMergeProbeRan: boolean;
  /** When `preMergeProbeRan`, true if the CLI refused the call. */
  finalizeRefusedPreMerge: boolean | null;
  /** Path under the run dir to the captured push JSON. */
  pushOutputJsonPath: string | null;
  /** Path under the run dir to the captured first-finalize JSON. */
  finalizeOutputJsonPath: string | null;
  /** Path under the run dir to the captured second-call finalize JSON. */
  finalizeSecondCallJsonPath: string | null;
  /** Path under the run dir to the captured pre-merge finalize JSON (if run). */
  finalizePreMergeJsonPath: string | null;
}

export class ScriptedFinalizeBackend implements Backend {
  readonly name = "scripted-finalize" as const;

  private readonly phaseDriver: PhaseDriver;
  private readonly phaseDriverFor:
    | ((entry: PlanEntry) => PhaseDriver)
    | undefined;
  private readonly capabilityForSlice: (sliceName: string) => string | undefined;

  constructor(options: ScriptedFinalizeBackendOptions = {}) {
    this.phaseDriver = options.phaseDriver ?? new StubPhaseDriver();
    this.phaseDriverFor = options.phaseDriverFor;
    this.capabilityForSlice = options.capabilityForSlice ??
      defaultCapabilityForSlice;
  }

  private state: {
    hub?: ScriptedHubState;
    /** Combined CLI sequence: plan + workspace + loop probes (C09/C10 shape). */
    planActions: ScriptedAction[];
    /** Per-iteration loop driver records (C10 shape). */
    loopSteps: ScriptedExecuteStep[];
    /** Per-step records specific to the C11 push/finalize phase. */
    finalizeSteps: ScriptedFinalizeStep[];
    workspaceStatusJson?: unknown;
    finalStatus?: unknown;
    finalNextReason: string | null;
    routedProjects: string[];
    slices: SliceInfo[];
    /** Snapshot of plan.yaml taken AFTER execute, BEFORE finalize moves it. */
    planYamlBeforeFinalize?: string;
    /** Push/finalize JSON captures. */
    pushOutput?: unknown;
    pushOutputJsonPath?: string;
    finalizeOutput?: unknown;
    finalizeOutputJsonPath?: string;
    finalizeSecondOutput?: unknown;
    finalizeSecondCallJsonPath?: string;
    finalizePreMergeOutput?: unknown;
    finalizePreMergeJsonPath?: string;
    finalizeRefusedPreMerge?: boolean;
    preMergeProbeRan: boolean;
    prNumbers: Record<string, number>;
    prRepoKeys: string[];
  } = {
    planActions: [],
    loopSteps: [],
    finalizeSteps: [],
    finalNextReason: null,
    routedProjects: [],
    slices: [],
    preMergeProbeRan: false,
    prNumbers: {},
    prRepoKeys: [],
  };

  async prepare(ctx: RunContext): Promise<void> {
    this.state.hub = await prepareScriptedHub(ctx);
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    if (!this.state.hub) {
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes:
          "scripted-finalize backend invoked without setup state. prepare() did not run.",
        assertions: [],
      };
    }
    const { setup, bin } = this.state.hub;

    // --- Phase A: deterministic plan creation (shared with C09) ---
    const planResult = await runPlanCreationSequence({
      bin,
      setup,
      actions: this.state.planActions,
    });
    if (!planResult.ok) {
      return this.bail(
        setup,
        "cli-substrate",
        `scripted-finalize backend: plan creation failed at ` +
          `\`specify ${planResult.failingArgs.join(" ")}\` (exit ${planResult.exitCode}). ` +
          `Downstream plan-*, execute-*, push-*, finalize-* assertions will skip.`,
      );
    }

    this.state.workspaceStatusJson = await syncAndProbeWorkspace({
      ctx,
      bin,
      setup,
      actions: this.state.planActions,
    });

    // --- Phase B: deterministic loop driver (C10 equivalent) ------
    const loopFailure = await this.runLoop(ctx, setup, bin);
    if (loopFailure) return loopFailure;

    // Capture final status + post-execute workspace status for evidence.
    try {
      const out = await runSpecifyJson({
        bin,
        cwd: setup.hubDir,
        args: ["change", "plan", "status"],
        env: setup.env,
      });
      this.state.finalStatus = out.json;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[scripted-finalize] final change plan status probe failed: ${msg}\n`,
      );
    }
    try {
      const { status } = await getWorkspaceStatus({
        bin,
        hubDir: setup.hubDir,
        env: setup.env,
      });
      this.state.workspaceStatusJson = status;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[scripted-finalize] post-execute workspace status probe failed: ${msg}\n`,
      );
    }

    // Promote execute-state onto RunContext for the C10 assertion stage.
    const executeState: ExecuteState = {
      changeName: CHANGE_NAME,
      hubDir: setup.hubDir,
      routedProjects: this.state.routedProjects.slice(),
      branch: `specify/${CHANGE_NAME}`,
      slices: this.state.slices.slice(),
    };
    (ctx as { executeState?: ExecuteState }).executeState = executeState;

    // Snapshot plan.yaml NOW (after execute, before finalize moves it).
    // ALSO write the snapshot to the run dir at the same path
    // `collectEvidence` would write to in teardown, so plan-* assertion
    // handlers can fall back to it via `resolvePlanPath` (see
    // `acceptance/assertions/plan-roles.ts`). teardown still calls
    // `collectEvidence` for parity; the second write is idempotent.
    this.state.planYamlBeforeFinalize = await readIfExists(
      join(setup.hubDir, "plan.yaml"),
    );
    if (this.state.planYamlBeforeFinalize !== undefined) {
      await Deno.writeTextFile(
        join(ctx.paths.runDir, "plan.yaml.before-finalize"),
        this.state.planYamlBeforeFinalize,
      );
    }

    // Initialise finalizeState early so partial-failure runs still
    // surface a populated FinalizeState (with empty prNumbers etc.).
    const finalizeState: FinalizeState = { prNumbers: {} };
    (ctx as { finalizeState?: FinalizeState }).finalizeState = finalizeState;

    // --- Phase C: workspace push -----------------------------------
    const pushFailure = await this.runWorkspacePush(ctx, setup, bin, finalizeState);
    if (pushFailure) return pushFailure;

    // --- Phase D: optional pre-merge negative probe ---------------
    // Best-effort: a failure here never fails the suite. The
    // `finalize-runs-before-prs-merged` handler reads the captured
    // output to decide pass / cli-substrate-finding.
    await this.runPreMergeProbe(ctx, setup, bin, finalizeState);

    // --- Phase E: mark fake PRs merged externally -----------------
    const markFailure = await this.markAllPrsMerged(ctx, setup);
    if (markFailure) return markFailure;

    // --- Phase F: change finalize ---------------------------------
    const finalizeFailure = await this.runChangeFinalize(ctx, setup, bin, finalizeState);
    if (finalizeFailure) return finalizeFailure;

    // --- Phase G: idempotency probe (second finalize) -------------
    await this.runSecondFinalize(ctx, setup, bin, finalizeState);

    return {
      verdict: "passed",
      faultDomain: null,
      notes:
        `scripted-finalize backend authored a 3-entry plan, drove it to ` +
        `${this.state.finalNextReason ?? "(unknown)"} ` +
        `(contract=${SLICE_CONTRACT}, backend=${SLICE_BACKEND}, mobile=${SLICE_MOBILE}), ` +
        `pushed ${Object.keys(this.state.prNumbers).length} routed clone(s), ` +
        `marked ${this.state.prRepoKeys.length} fake PR file(s) merged, then ran ` +
        `change finalize twice (second call expected non-zero with plan-not-found). ` +
        `Push/finalize-* assertions decide the final verdict.`,
      assertions: [],
      evidence: this.evidence(setup),
    };
  }

  async teardown(ctx: RunContext): Promise<void> {
    if (!this.state.hub) return;
    const { setup } = this.state.hub;

    // For C11 we want post-finalize Git logs from the routed
    // workspace clones (per the C06 evidence inventory). Mirror C10:
    // prefer the workspace slot when it still exists, falling back
    // to the source repo otherwise.
    const projectDirs: Record<string, string> = {};
    for (const [name, srcDir] of Object.entries(setup.projectDirs)) {
      const slot = join(setup.hubDir, ".specify", "workspace", name);
      try {
        const stat = await Deno.stat(slot);
        projectDirs[name] = stat.isDirectory ? slot : srcDir;
      } catch {
        projectDirs[name] = srcDir;
      }
    }

    try {
      await collectEvidence({
        runDir: ctx.paths.runDir,
        hubDir: setup.hubDir,
        projectDirs,
        fakeGhStateDir: setup.fakeGhStateDir,
        env: setup.env,
        workspaceStatusJson: this.state.workspaceStatusJson,
        // Snapshot taken pre-finalize so the C06 inventory's
        // `plan.yaml.before-finalize` file is populated even though
        // `plan.yaml` is gone from the live tree post-finalize.
        planYamlBeforeFinalize: this.state.planYamlBeforeFinalize,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[scripted-finalize] evidence collection error: ${msg}\n`,
      );
    }
  }

  // --- Loop driver ------------------------------------------------
  //
  // Same shape as `ScriptedExecuteBackend.invoke`'s phase B loop.
  // Kept inline rather than factored out to keep the cross-backend
  // surface narrow — the helpers in `scripted-shared.ts` cover the
  // setup + plan-creation surfaces both backends share, while the
  // loop's residue-path policy is the only thing C10 and C11 split
  // on (and the policy is identical here, mirroring `cross_repo.rs`).

  private async runLoop(
    ctx: RunContext,
    setup: SetupHubResult,
    bin: SpecifyBin,
  ): Promise<BackendResult | null> {
    const seen = new Set<string>();
    let iterations = 0;
    while (iterations < MAX_LOOP_ITERATIONS) {
      iterations += 1;
      let nextJson: PlanNextJson;
      try {
        const out = await runSpecifyJson<PlanNextJson>({
          bin,
          cwd: setup.hubDir,
          args: ["change", "plan", "next"],
          env: setup.env,
        });
        nextJson = out.json;
        this.state.planActions.push({
          step: this.state.planActions.length + 1,
          args: ["--format", "json", "change", "plan", "next"],
          cwd: setup.hubDir,
          exitCode: out.run.exitCode,
        });
      } catch (e) {
        return this.cliFailure(
          e,
          setup,
          `change plan next probe failed at iteration ${iterations}`,
        );
      }
      if (!nextJson.next) {
        this.state.finalNextReason = nextJson.reason ?? null;
        break;
      }
      const sliceName = nextJson.next;
      if (seen.has(sliceName)) {
        await appendLog(
          ctx.paths.stderrLog,
          `[scripted-finalize] plan next returned '${sliceName}' twice; ` +
            `aborting to avoid infinite loop.\n`,
        );
        return this.bail(
          setup,
          "cli-substrate",
          `scripted-finalize backend: \`specify change plan next\` returned ` +
            `'${sliceName}' a second time after a transition to done. ` +
            `Either the transition did not stick or the plan is malformed.`,
        );
      }
      seen.add(sliceName);

      const project = typeof nextJson.project === "string" ? nextJson.project : null;
      const workspaceProjectDir = project
        ? join(setup.hubDir, ".specify", "workspace", project)
        : undefined;
      const residuePath = project ? RESIDUE_PATHS[sliceName] : undefined;
      if (project && !residuePath) {
        return this.bail(
          setup,
          "runner-setup",
          `scripted-finalize backend: no residue path policy for routed slice ` +
            `'${sliceName}'. Add an entry to RESIDUE_PATHS in scripted-finalize.ts.`,
        );
      }

      try {
        const capabilityName = this.capabilityForSlice(sliceName);
        const driver = this.phaseDriverFor
          ? this.phaseDriverFor({
            name: sliceName,
            project,
            capability: capabilityName,
          })
          : this.phaseDriver;
        const drive = await driver.driveSlice({
          bin,
          env: setup.env,
          hubDir: setup.hubDir,
          sliceName,
          project,
          workspaceProjectDir,
          changeName: CHANGE_NAME,
          residuePath,
          capabilityName,
        });
        this.state.loopSteps.push({
          step: this.state.loopSteps.length + 1,
          sliceName,
          project,
          preparedBranch: drive.preparedBranch,
          stubActionCount: drive.actions.length,
        });
        this.state.slices.push({
          name: sliceName,
          project,
          capability: capabilityName,
        });
        if (project && !this.state.routedProjects.includes(project)) {
          this.state.routedProjects.push(project);
        }
      } catch (e) {
        return this.cliFailure(
          e,
          setup,
          `driveSlice failed for entry '${sliceName}'`,
        );
      }
    }

    if (iterations >= MAX_LOOP_ITERATIONS) {
      return this.bail(
        setup,
        "cli-substrate",
        `scripted-finalize backend: loop driver hit the safety limit ` +
          `of ${MAX_LOOP_ITERATIONS} iterations without reaching all-done.`,
      );
    }
    return null;
  }

  // --- Push -------------------------------------------------------

  private async runWorkspacePush(
    ctx: RunContext,
    setup: SetupHubResult,
    bin: SpecifyBin,
    finalizeState: FinalizeState,
  ): Promise<BackendResult | null> {
    let pushRun: SpecifyRun;
    let pushJson: unknown;
    try {
      const { run, json } = await runSpecifyJson({
        bin,
        cwd: setup.hubDir,
        args: ["workspace", "push"],
        env: setup.env,
      });
      pushRun = run;
      pushJson = json;
    } catch (e) {
      return this.cliFailure(
        e,
        setup,
        `workspace push failed`,
      );
    }
    this.recordFinalizeStep("workspace-push", pushRun);

    // Persist push JSON as captured evidence. The path lives in the
    // run dir (per the C06 inventory) so handlers can reference it.
    const pushPath = join(ctx.paths.runDir, "push-output.json");
    await Deno.writeTextFile(pushPath, JSON.stringify(pushJson, null, 2) + "\n");

    this.state.pushOutput = pushJson;
    this.state.pushOutputJsonPath = pushPath;
    finalizeState.pushOutput = pushJson;
    finalizeState.pushOutputJson = pushPath;

    // Extract per-project PR numbers for the assertion handlers.
    this.state.prNumbers = extractPrNumbers(pushJson);
    finalizeState.prNumbers = { ...this.state.prNumbers };

    // Snapshot the post-push PR file repo keys for evidence
    // (slug-encoded names like `shop_shop-backend`).
    const prStates = await readAllPrStates(setup.fakeGhStateDir);
    this.state.prRepoKeys = prStates.map((s) => s.repoKey);
    return null;
  }

  // --- Optional pre-merge negative probe -------------------------

  private async runPreMergeProbe(
    ctx: RunContext,
    setup: SetupHubResult,
    bin: SpecifyBin,
    finalizeState: FinalizeState,
  ): Promise<void> {
    this.state.preMergeProbeRan = true;
    let exitCode = 0;
    let captured: unknown = null;
    try {
      const { run, json } = await runSpecifyJson({
        bin,
        cwd: setup.hubDir,
        args: ["change", "finalize"],
        env: setup.env,
      });
      exitCode = run.exitCode;
      captured = json;
      this.recordFinalizeStep("pre-merge-finalize-probe", run);
    } catch (e) {
      if (e instanceof SpecifyCommandError) {
        exitCode = e.run.exitCode;
        // Try to parse stdout — finalize emits JSON even on refusal.
        try {
          captured = JSON.parse(e.run.stdout);
        } catch {
          captured = { "raw-stdout": e.run.stdout, "raw-stderr": e.run.stderr };
        }
        this.recordFinalizeStep("pre-merge-finalize-probe", e.run);
      } else {
        const msg = e instanceof Error ? e.message : String(e);
        await appendLog(
          ctx.paths.stderrLog,
          `[scripted-finalize] pre-merge probe non-fatal error: ${msg}\n`,
        );
        return;
      }
    }
    const path = join(ctx.paths.runDir, "finalize-output.pre-merge.json");
    await Deno.writeTextFile(
      path,
      JSON.stringify({ "exit-code": exitCode, output: captured }, null, 2) + "\n",
    );
    this.state.finalizePreMergeOutput = captured;
    this.state.finalizePreMergeJsonPath = path;
    this.state.finalizeRefusedPreMerge = exitCode !== 0;
    finalizeState.finalizePreMergeOutput = captured;
    finalizeState.finalizePreMergeJson = path;
    finalizeState.finalizeRefusedPreMerge = exitCode !== 0;
  }

  // --- Mark all fake PRs merged ----------------------------------

  private async markAllPrsMerged(
    ctx: RunContext,
    setup: SetupHubResult,
  ): Promise<BackendResult | null> {
    let states: Awaited<ReturnType<typeof readAllPrStates>>;
    try {
      states = await readAllPrStates(setup.fakeGhStateDir);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return this.bail(
        setup,
        "external-fake-boundary",
        `scripted-finalize backend: failed to read fake-gh PR-state dir ` +
          `${setup.fakeGhStateDir}: ${msg}`,
      );
    }
    if (states.length === 0) {
      return this.bail(
        setup,
        "external-fake-boundary",
        `scripted-finalize backend: no fake-gh PR-state files found in ` +
          `${setup.fakeGhStateDir} after \`workspace push\`. ` +
          `The push call did not create PR records.`,
      );
    }
    for (const s of states) {
      try {
        await markPrMerged({
          stateDir: setup.fakeGhStateDir,
          // Use the slug-encoded repoKey directly; it round-trips
          // through `repoKeyForName` unchanged.
          repo: s.repoKey,
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        await appendLog(
          ctx.paths.stderrLog,
          `[scripted-finalize] markPrMerged(${s.repoKey}) failed: ${msg}\n`,
        );
      }
    }
    this.recordFinalizeStep(
      "mark-prs-merged",
      {
        args: ["<runner: markPrMerged>", ...states.map((s) => s.repoKey)],
        cwd: setup.fakeGhStateDir,
        exitCode: 0,
      },
    );
    return null;
  }

  // --- Finalize ---------------------------------------------------

  private async runChangeFinalize(
    ctx: RunContext,
    setup: SetupHubResult,
    bin: SpecifyBin,
    finalizeState: FinalizeState,
  ): Promise<BackendResult | null> {
    let finalizeRun: SpecifyRun;
    let finalizeJson: unknown;
    try {
      const { run, json } = await runSpecifyJson({
        bin,
        cwd: setup.hubDir,
        args: ["change", "finalize"],
        env: setup.env,
      });
      finalizeRun = run;
      finalizeJson = json;
    } catch (e) {
      return this.cliFailure(e, setup, `change finalize (first call) failed`);
    }
    this.recordFinalizeStep("change-finalize", finalizeRun);

    const path = join(ctx.paths.runDir, "finalize-output.json");
    await Deno.writeTextFile(path, JSON.stringify(finalizeJson, null, 2) + "\n");
    this.state.finalizeOutput = finalizeJson;
    this.state.finalizeOutputJsonPath = path;
    finalizeState.finalizeOutput = finalizeJson;
    finalizeState.finalizeOutputJson = path;
    return null;
  }

  // --- Idempotency probe -----------------------------------------

  private async runSecondFinalize(
    ctx: RunContext,
    setup: SetupHubResult,
    bin: SpecifyBin,
    finalizeState: FinalizeState,
  ): Promise<void> {
    let exitCode = 0;
    let captured: unknown = null;
    try {
      const { run, json } = await runSpecifyJson({
        bin,
        cwd: setup.hubDir,
        args: ["change", "finalize"],
        env: setup.env,
      });
      exitCode = run.exitCode;
      captured = json;
      this.recordFinalizeStep("change-finalize-second-call", run);
    } catch (e) {
      if (e instanceof SpecifyCommandError) {
        exitCode = e.run.exitCode;
        try {
          captured = JSON.parse(e.run.stdout);
        } catch {
          captured = { "raw-stdout": e.run.stdout, "raw-stderr": e.run.stderr };
        }
        this.recordFinalizeStep("change-finalize-second-call", e.run);
      } else {
        const msg = e instanceof Error ? e.message : String(e);
        await appendLog(
          ctx.paths.stderrLog,
          `[scripted-finalize] second finalize non-fatal error: ${msg}\n`,
        );
        return;
      }
    }
    const path = join(ctx.paths.runDir, "finalize-output.second-call.json");
    await Deno.writeTextFile(
      path,
      JSON.stringify({ "exit-code": exitCode, output: captured }, null, 2) + "\n",
    );
    this.state.finalizeSecondOutput = captured;
    this.state.finalizeSecondCallJsonPath = path;
    finalizeState.finalizeSecondOutput = captured;
    finalizeState.finalizeSecondCallJson = path;
  }

  // --- Helpers ----------------------------------------------------

  private recordFinalizeStep(action: string, run: Pick<SpecifyRun, "args" | "cwd" | "exitCode">): void {
    this.state.finalizeSteps.push({
      step: this.state.finalizeSteps.length + 1,
      action,
      argv: run.args,
      cwd: run.cwd,
      exitCode: run.exitCode,
    });
  }

  private evidence(setup: SetupHubResult): {
    extras: {
      scriptedPlan: ScriptedPlanEvidence;
      scriptedExecute: ScriptedExecuteEvidence;
      scriptedFinalize: ScriptedFinalizeEvidence;
    };
  } {
    const planEv: ScriptedPlanEvidence = {
      changeName: CHANGE_NAME,
      hubName: HUB_NAME,
      hubDir: setup.hubDir,
      briefHubPath: this.state.hub?.briefHubPath ?? null,
      briefSourcePath: this.state.hub?.briefSourcePath ?? null,
      slices: {
        contract: SLICE_CONTRACT,
        backend: SLICE_BACKEND,
        mobile: SLICE_MOBILE,
      },
      actions: this.state.planActions.slice(),
    };
    const execEv: ScriptedExecuteEvidence = {
      changeName: CHANGE_NAME,
      hubName: HUB_NAME,
      hubDir: setup.hubDir,
      briefHubPath: this.state.hub?.briefHubPath ?? null,
      briefSourcePath: this.state.hub?.briefSourcePath ?? null,
      slices: {
        contract: SLICE_CONTRACT,
        backend: SLICE_BACKEND,
        mobile: SLICE_MOBILE,
      },
      planActions: this.state.planActions.slice(),
      loopSteps: this.state.loopSteps.slice(),
      finalStatus: this.state.finalStatus,
      finalNextReason: this.state.finalNextReason,
    };
    const finEv: ScriptedFinalizeEvidence = {
      changeName: CHANGE_NAME,
      hubName: HUB_NAME,
      hubDir: setup.hubDir,
      briefHubPath: this.state.hub?.briefHubPath ?? null,
      briefSourcePath: this.state.hub?.briefSourcePath ?? null,
      slices: {
        contract: SLICE_CONTRACT,
        backend: SLICE_BACKEND,
        mobile: SLICE_MOBILE,
      },
      finalizeSteps: this.state.finalizeSteps.slice(),
      prNumbers: { ...this.state.prNumbers },
      prRepoKeys: this.state.prRepoKeys.slice(),
      preMergeProbeRan: this.state.preMergeProbeRan,
      finalizeRefusedPreMerge: this.state.finalizeRefusedPreMerge ?? null,
      pushOutputJsonPath: this.state.pushOutputJsonPath ?? null,
      finalizeOutputJsonPath: this.state.finalizeOutputJsonPath ?? null,
      finalizeSecondCallJsonPath: this.state.finalizeSecondCallJsonPath ?? null,
      finalizePreMergeJsonPath: this.state.finalizePreMergeJsonPath ?? null,
    };
    return {
      extras: {
        scriptedPlan: planEv,
        scriptedExecute: execEv,
        scriptedFinalize: finEv,
      },
    };
  }

  private bail(
    setup: SetupHubResult,
    faultDomain: BackendResult["faultDomain"],
    notes: string,
  ): BackendResult {
    return {
      verdict: "failed",
      faultDomain,
      notes,
      assertions: [],
      evidence: this.evidence(setup),
    };
  }

  private cliFailure(
    e: unknown,
    setup: SetupHubResult,
    contextNote: string,
  ): BackendResult {
    let msg: string;
    let exit: number | undefined;
    let argsLine = "";
    if (e instanceof SpecifyCommandError) {
      msg = e.message;
      exit = e.run.exitCode;
      argsLine = e.run.args.join(" ");
    } else {
      msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    }
    return {
      verdict: "failed",
      faultDomain: "cli-substrate",
      notes:
        `scripted-finalize backend: ${contextNote}` +
        (argsLine ? ` (\`specify ${argsLine}\` exit ${exit})` : "") +
        `. Last 50 lines in stderr.log. ${msg.slice(0, 600)}`,
      assertions: [],
      evidence: this.evidence(setup),
    };
  }
}

/**
 * Pull `prNumbers` out of the `workspace push --format json` payload.
 * Pinned to the field name `pr` per the Layer 0 substrate test
 * (`cross_repo.rs::push_workspace`). Falls back to an empty record
 * silently — the assertion handlers surface the shape problem with a
 * `cli-substrate` finding rather than crashing the backend.
 */
function extractPrNumbers(pushJson: unknown): Record<string, number> {
  const out: Record<string, number> = {};
  if (!pushJson || typeof pushJson !== "object") return out;
  const projects = (pushJson as { projects?: unknown }).projects;
  if (!Array.isArray(projects)) return out;
  for (const p of projects) {
    if (!p || typeof p !== "object") continue;
    const r = p as Record<string, unknown>;
    if (typeof r.name !== "string") continue;
    if (typeof r.pr === "number") {
      out[r.name] = r.pr;
    }
  }
  return out;
}
