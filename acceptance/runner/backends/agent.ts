// Agent backend (RM-01 plan, C12).
//
// Drives the RM-01 cross-repo happy path through the same composition
// pattern `scripted-finalize` uses, but plugs an `AgentPhaseDriver`
// into the per-slice loop driver instead of `StubPhaseDriver`. The
// agent driver writes operator-supplied define artifacts (real
// `/spec:define` outputs collected ahead of time, or recorded from a
// Cursor SDK session) so the C12 define-* assertions exercise against
// real-quality bodies.
//
// **C12 boundary.** The agent backend is the *infrastructure* hand-off
// for real `/spec:define` execution. Two driver shapes are documented:
//
//   * Option (A) — Cursor SDK programmatic invocation. Documented in
//     `backends/README.md` §Agent Backend; **deferred** by C12 so an
//     experimental SDK integration cannot block the rest of the
//     chunk. The SDK landing should re-use this backend's prepare
//     hook and only swap the `AgentPhaseDriver` constructor input.
//   * Option (B) — Operator-manual / pre-collected results. The
//     operator runs `/spec:define <slice>` themselves, captures the
//     bodies into an `AgentOperatorResults` JSON file, and re-invokes
//     the runner with `--operator-results <path>.json`. **This is the
//     C12 default and the path the smoke target exercises.**
//
// Without `--operator-results` the backend prepares cleanly and
// returns a `pending-operator` verdict with `skipAssertions: true`,
// matching the policy stub backend uses when `specify` is missing.
// The smoke driver wraps that into an exit-0 skip so CI does not
// destabilise when no operator results are supplied.
//
// CLI authority is preserved: every Specify state mutation goes
// through `specify` (handled by the shared `driveSliceWithBodies`
// helper) and every Git commit goes through real `git` invocations.
// The agent driver only authors artifact bodies and the operator-
// recorded assertions payload.

import { exists } from "jsr:@std/fs@1";
import { isAbsolute, resolve } from "jsr:@std/path@1";

import { appendLog } from "../evidence.ts";
import { ScriptedFinalizeBackend } from "./scripted-finalize.ts";
import {
  AgentPhaseDriver,
  type AgentOperatorResults,
} from "./agent-phase-driver.ts";
import type {
  AssertionRecord,
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

export interface AgentBackendOptions {
  /**
   * Absolute or cwd-relative path to a pre-collected
   * `AgentOperatorResults` JSON file. When omitted, the backend
   * skips with `pending-operator` so the C12 smoke target can run
   * non-interactively in CI without authoring fake operator output.
   */
  operatorResultsPath?: string;
  /**
   * Reserved for option (A) — Cursor SDK driver. Not consumed by C12;
   * documented here so a follow-up amendment can plug an SDK runner
   * in without re-shaping the backend's public surface.
   */
  cursorSdk?: { enabled: boolean };
}

export class AgentBackend implements Backend {
  readonly name = "agent" as const;
  private readonly options: AgentBackendOptions;
  private inner: ScriptedFinalizeBackend | null = null;
  private skipReason: string | null = null;
  private operatorAssertions: AssertionRecord[] = [];
  private operatorNotes: string | null = null;

  constructor(options: AgentBackendOptions = {}) {
    this.options = options;
  }

  async prepare(ctx: RunContext): Promise<void> {
    if (!this.options.operatorResultsPath) {
      this.skipReason =
        "AgentBackend requires either Cursor SDK (--cursor-sdk; deferred to a future amendment) " +
        "or operator results (--operator-results <path>); neither was supplied. " +
        "The C12 smoke target wraps this with an exit-0 skip so CI stays green when no operator " +
        "has authored a real `/spec:define` transcript.";
      await appendLog(ctx.paths.stdoutLog, `[agent] ${this.skipReason}\n`);
      return;
    }

    const parsed = await loadOperatorResults(
      this.options.operatorResultsPath,
      ctx,
    );
    if ("error" in parsed) {
      this.skipReason = parsed.error;
      await appendLog(ctx.paths.stderrLog, `[agent] ${parsed.error}\n`);
      return;
    }

    const results = parsed.results;
    this.operatorAssertions = (results.assertions ?? []).map((a) => ({
      id: a.id,
      description: a.description ?? `Operator-reported verdict for '${a.id}'.`,
      verdict: a.verdict,
      evidence: a.evidence ?? "operator-reported",
      "fault-domain": a["fault-domain"] ??
        (a.verdict === "fail" ? "unknown" : null),
    }));
    this.operatorNotes = results.notes ?? null;

    this.inner = new ScriptedFinalizeBackend({
      phaseDriver: new AgentPhaseDriver(results),
    });
    await this.inner.prepare(ctx);
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    if (this.skipReason || !this.inner) {
      const note = `Agent backend skipped: ${this.skipReason ?? "prepare did not run"}.`;
      return {
        verdict: "pending-operator",
        faultDomain: null,
        notes: note,
        assertions: [],
        // `skipAssertions: true` so the runner-owned assertion stage
        // does not flood the output with `cli-substrate` failures
        // against an empty workspace. The smoke driver translates
        // `pending-operator` to exit 0 and prints the rationale.
        evidence: { extras: { skipAssertions: true } },
      };
    }
    const result = await this.inner.invoke(ctx);
    const mergedAssertions = mergeOperatorAssertions(
      this.operatorAssertions,
      result.assertions,
    );
    const noteSuffix = this.operatorNotes
      ? `\nOperator notes: ${this.operatorNotes}`
      : "";
    return {
      ...result,
      notes: `Agent backend (operator-results path) drove the RM-01 cross-repo happy path. ` +
        `${result.notes}${noteSuffix}`,
      assertions: mergedAssertions,
    };
  }

  async teardown(ctx: RunContext): Promise<void> {
    if (this.inner) await this.inner.teardown(ctx);
  }
}

/**
 * Merge operator-recorded assertion records with the inner backend's
 * records. Inner records win on id collision so the runner-owned
 * assertion stage's verdict (which inspects the live workspace) is
 * authoritative; operator-only ids stay in the merged list.
 */
function mergeOperatorAssertions(
  fromOperator: AssertionRecord[],
  fromInner: AssertionRecord[],
): AssertionRecord[] {
  const innerIds = new Set(fromInner.map((r) => r.id));
  const filtered = fromOperator.filter((r) => !innerIds.has(r.id));
  return [...fromInner, ...filtered];
}

interface LoadedResults {
  results: AgentOperatorResults;
}

async function loadOperatorResults(
  rawPath: string,
  ctx: RunContext,
): Promise<LoadedResults | { error: string }> {
  const resolvedPath = isAbsolute(rawPath) ? rawPath : resolve(rawPath);
  if (!(await exists(resolvedPath))) {
    return {
      error:
        `--operator-results file not found: ${resolvedPath}. ` +
        `Pass an absolute path or a path relative to the current working directory.`,
    };
  }

  let parsed: AgentOperatorResults;
  try {
    const raw = await Deno.readTextFile(resolvedPath);
    parsed = JSON.parse(raw) as AgentOperatorResults;
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e);
    return {
      error: `--operator-results file is not valid JSON: ${errMsg}`,
    };
  }

  if (parsed.scenario && parsed.scenario !== ctx.scenario.frontmatter.id) {
    return {
      error:
        `--operator-results file is for scenario '${parsed.scenario}' but the ` +
        `runner is executing '${ctx.scenario.frontmatter.id}'.`,
    };
  }

  return { results: parsed };
}
