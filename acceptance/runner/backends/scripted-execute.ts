// Scripted-execute backend (RM-01 plan, C10).
//
// Purpose: drive the RM-01 cross-repo happy path through plan creation
// AND a deterministic `/change:execute loop` equivalent, using stubbed
// phase outcomes so the execution mechanics can be tested without live
// agent generation.
//
// **Important boundary.** The actual `/change:execute loop` is a
// Cursor slash-command skill, not a CLI subcommand. Just like the C09
// `scripted-plan` backend stands in for `/change:plan`, this backend
// is the deterministic stand-in for the loop driver. It does NOT prove
// the loop skill itself does the right thing — that requires the
// reserved `agent` backend. What it DOES prove:
//
//   * the CLI substrate the loop skill composes is healthy
//     (`change plan next`, `workspace prepare-branch`, `change plan
//     transition`),
//   * the per-slice baseline / residue commit boundaries hold,
//   * routed slices land on `specify/<change>` branches in clean
//     workspaces,
//   * the plan reaches `all-done` with no failed/blocked entries.
//
// Architecture (C10 amendment §"Recommendation: composition"):
//
//   1. `prepare(ctx)` reuses `prepareScriptedHub` from
//      `scripted-shared.ts` — the same primitive `ScriptedPlanBackend`
//      uses. Setup is byte-for-byte identical between the two
//      backends.
//   2. `invoke(ctx)` runs `runPlanCreationSequence` (also shared) and
//      `syncAndProbeWorkspace`, then iterates `specify --format json
//      change plan next` until `all-done`. Each entry is dispatched to
//      `StubBackend.driveSlice` — the loop driver lives here, the stub
//      stays a passive lifecycle executor (C10 amendment §"loop driver
//      lives in C10").
//   3. `teardown(ctx)` collects evidence (registry, plan snapshot,
//      workspace status, hub + project clone Git logs, fake-`gh` PR
//      state) via `collectEvidence`. The plan-snapshot file is named
//      `plan.yaml.before-finalize` per the C06 evidence inventory; C11
//      will rotate the name once finalize lands.

import { join } from "jsr:@std/path@1";

import { collectEvidence } from "../evidence-collectors.ts";
import { appendLog } from "../evidence.ts";
import { runSpecifyJson, SpecifyCommandError } from "../specify-cli.ts";
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
import type { ScriptedPlanEvidence } from "./scripted-plan.ts";
import type {
  Backend,
  BackendResult,
  ExecuteState,
  RunContext,
  SetupHubResult,
  SliceInfo,
} from "../types.ts";

/**
 * Per-slice plan entry view the C13 amendment exposes to
 * `phaseDriverFor`. Carries everything the dispatch callback needs to
 * pick a driver without re-deriving it from the loop iteration.
 */
export interface PlanEntry {
  /** Plan-entry name (e.g. `oauth-login-contract`). */
  name: string;
  /** Routed project, or `null` for projectless contract slices. */
  project: string | null;
  /** Capability brief routing the slice (`contracts`, `omnia`, `vectis`). */
  capability?: string;
}

/** Constructor options for `ScriptedExecuteBackend` (C12 + C13 amendments). */
export interface ScriptedExecuteBackendOptions {
  /**
   * Per-slice phase-outcome producer the loop driver dispatches to
   * when `phaseDriverFor` is unset. Defaults to `new StubPhaseDriver()`
   * so existing smokes (C09/C10) keep their byte-for-byte behaviour.
   * The C12 `AgentBackend` plugs in `AgentPhaseDriver` instead.
   */
  phaseDriver?: PhaseDriver;
  /**
   * C13 amendment: per-slice phase-driver dispatch. When supplied,
   * the loop driver invokes this callback once per plan entry to
   * pick the driver to use for that slice. This lets the contract
   * slice get a `ContractsBuildPhaseDriver` while backend / mobile
   * slices keep the deterministic stub driver. When unset, the loop
   * uses `phaseDriver` (or `StubPhaseDriver` by default) for every
   * slice — backwards-compatible with the C09/C10/C11/C12 smokes.
   */
  phaseDriverFor?: (entry: PlanEntry) => PhaseDriver;
  /**
   * Per-slice capability name lookup. The driver forwards
   * `capabilityName` into `DriveSliceOpts` so the body factory can
   * decide whether to write `design.md`. The C12 `AgentBackend`
   * supplies a real lookup; `scripted-execute` defaults to a
   * static map matching the RM-01 fixture.
   */
  capabilityForSlice?: (sliceName: string) => string | undefined;
}

