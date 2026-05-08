// Runner-owned `assertions` stage.
//
// Lifecycle position: runs between `backend.invoke()` and
// `backend.teardown()`. The dispatcher reads the scenario frontmatter's
// `assertions:` ids, looks up a handler for each id, and appends the
// resulting `AssertionRecord`s to whatever the backend already
// returned. The combined list is what `assertions.json` and
// `summary.md` end up rendering.
//
// Handlers live under `acceptance/assertions/` so they can be reused
// across backends without pulling in backend code. New ids register
// here, in the `defaultDispatch` map. Unknown ids surface as `skip`
// records — never silent — so missing handlers are visible in evidence.

import { skip as skipRecord } from "../assertions/types.ts";
import {
  assertFileExists,
  assertForbiddenPathsUntouched,
  assertVerifierStatus,
} from "../assertions/index.ts";
import type { AssertionContext } from "../assertions/types.ts";
import { setupHandlers, SETUP_ASSERTION_IDS } from "../assertions/setup.ts";
import {
  PLAN_ROLE_ASSERTION_IDS,
  planRoleHandlers,
} from "../assertions/plan-roles.ts";
import {
  EXECUTE_ASSERTION_IDS,
  executeHandlers,
} from "../assertions/execute.ts";
import {
  PUSH_FINALIZE_ASSERTION_IDS,
  pushFinalizeHandlers,
} from "../assertions/push-finalize.ts";
import {
  DEFINE_ASSERTION_IDS,
  defineHandlers,
} from "../assertions/define.ts";
import {
  CONTRACTS_BUILD_ASSERTION_IDS,
  contractsBuildHandlers,
} from "../assertions/contracts-build.ts";
import {
  OMNIA_BUILD_ASSERTION_IDS,
  omniaBuildHandlers,
} from "../assertions/omnia-build.ts";
import {
  VECTIS_BUILD_ASSERTION_IDS,
  vectisBuildHandlers,
} from "../assertions/vectis-build.ts";
import {
  RECORDED_ASSERTION_IDS,
  recordedHandlers,
} from "../assertions/recorded.ts";
import type { AssertionRecord, BackendResult, RunContext } from "./types.ts";

/**
 * Signature for an assertion handler registered in the dispatch table.
 *
 * `prior` is the list of records produced earlier in this run's
 * assertion stage. C09 (RM-01 plan-level outside-in) needs it so
 * downstream plan-* handlers can demote themselves to `skip` when an
 * upstream `setup-*` handler failed. Older handlers ignore the
 * argument.
 */
export type AssertionHandler = (
  id: string,
  ctx: RunContext,
  backendResult: BackendResult,
  prior: ReadonlyArray<AssertionRecord>,
) => Promise<AssertionRecord[]>;

/**
 * Default dispatch table. Each entry maps a scenario `assertions:` id to
 * a handler. Handlers are pure with respect to the runner: they only
 * read the on-disk workspace and the backend evidence the runner
 * already collected. Documented in
 * `acceptance/assertions/README.md` §Assertion Dispatch.
 *
 * `ctx` is consulted to decide whether to wire up cross-repo suite
 * handlers (`setup-*`, `plan-*`). When `ctx.setup` is unset we skip
 * those registrations so single-repo scenarios stay free of
 * cross-repo plumbing.
 */
