// Scripted-plan backend (RM-01 plan, C09).
//
// Purpose: deterministically materialise the cross-repo plan structure
// the role-based assertions in `expected/plan-roles.md` score against,
// without depending on a live `/change:plan` runtime. The backend
// approximates what the planner would produce from the OAuth-login
// fixture brief by driving a fixed sequence of `specify change plan`
// CLI calls.
//
// **Important boundary.** This backend proves the *assertion plumbing*
// (setup → plan-shape → assertions) end to end against a deterministic
// baseline. It does NOT prove that `/change:plan` itself does the right
// thing on the same brief — that requires a real agent backend (the
// `agent` slot in `BackendName`, deferred to a future RM-15-ish chunk).
// The backend deliberately:
//   * does NOT read the body of the fixture brief,
//   * does NOT call any agent runtime,
//   * does NOT vary entry names or routing based on prose content.
//
// What it does:
//   1. `prepare(ctx)` — resolve `specify`, run `setupHub` (hub +
//      registered projects + fake `gh`), and copy the fixture brief
//      into the hub at `docs/oauth-login.md`. Stash the
//      `SetupHubResult` and `SpecifyBin` on `ctx` so the runner-owned
//      assertions stage can read hub state. Setup is the
//      `prepareScriptedHub` helper from `scripted-shared.ts`; both
//      `scripted-plan` (C09) and `scripted-execute` (C10) reuse it.
//   2. `invoke(ctx)` — run, in order:
//        a. `specify change create <change-name>`
//        b. `specify change plan create <change-name>`
//        c. `specify change plan add <contract-slice> --schema contracts@v1 ...`
//        d. `specify change plan add <backend-slice> --project shop-backend --depends-on <contract-slice> ...`
//        e. `specify change plan add <mobile-slice> --project shop-mobile --depends-on <contract-slice> ...`
//        f. `specify workspace sync` + `specify --format json workspace status`
//      The CLI exit codes alone decide whether `invoke` returns
//      `passed` (no records) or `failed` (with the `cli-substrate`
//      fault domain). The role-based assertions still drive the
//      workspace probe in the assertion stage.
//   3. `teardown(ctx)` — collects evidence (registry copy, plan
//      snapshot, workspace status, Git logs, fake-`gh` PR file dump)
//      via `collectEvidence` so the C06 inventory paths populate.
//
// Skip semantics:
//   * `findSpecifyBin` returns null  → `prepare` throws; the runner
//     wraps it as a `runner-setup` failure. The smoke driver is
//     responsible for the exit-0 skip when the binary is intentionally
//     absent (CI without the dev tool installed).
//   * `init --hub` / `registry add` non-zero → `cli-substrate`.
//
// CLI authority: every Specify state mutation goes through `specify`.
// The backend never hand-edits `.specify/`, `plan.yaml`, or
// `registry.yaml`.

import { join } from "jsr:@std/path@1";

import { collectEvidence } from "../evidence-collectors.ts";
import { appendLog } from "../evidence.ts";
import {
  CHANGE_NAME,
  HUB_NAME,
  prepareScriptedHub,
  readIfExists,
  runPlanCreationSequence,
  type ScriptedAction,
  type ScriptedHubState,
  SLICE_BACKEND,
  SLICE_CONTRACT,
  SLICE_MOBILE,
  syncAndProbeWorkspace,
} from "./scripted-shared.ts";
import type {
  Backend,
  BackendResult,
  RunContext,
  SetupHubResult,
} from "../types.ts";

/** @deprecated re-exported for backwards compatibility — use `ScriptedAction` from scripted-shared.ts. */
export type ScriptedPlanAction = ScriptedAction;

export interface ScriptedPlanEvidence {
  changeName: string;
  hubName: string;
  hubDir: string;
  briefHubPath: string | null;
  briefSourcePath: string | null;
  slices: { contract: string; backend: string; mobile: string };
  actions: ScriptedAction[];
}

export class ScriptedPlanBackend implements Backend {
  readonly name = "scripted-plan" as const;

  /**
   * State produced during `prepare` and read back in `invoke`/
   * `teardown`. Kept on the instance rather than in `ctx` so the
   * runner core does not have to know about the cross-repo backend's
   * internals beyond the promoted `RunContext.setup` field.
   */
  private state: {
    hub?: ScriptedHubState;
    actions: ScriptedAction[];
    workspaceStatusJson?: unknown;
  } = { actions: [] };

  async prepare(ctx: RunContext): Promise<void> {
    const hub = await prepareScriptedHub(ctx);
    this.state.hub = hub;
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    if (!this.state.hub) {
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: "scripted-plan backend invoked without setup state. prepare() did not run.",
        assertions: [],
      };
    }
    const { setup, bin } = this.state.hub;

    const planResult = await runPlanCreationSequence({
      bin,
      setup,
      actions: this.state.actions,
    });

    if (!planResult.ok) {
      return {
        verdict: "failed",
        faultDomain: "cli-substrate",
        notes:
          `scripted-plan backend: \`specify ${planResult.failingArgs.join(" ")}\` failed with exit ` +
          `${planResult.exitCode}. The plan was not fully assembled; downstream plan-* ` +
          `assertions will skip.`,
        assertions: [],
        evidence: {
          extras: {
            scriptedPlan: this.scriptedPlanEvidence(setup),
          },
        },
      };
    }

    this.state.workspaceStatusJson = await syncAndProbeWorkspace({
      ctx,
      bin,
      setup,
      actions: this.state.actions,
    });

    return {
      verdict: "passed",
      faultDomain: null,
      notes:
        `scripted-plan backend authored a deterministic 3-entry plan ` +
        `(contract=${SLICE_CONTRACT}, backend=${SLICE_BACKEND}, mobile=${SLICE_MOBILE}) ` +
        `against hub '${HUB_NAME}'. Role-based plan assertions decide the final verdict.`,
      assertions: [],
      evidence: {
        extras: {
          scriptedPlan: this.scriptedPlanEvidence(setup),
        },
      },
    };
  }

  async teardown(ctx: RunContext): Promise<void> {
    if (!this.state.hub) return;
    const { setup } = this.state.hub;
    try {
      await collectEvidence({
        runDir: ctx.paths.runDir,
        hubDir: setup.hubDir,
        projectDirs: setup.projectDirs,
        fakeGhStateDir: setup.fakeGhStateDir,
        env: setup.env,
        workspaceStatusJson: this.state.workspaceStatusJson,
        // Plan-only stage: snapshot plan.yaml under the C06 inventory
        // name. C10/C11 rotate this name once an execute/finalize
        // sequence runs.
        planYamlBeforeFinalize: await readIfExists(join(setup.hubDir, "plan.yaml")),
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[scripted-plan] evidence collection error: ${msg}\n`,
      );
    }
  }

  private scriptedPlanEvidence(setup: SetupHubResult): ScriptedPlanEvidence {
    return {
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
      actions: this.state.actions.slice(),
    };
  }
}
