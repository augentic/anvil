// Recorded transcript assertion handlers (RM-01 plan, C15).
//
// Three structural assertions cover the C15 recorded backend's verdict:
//
//   * `recorded-trace-replays-cleanly` — every CLI argv in the trace
//     replayed against the live binary with a matching exit code.
//     Sources its evidence from `BackendResult.evidence.extras.recorded`.
//   * `recorded-trace-final-state-matches` — when the trace carries a
//     trailing `recorded-trace-final-state` record, every declared
//     hub-relative path exists on disk after replay.
//   * `recorded-trace-no-extra-actions` — the live replay did not
//     issue any CLI actions beyond what the trace records. C15's
//     recorded backend is purely a replayer (it never invents
//     argvs), so today this handler effectively asserts that
//     `replayedCommandCount + syntheticSkippedCount === actionCount`
//     — a consistency check on the replay engine itself.
//
// All three handlers self-skip cleanly when the run was not driven by
// the recorded backend (no `extras.recorded` payload) so other suites
// can leave the ids in their assertion list without paying a penalty.

import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionResult,
} from "./types.ts";

import type { GitEnv } from "../runner/git.ts";
import type {
  RecordedEvidenceRef,
  SetupHubResult,
  SpecifyBin,
} from "../runner/types.ts";

/** Stable list of recorded assertion ids. */
export const RECORDED_ASSERTION_IDS = [
  "recorded-trace-replays-cleanly",
  "recorded-trace-final-state-matches",
  "recorded-trace-no-extra-actions",
] as const;

export type RecordedAssertionId = typeof RECORDED_ASSERTION_IDS[number];

/**
 * Inputs the recorded handlers consume. The runner threads
 * `setup` / `specifyBin` / `env` through `RunContext` already; the
 * handlers also need access to the run's `BackendResult.evidence`
 * which is fetched from the `AssertionContext.run` field.
 */
export interface RecordedAssertionInputs {
  setup: SetupHubResult;
  specifyBin: SpecifyBin;
  env: GitEnv;
}

/**
 * Build the dispatch map for the recorded family. Returns a partial
 * map (one handler per id) so the runner-side dispatcher can wire
 * each id without re-shaping the existing pattern.
 */
export function recordedHandlers(
  inputs: RecordedAssertionInputs,
): Map<RecordedAssertionId, AssertionHandler> {
  const map = new Map<RecordedAssertionId, AssertionHandler>();
  map.set("recorded-trace-replays-cleanly", replaysCleanly());
  map.set("recorded-trace-final-state-matches", finalStateMatches(inputs));
  map.set("recorded-trace-no-extra-actions", noExtraActions());
  return map;
}

// --- Handlers -------------------------------------------------------

function replaysCleanly(): AssertionHandler {
  return async (id: string, ctx: AssertionContext): Promise<AssertionResult> => {
    const evidence = readRecordedEvidence(ctx);
    if (!evidence) {
      return {
        records: [
          skip(
            id,
            `Run was not driven by the recorded backend; nothing to compare.`,
            `no extras.recorded payload — backend was '${ctx.run.scenario.frontmatter.backend}'`,
          ),
        ],
      };
    }
    if (evidence.replayedCommandCount === 0) {
      return {
        records: [
          skip(
            id,
            `Recorded trace contained no CLI commands to replay (${evidence.actionCount} synthetic record(s) only).`,
            `replayedCommandCount=0`,
          ),
        ],
      };
    }
    if (evidence.firstMismatch) {
      const mismatch = evidence.replayedActions.find(
        (r) => r.outcome === "mismatch" || r.outcome === "error",
      );
      const fault = mismatch?.faultDomain ?? "unknown";
      return {
        records: [
          fail(
            id,
            `Recorded replay diverged from the trace.`,
            { summary: evidence.firstMismatch },
            fault,
          ),
        ],
      };
    }
    return {
      records: [
        pass(
          id,
          `All ${evidence.replayedCommandCount} recorded CLI argv(s) replayed with matching exit codes.`,
          {
            summary: `replayedCommands=${evidence.replayedCommandCount}, ` +
              `syntheticSkipped=${evidence.syntheticSkippedCount}, ` +
              `trace=${evidence.tracePath}`,
          },
        ),
      ],
    };
  };
}

function finalStateMatches(
  inputs: RecordedAssertionInputs,
): AssertionHandler {
  return async (id: string, ctx: AssertionContext): Promise<AssertionResult> => {
    const evidence = readRecordedEvidence(ctx);
    if (!evidence) {
      return {
        records: [
          skip(
            id,
            `Run was not driven by the recorded backend.`,
            `no extras.recorded payload`,
          ),
        ],
      };
    }
    const finalState = evidence.finalState;
    if (!finalState || finalState.expectedPaths.length === 0) {
      return {
        records: [
          skip(
            id,
            `Trace did not declare a recorded-trace-final-state record; structural check skipped.`,
            `add { kind: "recorded-trace-final-state", expectedPaths: [...] } to the trace to enable this assertion`,
          ),
        ],
      };
    }
    const missing: string[] = [];
    for (const rel of finalState.expectedPaths) {
      const abs = join(inputs.setup.hubDir, rel);
      try {
        await Deno.stat(abs);
      } catch {
        missing.push(rel);
      }
    }
    if (missing.length > 0) {
      return {
        records: [
          fail(
            id,
            `${missing.length} recorded final-state path(s) missing under hub.`,
            { summary: `missing: ${missing.slice(0, 8).join(", ")}`, paths: missing },
            "live-agent-nondeterminism",
          ),
        ],
      };
    }
    return {
      records: [
        pass(
          id,
          `All ${finalState.expectedPaths.length} recorded final-state path(s) present after replay.`,
          {
            summary: `hub=${inputs.setup.hubDir}`,
            paths: finalState.expectedPaths,
          },
        ),
      ],
    };
  };
}

function noExtraActions(): AssertionHandler {
  return async (id: string, ctx: AssertionContext): Promise<AssertionResult> => {
    const evidence = readRecordedEvidence(ctx);
    if (!evidence) {
      return {
        records: [
          skip(
            id,
            `Run was not driven by the recorded backend.`,
            `no extras.recorded payload`,
          ),
        ],
      };
    }
    const accounted = evidence.replayedCommandCount + evidence.syntheticSkippedCount;
    if (accounted !== evidence.actionCount) {
      return {
        records: [
          fail(
            id,
            `Replay engine accounted for ${accounted} of ${evidence.actionCount} ` +
              `recorded actions — internal book-keeping drift.`,
            {
              summary:
                `actionCount=${evidence.actionCount}, ` +
                `replayed=${evidence.replayedCommandCount}, ` +
                `synthetic=${evidence.syntheticSkippedCount}`,
            },
            "runner-setup",
          ),
        ],
      };
    }
    return {
      records: [
        pass(
          id,
          `Replay accounted for every recorded action (${evidence.replayedCommandCount} replayed + ` +
            `${evidence.syntheticSkippedCount} synthetic = ${evidence.actionCount}).`,
          {
            summary:
              `actionCount=${evidence.actionCount}, replayedCommands=${evidence.replayedCommandCount}`,
          },
        ),
      ],
    };
  };
}

// --- Helpers --------------------------------------------------------

function readRecordedEvidence(
  ctx: AssertionContext,
): RecordedEvidenceRef | null {
  return ctx.run.recordedEvidence ?? null;
}