export function defaultDispatch(ctx: RunContext): Map<string, AssertionHandler> {
  const map = new Map<string, AssertionHandler>();

  // Files-exist family: every path declared in `expected-artifacts`
  // must exist in the workspace as a regular file. Used by both the
  // primary `files-exist` id and the regression-path variant in
  // `update.md`.
  const filesExistHandler: AssertionHandler = async (id, ctx) => {
    const expected = ctx.scenario.frontmatter["expected-artifacts"] ?? [];
    if (expected.length === 0) {
      return [
        skipRecord(
          id,
          `Scenario declared '${id}' but supplied no 'expected-artifacts:' list.`,
          `no expected-artifacts in frontmatter`,
        ),
      ];
    }
    const out: AssertionRecord[] = [];
    for (const p of expected) {
      out.push(await assertFileExists(id, ctx.paths.workspace, p));
    }
    return out;
  };
  map.set("files-exist", filesExistHandler);
  map.set("regression-path-files-exist", filesExistHandler);

  // Forbidden contract YAML family: an implementation slice must not
  // emit anything under `contracts/**/*.yaml`. The same handler covers
  // both the negative-path "no contract YAML written" check and the
  // negative-path "no contract deltas merged into baseline" check —
  // they assert the same on-disk invariant from a fresh-project
  // workspace.
  const forbiddenContractsHandler: AssertionHandler = async (id, ctx) => {
    return [
      await assertForbiddenPathsUntouched(id, ctx.paths.workspace, [
        "contracts/**/*.yaml",
        "contracts/**/*.yml",
      ]),
    ];
  };
  map.set(
    "implementation-schema-emits-no-contract-yaml",
    forbiddenContractsHandler,
  );
  map.set(
    "implementation-slice-merges-contract-deltas-to-baseline",
    forbiddenContractsHandler,
  );

  // Contract verifier: real wiring lands with C13 (real contracts
  // build). Until then, the helper returns `skip` when the backend has
  // no captured stdout — visible in evidence rather than silently
  // passing.
  const verifierHandler: AssertionHandler = (id, ctx, backendResult) => {
    return Promise.resolve([
      assertVerifierStatus({
        id,
        contractsDir: `${ctx.paths.workspace}/contracts`,
        stdout: backendResult.evidence?.verifierStdout ?? "",
        expected: "clean",
      }),
    ]);
  };
  map.set("contract-validator-clean", verifierHandler);
  map.set("regression-path-contract-validator-clean", verifierHandler);

  // RM-01 cross-repo suite (C09): wire the four setup-* handlers and
  // the nine plan-* handlers when the run carries cross-repo state.
  // The handlers themselves live under `acceptance/assertions/` so
  // they can be reused without pulling in backend code; here we adapt
  // their `AssertionContext` shape into the runner's dispatch
  // signature.
  if (ctx.setup && ctx.specifyBin) {
    const setupInputs = {
      hubDir: ctx.setup.hubDir,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const setupMap = setupHandlers(setupInputs);
    for (const id of SETUP_ASSERTION_IDS) {
      const handler = setupMap[id];
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    const planInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const planMap = planRoleHandlers(planInputs);
    for (const id of PLAN_ROLE_ASSERTION_IDS) {
      const handler = planMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C10 execute-* family: handlers self-skip when
    // `ctx.executeState` is undefined (plan-only run), so they are
    // safe to register alongside the plan-* family for any cross-repo
    // suite that has setup state.
    const executeInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const executeMap = executeHandlers(executeInputs);
    for (const id of EXECUTE_ASSERTION_IDS) {
      const handler = executeMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C11 push/finalize-* family: handlers self-skip when
    // `ctx.finalizeState` is undefined (execute-only run), so they
    // are safe to register alongside the execute-* family for any
    // cross-repo suite that has setup state.
    const pushFinalizeInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const pushFinalizeMap = pushFinalizeHandlers(pushFinalizeInputs);
    for (const id of PUSH_FINALIZE_ASSERTION_IDS) {
      const handler = pushFinalizeMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C12 define-* family: per-slice define + merge stage handlers.
    // Self-skip when `ctx.executeState` is undefined so a plan-only
    // run (`scripted-plan`) cleanly demotes them. Registered behind
    // the same `ctx.setup && ctx.specifyBin` guard as the plan-* /
    // execute-* / push-finalize-* families.
    const defineInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const defineMap = defineHandlers(defineInputs);
    for (const id of DEFINE_ASSERTION_IDS) {
      const handler = defineMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C13 contracts-build family: per-slice contract YAML coverage.
    // Self-skips when `ctx.executeState` is undefined; for non-
    // contract-build backends (stub, scripted-execute) the contract
    // bundle is absent so the YAML / validator handlers fail with
    // `specialist-generation` — exactly the signal C13 expects when
    // the scenario is run with the wrong backend. Suites that do not
    // exercise contracts-build leave the ids out of their frontmatter.
    const contractsBuildInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const contractsBuildMap = contractsBuildHandlers(contractsBuildInputs);
    for (const id of CONTRACTS_BUILD_ASSERTION_IDS) {
      const handler = contractsBuildMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C14a omnia-build family: per-slice Omnia crate skeleton
    // coverage. Self-skips when `ctx.executeState` is undefined,
    // when no Omnia slice ran, or when upstream setup-* / plan-*
    // failed. For non-omnia-build backends (stub, scripted-execute,
    // contracts-build) the routed clone has no `crates/<crate>/`
    // tree so the handlers cleanly demote to skip — same "wrong
    // backend" signal pattern C13's contracts-build family uses.
    const omniaBuildInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const omniaBuildMap = omniaBuildHandlers(omniaBuildInputs);
    for (const id of OMNIA_BUILD_ASSERTION_IDS) {
      const handler = omniaBuildMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C14b vectis-build family: per-slice Vectis composition +
    // SwiftUI shell coverage. Self-skips when `ctx.executeState`
    // is undefined, when no Vectis slice ran, or when upstream
    // setup-* / plan-* failed. For non-vectis-build backends
    // (stub, scripted-execute, contracts-build, omnia-build) the
    // routed clone has no `composition.yaml` so the handlers
    // cleanly demote to skip — same "wrong backend" signal pattern
    // C13's contracts-build / C14a's omnia-build families use.
    const vectisBuildInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const vectisBuildMap = vectisBuildHandlers(vectisBuildInputs);
    for (const id of VECTIS_BUILD_ASSERTION_IDS) {
      const handler = vectisBuildMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
    // C15 recorded family: replay-outcome handlers. Self-skip when
    // `ctx.recordedEvidence` is undefined (the run was not driven by
    // the recorded backend) so other suites can leave the ids in
    // their assertion list without paying a penalty.
    const recordedInputs = {
      setup: ctx.setup,
      specifyBin: ctx.specifyBin,
      env: ctx.setup.env,
    };
    const recordedMap = recordedHandlers(recordedInputs);
    for (const id of RECORDED_ASSERTION_IDS) {
      const handler = recordedMap.get(id);
      if (!handler) continue;
      map.set(id, async (rid, rctx, _backendResult, prior) => {
        const ac = buildAssertionContext(rctx, prior);
        const res = await handler(rid, ac);
        return res.records;
      });
    }
  }

  return map;
}

/** Build the `AssertionContext` shape the suite handlers consume. */
function buildAssertionContext(
  ctx: RunContext,
  prior: ReadonlyArray<AssertionRecord>,
): AssertionContext {
  return {
    run: ctx,
    workspace: ctx.paths.workspace,
    prior,
  };
}

/**
 * Run the `assertions` stage. Returns the combined list of records
 * (backend records first, then dispatcher-emitted records) along with
 * a derived verdict the runner uses to upgrade `pending-operator` runs
 * to `passed`/`failed` when at least one helper produced a verdict.
 */
export async function runAssertions(
  ctx: RunContext,
  backendResult: BackendResult,
  dispatch: Map<string, AssertionHandler> = defaultDispatch(ctx),
): Promise<{ records: AssertionRecord[]; helperVerdict: HelperVerdict }> {
  const declared = orderForCascade(ctx.scenario.frontmatter.assertions ?? []);
  const helperRecords: AssertionRecord[] = [];

  for (const id of declared) {
    const handler = dispatch.get(id);
    if (!handler) {
      helperRecords.push(
        skipRecord(
          id,
          `No handler registered for assertion id '${id}'.`,
          `register a handler in acceptance/runner/assertions.ts`,
        ),
      );
      continue;
    }
    try {
      const recs = await handler(id, ctx, backendResult, helperRecords);
      helperRecords.push(...recs);
    } catch (e) {
      const msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      helperRecords.push({
        id,
        description: `Assertion handler threw before producing a record.`,
        verdict: "fail",
        evidence: msg,
        "fault-domain": "runner-setup",
      });
    }
  }

  const merged = mergeAssertions(backendResult.assertions, helperRecords);
  const helperVerdict = decideHelperVerdict(helperRecords);
  return { records: merged, helperVerdict };
}

/**
 * Merge backend-supplied records with dispatcher-emitted records.
 * Dispatcher records win when an id collides — the backend's
 * `pending-operator` placeholder must not mask a real verdict.
 */
function mergeAssertions(
  fromBackend: AssertionRecord[],
  fromDispatch: AssertionRecord[],
): AssertionRecord[] {
  const dispatchIds = new Set(fromDispatch.map((r) => r.id));
  const filtered = fromBackend.filter((r) => !dispatchIds.has(r.id));
  return [...filtered, ...fromDispatch];
}

/** Coarse verdict the assertion stage produced. */
export type HelperVerdict =
  | { kind: "no-helpers" }
  | { kind: "all-skip" }
  | { kind: "passed" }
  | { kind: "failed"; firstFailure: AssertionRecord };

/**
 * Reorder `setup-*` ids ahead of `plan-*` ids so the C09 cascade-skip
 * logic in the plan-role handlers reads the right `prior` records.
 * Other ids keep their declared order. Stable: pure ordering by
 * group key.
 */
function orderForCascade(ids: ReadonlyArray<string>): string[] {
  const groupOf = (id: string): number => {
    if (id.startsWith("setup-")) return 0;
    if (id === "plan-yaml-exists") return 1;
    if (id === "plan-validate-clean") return 2;
    if (id.startsWith("plan-")) return 3;
    // Role-based plan assertions (cosmetically grouped with plan-*).
    if (
      id === "backend-slice-routed-to-shop-backend" ||
      id === "mobile-slice-routed-to-shop-mobile" ||
      id === "implementation-slices-depend-on-contract" ||
      id === "contract-slice-projectless"
    ) return 3;
    if (
      id === "branch-prepared" ||
      id === "baseline-merge-commit-clean" ||
      id === "residue-commit-non-empty" ||
      id === "workspace-clean-before-push"
    ) return 4;
    // C12: define-* / merge-* family runs after execute-* so a
    // commit-shape failure upstream cascade-skips define checks.
    if (
      id === "slice-has-proposal" ||
      id === "slice-has-spec" ||
      id === "slice-has-design-when-required" ||
      id === "slice-has-tasks" ||
      id === "slice-baseline-promoted" ||
      id === "slice-archived" ||
      id === "implementation-slice-reads-baseline-contract"
    ) return 5;
    // C13 contract-build family: runs after define-* / merge-* so the
    // baseline-files-present check reads the post-merge tree, and
    // after execute-* so missing-baseline failures cascade-skip rather
    // than firing on every slice.
    if (
      id === "contract-slice-emits-yaml-artifacts" ||
      id === "contract-slice-yaml-validates-via-tool" ||
      id === "contract-slice-includes-openapi-or-asyncapi" ||
      id === "contract-slice-includes-required-schemas" ||
      id === "contract-baseline-files-present"
    ) return 5;
    // C14a omnia-build family: runs alongside the contract-build
    // family (also group 5) so a missing Omnia crate is reported
    // after define / merge but before push / finalize. Cascade-
    // skip semantics mirror contracts-build.
    if (
      id === "omnia-slice-emits-cargo-toml" ||
      id === "omnia-slice-emits-lib-rs" ||
      id === "omnia-slice-residue-under-routed-project" ||
      id === "omnia-slice-no-output-outside-project" ||
      id === "omnia-baseline-files-present"
    ) return 5;
    // C14b vectis-build family: runs alongside the contract /
    // omnia-build families (also group 5) so a missing Vectis
    // shell is reported after define / merge but before push /
    // finalize. Cascade-skip semantics mirror omnia-build.
    if (
      id === "vectis-slice-emits-composition-yaml" ||
      id === "vectis-slice-emits-screen-files" ||
      id === "vectis-slice-residue-under-routed-project" ||
      id === "vectis-slice-no-output-outside-project" ||
      id === "vectis-baseline-files-present"
    ) return 5;
    // C11: push-* before finalize-* so the finalize-runs-before-prs-merged
    // negative probe (which conceptually fits between push and finalize)
    // still reads execute-* failures correctly.
    if (id.startsWith("push-")) return 6;
    if (id === "finalize-runs-before-prs-merged") return 7;
    if (id.startsWith("finalize-")) return 8;
    // C15 recorded-trace family: last in the cascade because the
    // replay outcome is independent of the underlying suite assertions
    // (a recorded run emits skip records for setup-/plan-/etc.; the
    // recorded-* ids are what decide pass/fail).
    if (id.startsWith("recorded-trace-")) return 9;
    return 10;
  };
  return ids.map((id, i) => ({ id, i, g: groupOf(id) })).sort((a, b) =>
    a.g === b.g ? a.i - b.i : a.g - b.g
  ).map((e) => e.id);
}

function decideHelperVerdict(records: AssertionRecord[]): HelperVerdict {
  if (records.length === 0) return { kind: "no-helpers" };
  const failures = records.filter((r) => r.verdict === "fail");
  if (failures.length > 0) return { kind: "failed", firstFailure: failures[0] };
  const passes = records.filter((r) => r.verdict === "pass");
  if (passes.length === 0) return { kind: "all-skip" };
  return { kind: "passed" };
}
