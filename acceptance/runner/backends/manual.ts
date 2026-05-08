// Manual backend: documentation-only path that records the scenario,
// prints the next operator action, and (optionally) consumes a
// pre-collected operator-results JSON file so non-interactive runs can
// produce a real verdict.
//
// The two paths in detail:
//
//   1. Default (no `--operator-results`): the backend prints the
//      operator briefing, marks every declared assertion as `skip`, and
//      returns the `pending-operator` verdict introduced by C04. This
//      is the long-standing manual contract — a human runs the prompts
//      and fills in the run summary.
//
//   2. With `--operator-results <path>`: the backend reads the JSON
//      results an operator pre-collected for a non-interactive run
//      (handy for CI, dry runs, or when an upstream agent has already
//      executed the entrypoint). The runner-owned `assertions` stage
//      still decides the final verdict on the workspace; the
//      operator-results file is the bridge that reports what the
//      operator already ran. See the `OperatorResults` type below for
//      the on-disk shape.
//
// This backend never mutates `.specify/` lifecycle state.

import { exists } from "jsr:@std/fs@1";
import { isAbsolute, resolve } from "jsr:@std/path@1";

import { appendLog } from "../evidence.ts";
import type {
  AssertionRecord,
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

/**
 * On-disk shape of the file `--operator-results <path>` consumes.
 * Operators (or agents) write this file after running the scenario's
 * Invocation block to capture which next-step they completed and what
 * they observed.
 *
 * Example:
 *
 * ```json
 * {
 *   "scenario": "contracts-describe",
 *   "completed": true,
 *   "notes": "Ran /spec:define ... in workspace foo.",
 *   "verifierStdout": "{\"status\":\"clean\"}\n",
 *   "assertions": [
 *     { "id": "files-exist", "verdict": "pass", "evidence": "see workspace" }
 *   ]
 * }
 * ```
 *
 * `assertions` is optional. When absent, the runner-owned `assertions`
 * stage still runs against the on-disk workspace, so an operator can
 * record completion without pre-judging each assertion.
 */
export interface OperatorResults {
  scenario?: string;
  completed?: boolean;
  notes?: string;
  /** Raw verifier stdout, forwarded as `BackendEvidence.verifierStdout`. */
  verifierStdout?: string;
  assertions?: Array<{
    id: string;
    verdict: "pass" | "fail" | "skip";
    evidence?: string;
    description?: string;
    "fault-domain"?: AssertionRecord["fault-domain"];
  }>;
}

export interface ManualBackendOptions {
  /** Absolute or workspace-relative path to a pre-collected results file. */
  operatorResultsPath?: string;
}

export class ManualBackend implements Backend {
  readonly name = "manual" as const;
  private readonly options: ManualBackendOptions;

  constructor(options: ManualBackendOptions = {}) {
    this.options = options;
  }

  async prepare(_ctx: RunContext): Promise<void> {
    // Nothing to seed for the documentation-only path.
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    const { scenario, paths } = ctx;
    const lines = renderOperatorBriefing(ctx);
    const text = lines.join("\n") + "\n";

    console.log(text);
    await appendLog(paths.stdoutLog, text);

    if (this.options.operatorResultsPath) {
      return await this.invokeWithResults(ctx, this.options.operatorResultsPath);
    }

    const assertions: AssertionRecord[] = (scenario.frontmatter.assertions ?? []).map((id) => ({
      id,
      description: `Declared assertion '${id}' — pending operator confirmation.`,
      verdict: "skip",
      evidence: "manual backend: no --operator-results supplied",
      "fault-domain": null,
    }));

    return {
      verdict: "pending-operator",
      faultDomain: null,
      notes:
        "Manual backend recorded the run. No automated assertions executed. " +
        "Operator follows the printed Invocation block and the scenario's " +
        "Assertions / Negative Expectations sections, then fills in the run summary. " +
        "Pass --operator-results <path> to record pre-collected outcomes for a non-interactive run.",
      assertions,
    };
  }

  private async invokeWithResults(
    ctx: RunContext,
    rawPath: string,
  ): Promise<BackendResult> {
    const resolvedPath = isAbsolute(rawPath) ? rawPath : resolve(rawPath);
    if (!(await exists(resolvedPath))) {
      const msg =
        `--operator-results file not found: ${resolvedPath}. ` +
        `Pass an absolute path or a path relative to the current working directory.`;
      await appendLog(ctx.paths.stderrLog, msg + "\n");
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: msg,
        assertions: [],
      };
    }

    let parsed: OperatorResults;
    try {
      const raw = await Deno.readTextFile(resolvedPath);
      parsed = JSON.parse(raw) as OperatorResults;
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      const msg = `--operator-results file is not valid JSON: ${errMsg}`;
      await appendLog(ctx.paths.stderrLog, msg + "\n");
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: msg,
        assertions: [],
      };
    }

    if (parsed.scenario && parsed.scenario !== ctx.scenario.frontmatter.id) {
      const msg =
        `--operator-results file is for scenario '${parsed.scenario}' but the ` +
        `runner is executing '${ctx.scenario.frontmatter.id}'.`;
      await appendLog(ctx.paths.stderrLog, msg + "\n");
      return {
        verdict: "failed",
        faultDomain: "runner-setup",
        notes: msg,
        assertions: [],
      };
    }

    const reported: AssertionRecord[] = (parsed.assertions ?? []).map((a) => ({
      id: a.id,
      description: a.description ??
        `Operator-reported verdict for '${a.id}'.`,
      verdict: a.verdict,
      evidence: a.evidence ?? "operator-reported",
      "fault-domain": a["fault-domain"] ?? (a.verdict === "fail" ? "unknown" : null),
    }));

    const completed = parsed.completed !== false;
    const notes = parsed.notes
      ? `Operator-results loaded from ${resolvedPath}: ${parsed.notes}`
      : `Operator-results loaded from ${resolvedPath}.`;

    // The manual backend itself only reports what the operator
    // submitted. The runner-owned `assertions` stage still examines
    // the workspace and merges its own records, which take precedence
    // over operator-reported records when ids collide.
    return {
      verdict: completed ? "passed" : "pending-operator",
      faultDomain: null,
      notes,
      assertions: reported,
      evidence: {
        verifierStdout: parsed.verifierStdout,
      },
    };
  }

  async teardown(_ctx: RunContext): Promise<void> {
    // No external resources. Workspace and run-directory cleanup are
    // owned by the runner so retention rules stay centralised.
  }
}

