// Contracts-build backend (RM-01 plan, C13).
//
// Drives the RM-01 cross-repo happy path through plan creation +
// deterministic loop driver, but uses the C13 per-slice phase-driver
// dispatch (`phaseDriverFor`) so the contract slice gets the
// `ContractsBuildPhaseDriver` (deterministic OpenAPI / JSON Schema
// emission) while implementation slices keep the standard
// `StubPhaseDriver`. Backend / mobile builds remain stubbed —
// C14a / C14b reserve real Omnia / Vectis specialist generation.
//
// Composition (matches the C10/C11 pattern documented in
// `backends/README.md` §Composition Pattern):
//
//   1. `prepare(ctx)` reuses `prepareScriptedHub` (setup + brief copy).
//   2. `invoke(ctx)` runs the C09 plan-creation sequence + C10
//      deterministic loop driver via the `ScriptedExecuteBackend`.
//      The loop driver is configured with `phaseDriverFor` so the
//      contract slice → `ContractsBuildPhaseDriver`,
//      everything else → `StubPhaseDriver`.
//   3. `teardown(ctx)` collects evidence (registry, plan snapshot,
//      workspace status, hub + project clone Git logs, fake-`gh` PR
//      state) — same shape as `scripted-execute`.
//
// **Boundary.** This backend is execute-only (no push/finalize).
// C13's scope is "real contracts build before implementation
// capabilities consume it"; landing-path coverage stays on
// `scripted-finalize` / `agent`. C14a/C14b can extend through
// finalize once Omnia / Vectis builds are deterministic.

import { ScriptedExecuteBackend } from "./scripted-execute.ts";
import { ContractsBuildPhaseDriver } from "./contracts-build-driver.ts";
import { StubPhaseDriver } from "./phase-driver.ts";
import { SLICE_CONTRACT } from "./scripted-shared.ts";
import type {
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

export class ContractsBuildBackend implements Backend {
  readonly name = "contracts-build" as const;
  private readonly inner: ScriptedExecuteBackend;

  constructor() {
    const stubDriver = new StubPhaseDriver();
    const contractsDriver = new ContractsBuildPhaseDriver();
    this.inner = new ScriptedExecuteBackend({
      phaseDriverFor: (entry) =>
        entry.name === SLICE_CONTRACT ? contractsDriver : stubDriver,
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
        `contracts-build backend (per-slice dispatch: contract slice → ` +
        `ContractsBuildPhaseDriver, implementation slices → StubPhaseDriver). ` +
        result.notes,
    };
  }

  teardown(ctx: RunContext): Promise<void> {
    return this.inner.teardown(ctx);
  }
}