/**
 * Default capability lookup matching the RM-01 cross-repo fixture
 * (`shop-backend → omnia`, `shop-mobile → vectis`, contract slice
 * → `contracts`). Backends that mix RM-01 with other suites can pass
 * their own lookup via `ScriptedExecuteBackendOptions`.
 */
export function defaultCapabilityForSlice(
  sliceName: string,
): string | undefined {
  if (sliceName === SLICE_CONTRACT) return "contracts";
  if (sliceName === SLICE_BACKEND) return "omnia";
  if (sliceName === SLICE_MOBILE) return "vectis";
  return undefined;
}

/** Per-slice loop driver step persisted into evidence. */
export interface ScriptedExecuteStep {
  step: number;
  sliceName: string;
  project: string | null;
  preparedBranch: string | null;
  /** Number of stub actions recorded for this entry. */
  stubActionCount: number;
}

/** Evidence shape stored on `BackendResult.evidence.extras.scriptedExecute`. */
export interface ScriptedExecuteEvidence {
  changeName: string;
  hubName: string;
  hubDir: string;
  briefHubPath: string | null;
  briefSourcePath: string | null;
  slices: { contract: string; backend: string; mobile: string };
  /** CLI sequence the plan-creation phase executed. */
  planActions: ScriptedAction[];
  /** Per-entry loop driver steps. */
  loopSteps: ScriptedExecuteStep[];
  /** Final `change plan status` payload (for self-describing run dirs). */
  finalStatus?: unknown;
  /** Final `--format json change plan next` reason (e.g. `all-done`). */
  finalNextReason: string | null;
}

/** Strict-typed slice of `change plan next` JSON we consume. */
interface PlanNextJson {
  next: string | null;
  reason: string | null;
  project?: string | null;
  schema?: string | null;
  description?: string | null;
}

const MAX_LOOP_ITERATIONS = 32;

export class ScriptedExecuteBackend implements Backend {
  readonly name = "scripted-execute" as const;

  private readonly phaseDriver: PhaseDriver;
  private readonly phaseDriverFor:
    | ((entry: PlanEntry) => PhaseDriver)
    | undefined;
  private readonly capabilityForSlice: (sliceName: string) => string | undefined;

  private state: {
    hub?: ScriptedHubState;
    planActions: ScriptedAction[];
    loopSteps: ScriptedExecuteStep[];
    workspaceStatusJson?: unknown;
    finalStatus?: unknown;
    finalNextReason: string | null;
    routedProjects: string[];
    slices: SliceInfo[];
  } = {
    planActions: [],
    loopSteps: [],
    finalNextReason: null,
    routedProjects: [],
    slices: [],
  };

  constructor(options: ScriptedExecuteBackendOptions = {}) {
    this.phaseDriver = options.phaseDriver ?? new StubPhaseDriver();
    this.phaseDriverFor = options.phaseDriverFor;
    this.capabilityForSlice = options.capabilityForSlice ??
      defaultCapabilityForSlice;
  }

  async prepare(ctx: RunContext): Promise<void> {
    const hub = await prepareScriptedHub(ctx);
    this.state.hub = hub;
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    if (!this.state.hub) {
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes:
          "scripted-execute backend invoked without setup state. prepare() did not run.",
        assertions: [],
      };
    }
    const { setup, bin } = this.state.hub;

    // --- Phase A: deterministic plan creation (shared with C09) ----
    const planResult = await runPlanCreationSequence({
      bin,
      setup,
      actions: this.state.planActions,
    });
    if (!planResult.ok) {
      return {
        verdict: "failed",
        faultDomain: "cli-substrate",
        notes:
          `scripted-execute backend: plan creation failed at ` +
          `\`specify ${planResult.failingArgs.join(" ")}\` (exit ${planResult.exitCode}). ` +
          `Downstream plan-* and execute-* assertions will skip.`,
        assertions: [],
        evidence: this.evidence(setup),
      };
    }