function renderOperatorBriefing(ctx: RunContext): string[] {
  const { scenario, paths } = ctx;
  const fm = scenario.frontmatter;
  const lines: string[] = [];

  lines.push(`=== Manual Acceptance Run: ${fm.id} ===`);
  lines.push("");
  lines.push(`Scenario file: ${scenario.relPath}`);
  lines.push(`Capability:    ${fm.capability ?? "n/a"}`);
  lines.push(`Backend:       ${fm.backend}`);
  lines.push(`Entrypoint:    ${fm.entrypoint}`);
  lines.push(`Stages:        ${fm.stages.join(", ")}`);
  lines.push(`Isolation:     ${fm.isolation}`);
  lines.push(`Workspace:     ${paths.workspace}`);
  lines.push(`Run directory: ${paths.runDir}`);
  lines.push("");

  if (scenario.body.intent) {
    lines.push("--- Intent ---");
    lines.push(scenario.body.intent);
    lines.push("");
  }

  if (scenario.body.invocation) {
    lines.push("--- Invocation ---");
    lines.push(scenario.body.invocation);
    lines.push("");
  }

  lines.push("--- Next Operator Actions ---");
  let step = 1;
  lines.push(
    `${step++}. cd into the workspace at ${paths.workspace} (or your project of choice if the scenario is not 'fresh-project').`,
  );
  if (scenario.body.inputs.trim()) {
    lines.push(
      `${step++}. Materialise any source files described under '## Inputs' in ${scenario.relPath} before invoking the entrypoint.`,
    );
  }
  lines.push(
    `${step++}. Run the entrypoint and any follow-up commands from the '## Invocation' block above.`,
  );
  lines.push(
    `${step++}. Record results against the scenario's '## Assertions' and '## Negative Expectations' sections.`,
  );
  lines.push(
    `${step++}. Apply '## Cleanup' from the scenario file. The runner only cleans the temp workspace; lifecycle state must be cleaned through the 'specify' CLI.`,
  );
  lines.push(
    `${step++}. Fill in the run summary at ${paths.runDir}/summary.md (or paste it into the operator notes).`,
  );
  lines.push(
    `${step++}. (optional, non-interactive) Capture results in JSON and re-invoke the runner with '--operator-results <path>' to upgrade the verdict from 'pending-operator' to a real pass/fail.`,
  );

  return lines;
}
