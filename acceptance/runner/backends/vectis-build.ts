// Vectis-build backend (RM-01 plan, C14b).
//
// Drives the RM-01 cross-repo happy path through plan creation +
// deterministic loop driver, but uses the C13 per-slice phase-driver
// dispatch (`phaseDriverFor`) so:
//
//   * the contract slice runs through the C13
//     `ContractsBuildPhaseDriver` (deterministic OpenAPI / JSON
//     Schema emission) — Vectis builds need real contract baseline
//     YAML to consume,
//   * Vectis-capability slices run through the new C14b
//     `VectisBuildPhaseDriver` (deterministic composition.yaml +
//     SwiftUI shell emission),
//   * everything else stays on the deterministic `StubPhaseDriver`.
//
// C14a runs the equivalent pattern for Omnia (backend) slices; a
// future "real builds for everything" backend composes all three
// drivers by chaining capability checks in `phaseDriverFor`.
//
// Composition (matches the C10/C11/C13/C14a pattern documented in
// `backends/README.md` §Composition Pattern):
//
//   1. `prepare(ctx)` reuses `prepareScriptedHub` (setup + brief copy).
//   2. `invoke(ctx)` runs the C09 plan-creation sequence + C10
//      deterministic loop driver via `ScriptedExecuteBackend`,
//      configured with the per-slice dispatch above.
//   3. `teardown(ctx)` collects evidence (registry, plan snapshot,
//      workspace status, hub + project clone Git logs, fake-`gh`
//      PR state) — same shape as `scripted-execute` /
//      `contracts-build` / `omnia-build`.
//
// **Boundary.** This backend is execute-only (no push/finalize),
// matching C13/C14a's contracts-/omnia-build boundaries. C14b's
// scope is "real Vectis build before broader downstream coverage";
// landing-path coverage stays on `scripted-finalize` / `agent`. A
// future amendment can extend through finalize once Omnia + Vectis
// builds are both deterministic.

import { ScriptedExecuteBackend } from "./scripted-execute.ts";
import { ContractsBuildPhaseDriver } from "./contracts-build-driver.ts";
import { VectisBuildPhaseDriver } from "./vectis-build-driver.ts";
import { StubPhaseDriver } from "./phase-driver.ts";
import { SLICE_CONTRACT } from "./scripted-shared.ts";
import type {
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

export class VectisBuildBackend implements Backend {
  readonly name = "vectis-build" as const;
  private readonly inner: ScriptedExecuteBackend;

  constructor() {
    const stubDriver = new StubPhaseDriver();
    const contractsDriver = new ContractsBuildPhaseDriver();
    const vectisDriver = new VectisBuildPhaseDriver();
    this.inner = new ScriptedExecuteBackend({
      phaseDriverFor: (entry) => {
        if (entry.name === SLICE_CONTRACT) return contractsDriver;
        if (entry.capability === "vectis") return vectisDriver;
        return stubDriver;
      },
    });
  }

  prepare(ctx: RunContext): Promise<void> {
    return this.inner.prepare(ctx);
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    const result = await this.inner.invoke(ctx);
    return {
      ...result,
      notes:
        `vectis-build backend (per-slice dispatch: contract slice → ` +
        `ContractsBuildPhaseDriver, vectis slices → VectisBuildPhaseDriver, ` +
        `other slices → StubPhaseDriver). ` +
        result.notes,
    };
  }

  teardown(ctx: RunContext): Promise<void> {
    return this.inner.teardown(ctx);
  }
}