    this.state.workspaceStatusJson = await syncAndProbeWorkspace({
      ctx,
      bin,
      setup,
      actions: this.state.planActions,
    });

    // --- Phase B: deterministic loop driver ------------------------
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
          `[scripted-execute] plan next returned '${sliceName}' twice; ` +
            `aborting to avoid infinite loop.\n`,
        );
        return {
          verdict: "failed",
          faultDomain: "cli-substrate",
          notes:
            `scripted-execute backend: \`specify change plan next\` returned ` +
            `'${sliceName}' a second time after a transition to done. ` +
            `Either the transition did not stick or the plan is malformed.`,
          assertions: [],
          evidence: this.evidence(setup),
        };
      }
      seen.add(sliceName);

      const project = typeof nextJson.project === "string" ? nextJson.project : null;
      const workspaceProjectDir = project
        ? join(setup.hubDir, ".specify", "workspace", project)
        : undefined;
      const residuePath = project ? RESIDUE_PATHS[sliceName] : undefined;
      if (project && !residuePath) {
        return {
          verdict: "failed",
          faultDomain: "runner-setup",
          notes:
            `scripted-execute backend: no residue path policy for routed slice ` +
            `'${sliceName}'. Add an entry to RESIDUE_PATHS in scripted-execute.ts.`,
          assertions: [],
          evidence: this.evidence(setup),
        };
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
      return {
        verdict: "failed",
        faultDomain: "cli-substrate",
        notes:
          `scripted-execute backend: loop driver hit the safety limit ` +
          `of ${MAX_LOOP_ITERATIONS} iterations without reaching all-done. ` +
          `This usually means \`specify change plan next\` is not advancing.`,
        assertions: [],
        evidence: this.evidence(setup),
      };
    }

    // Capture final status for evidence.
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
        `[scripted-execute] final change plan status probe failed: ${msg}\n`,
      );
    }

    // Refresh workspace status after execute (per C06 inventory: the
    // file should reflect post-execute state).
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
        `[scripted-execute] post-execute workspace status probe failed: ${msg}\n`,
      );
    }

    // Promote execute-state onto RunContext for the assertion stage.
    const executeState: ExecuteState = {
      changeName: CHANGE_NAME,
      hubDir: setup.hubDir,
      routedProjects: this.state.routedProjects.slice(),
      branch: `specify/${CHANGE_NAME}`,
      slices: this.state.slices.slice(),
    };
    (ctx as { executeState?: ExecuteState }).executeState = executeState;

    return {
      verdict: "passed",
      faultDomain: null,
      notes:
        `scripted-execute backend authored a 3-entry plan and drove it to ` +
        `${this.state.finalNextReason ?? "(unknown)"} ` +
        `(contract=${SLICE_CONTRACT}, backend=${SLICE_BACKEND}, mobile=${SLICE_MOBILE}, ` +
        `iterations=${iterations}). Role-based plan / execute-* assertions decide the final verdict.`,
      assertions: [],
      evidence: this.evidence(setup),
    };
  }

  async teardown(ctx: RunContext): Promise<void> {
    if (!this.state.hub) return;
    const { setup } = this.state.hub;

    // For C10 we want the post-execute Git logs from the routed
    // workspace clones, not the (untouched) source repos. Build a
    // projectDirs map that points at the workspace slots when they
    // exist, falling back to the source repos otherwise.
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
        planYamlBeforeFinalize: await readIfExists(join(setup.hubDir, "plan.yaml")),
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[scripted-execute] evidence collection error: ${msg}\n`,
      );
    }
  }

  private evidence(setup: SetupHubResult): {
    extras: { scriptedPlan: ScriptedPlanEvidence; scriptedExecute: ScriptedExecuteEvidence };
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
    return { extras: { scriptedPlan: planEv, scriptedExecute: execEv } };
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
        `scripted-execute backend: ${contextNote}` +
        (argsLine ? ` (\`specify ${argsLine}\` exit ${exit})` : "") +
        `. Last 50 lines in stderr.log. ${msg.slice(0, 600)}`,
      assertions: [],
      evidence: this.evidence(setup),
    };
  }
}
