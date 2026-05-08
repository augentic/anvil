// Temp workspace + run-directory creation and teardown.
//
// `<TMPDIR>/specify-acceptance-runs/<bucket>/<scenario-id>/<run-id>/` holds
// the run's evidence files. The workspace (the temp project root) is a
// separate temp directory created via `Deno.makeTempDir` so it can be
// removed independently when the run passes.
//
// Retention rules (per `acceptance/README.md` §Run Evidence Policy):
//   - pass: discard workspace AND run dir unless `--preserve`,
//   - fail: keep both,
//   - `--preserve`: keep both regardless of outcome.

import { join } from "jsr:@std/path@1";

import { bucketFor } from "./discovery.ts";
import type { RunPaths, Scenario } from "./types.ts";

const TEMP_RUN_PREFIX = "specify-acceptance-runs";
const WORKSPACE_PREFIX = "specify-acceptance-";

/** Create a fresh run directory and an isolated workspace for a scenario. */
export async function createRunPaths(scenario: Scenario): Promise<RunPaths> {
  const runRoot = runRootFor(scenario);
  await Deno.mkdir(runRoot, { recursive: true });

  const workspace = await Deno.makeTempDir({ prefix: WORKSPACE_PREFIX });

  return {
    runDir: runRoot,
    workspace,
    stdoutLog: join(runRoot, "stdout.log"),
    stderrLog: join(runRoot, "stderr.log"),
    transcriptMd: join(runRoot, "transcript.md"),
    toolCallsJsonl: join(runRoot, "tool-calls.jsonl"),
    summaryMd: join(runRoot, "summary.md"),
    scenarioMd: join(runRoot, "scenario.md"),
    assertionsJson: join(runRoot, "assertions.json"),
    finalTreeTxt: join(runRoot, "final-tree.txt"),
  };
}

function runRootFor(scenario: Scenario): string {
  const tmp = Deno.env.get("TMPDIR") ?? "/tmp";
  const bucket = bucketFor(scenario);
  return join(
    tmp,
    TEMP_RUN_PREFIX,
    bucket,
    scenario.frontmatter.id,
    runId(),
  );
}

function runId(): string {
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const rand = Math.random().toString(36).slice(2, 8);
  return `${ts}-${rand}`;
}

export interface TeardownDecision {
  removeWorkspace: boolean;
  removeRunDir: boolean;
}

/** Apply the retention rules above. */
export function teardownDecision(
  passed: boolean,
  preserve: boolean,
): TeardownDecision {
  if (preserve) return { removeWorkspace: false, removeRunDir: false };
  if (passed) return { removeWorkspace: true, removeRunDir: true };
  return { removeWorkspace: false, removeRunDir: false };
}

export async function applyTeardown(
  paths: RunPaths,
  decision: TeardownDecision,
): Promise<void> {
  if (decision.removeWorkspace) await removeIfExists(paths.workspace);
  if (decision.removeRunDir) await removeIfExists(paths.runDir);
}

async function removeIfExists(path: string): Promise<void> {
  try {
    await Deno.remove(path, { recursive: true });
  } catch (e) {
    if (!(e instanceof Deno.errors.NotFound)) throw e;
  }
}
