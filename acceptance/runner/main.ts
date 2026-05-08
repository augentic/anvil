// Acceptance runner CLI entrypoint.
//
// Usage (Deno):
//   deno run --allow-read --allow-write --allow-env --allow-run \
//     acceptance/runner/main.ts [--list] [--suite <name>] \
//     [--scenario <id>] [--backend <name>] [--operator-results <path>] \
//     [--allow-backend-mismatch] [--preserve] [--help]
//
// The runner discovers scenarios from the four roots documented in
// `acceptance/README.md` §Scenario Discovery, prepares an isolated temp
// workspace plus an evidence run directory, invokes the requested
// backend, runs the runner-owned `assertions` stage between
// `backend.invoke()` and `backend.teardown()`, writes evidence, and
// applies the retention rules.

import { parseArgs } from "jsr:@std/cli@1/parse-args";
import { dirname, fromFileUrl, resolve } from "jsr:@std/path@1";

import { runAssertions } from "./assertions.ts";
import { bucketFor, discoverScenarios } from "./discovery.ts";
import {
  initEmptyEvidence,
  writeAssertions,
  writeFinalTree,
  writeScenarioCopy,
  writeSummary,
} from "./evidence.ts";
import {
  applyTeardown,
  createRunPaths,
  teardownDecision,
} from "./workspace.ts";
import { ManualBackend } from "./backends/manual.ts";
import { FixtureBackend } from "./backends/fixture.ts";
import { StubBackend } from "./backends/stub.ts";
import type { StubEvidence } from "./backends/stub.ts";
import { ScriptedPlanBackend } from "./backends/scripted-plan.ts";
import type { ScriptedPlanEvidence } from "./backends/scripted-plan.ts";
import { ScriptedExecuteBackend } from "./backends/scripted-execute.ts";
import type { ScriptedExecuteEvidence } from "./backends/scripted-execute.ts";
import { ScriptedFinalizeBackend } from "./backends/scripted-finalize.ts";
import type { ScriptedFinalizeEvidence } from "./backends/scripted-finalize.ts";
import { AgentBackend } from "./backends/agent.ts";
import { ContractsBuildBackend } from "./backends/contracts-build.ts";
import { OmniaBuildBackend } from "./backends/omnia-build.ts";
import { VectisBuildBackend } from "./backends/vectis-build.ts";
import { RecordedBackend } from "./backends/recorded.ts";
import type { RecordedEvidence } from "./backends/recorded.ts";
import type {
  Backend,
  BackendResult,
  FaultDomain,
  RunContext,
  Scenario,
} from "./types.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

const HELP = `acceptance runner — RM-01 framework (C04 + C05)

USAGE:
  deno run --allow-read --allow-write --allow-env --allow-run \\
    acceptance/runner/main.ts [OPTIONS]

OPTIONS:
  --list                          Discover scenarios and print them. No execution.
  --suite <name>                  Filter to scenarios under acceptance/suites/<name>/.
  --scenario <id>                 Filter to a single scenario id.
  --backend <name>                Backend to run (default: manual). Implemented:
                                    manual, fixture, stub, scripted-plan,
                                    scripted-execute, scripted-finalize, agent,
                                    contracts-build, omnia-build, vectis-build,
                                    recorded.
                                  scripted-plan: deterministic stand-in for
                                  the planner skill that drives a fixed
                                  sequence of CLI calls so the role-based
                                  RM-01 plan assertions exercise end-to-end.
                                  scripted-execute: composes scripted-plan
                                  setup with a deterministic loop driver
                                  (the moral equivalent of /change:execute
                                  loop) so the C10 execute-* assertions
                                  exercise end-to-end.
                                  scripted-finalize: composes scripted-execute
                                  with workspace push, fake-gh mark-merged,
                                  and change finalize so the C11 push-* /
                                  finalize-* assertions exercise end-to-end.
                                  None of the scripted backends prove the
                                  matching skill itself; that requires the
                                  reserved agent backend.
                                  recorded: replay a previously trusted run
                                  by reading a recorded JSONL trace file
                                  (--recorded-trace <path>) and re-executing
                                  every recorded \`specify\` argv against the
                                  live binary. Cheap regression coverage; not
                                  a replacement for periodic live runs.
  --recorded-trace <path>         Recorded backend. Path to a JSONL trace
                                  produced by concatenating the JSONL writer
                                  files (scripted-plan-actions.jsonl,
                                  scripted-execute-loop.jsonl, etc.) from a
                                  previously trusted run. See
                                  acceptance/runner/backends/README.md
                                  §Regenerating A Recorded Trace.
  --operator-results <path>       Manual or agent backend. JSON file with
                                  operator-reported outcomes (and, for the agent
                                  backend, per-slice define-stage bodies). See
                                  .cursor/schemas/operator-results.schema.json
                                  for the on-disk shape.
  --allow-backend-mismatch        Accept --backend <name> for a scenario whose
                                  frontmatter declares a different backend.
                                  Default: hard error.
  --preserve                      Keep the run directory and workspace regardless
                                  of outcome (otherwise: kept on failure only).
  --help, -h                      Show this help.

DISCOVERY ROOTS (see acceptance/README.md §Scenario Discovery):
  1. acceptance/suites/<suite>/scenario.md
  2. capabilities/<capability>/tests/<scenario>.md
  3. capabilities/<capability>/tests/<scenario>/scenario.md
  4. plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md

EVIDENCE:
  Each run writes summary.md, scenario.md, assertions.json, final-tree.txt,
  stdout.log, and stderr.log under
    \${TMPDIR}/specify-acceptance-runs/<bucket>/<scenario-id>/<run-id>/
  transcript.md and tool-calls.jsonl filenames are reserved for later
  backends; the skeleton manual and fixture backends do not write them.
`;

interface ParsedArgs {
  list: boolean;
  suite?: string;
  scenario?: string;
  backend: string;
  operatorResults?: string;
  recordedTrace?: string;
  allowBackendMismatch: boolean;
  preserve: boolean;
  help: boolean;
}

function parseCli(args: string[]): ParsedArgs {
  const flags = parseArgs(args, {
    boolean: ["list", "preserve", "help", "allow-backend-mismatch"],
    string: ["suite", "scenario", "backend", "operator-results", "recorded-trace"],
    alias: { h: "help" },
    default: {
      list: false,
      preserve: false,
      help: false,
      backend: "manual",
      "allow-backend-mismatch": false,
    },
    unknown: (arg: string) => {
      throw new Error(`Unknown argument: ${arg}`);
    },
  });
  return {
    list: Boolean(flags.list),
    suite: typeof flags.suite === "string" ? flags.suite : undefined,
    scenario: typeof flags.scenario === "string" ? flags.scenario : undefined,
    backend: String(flags.backend),
    operatorResults: typeof flags["operator-results"] === "string"
      ? (flags["operator-results"] as string)
      : undefined,
    recordedTrace: typeof flags["recorded-trace"] === "string"
      ? (flags["recorded-trace"] as string)
      : undefined,
    allowBackendMismatch: Boolean(flags["allow-backend-mismatch"]),
    preserve: Boolean(flags.preserve),
    help: Boolean(flags.help),
  };
}

function selectBackend(name: string, args: ParsedArgs): Backend {
  switch (name) {
    case "manual":
      return new ManualBackend({ operatorResultsPath: args.operatorResults });
    case "fixture":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the fixture backend ignores it.`,
        );
      }
      return new FixtureBackend();
    case "stub":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the stub backend ignores it.`,
        );
      }
      return new StubBackend();
    case "scripted-plan":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the scripted-plan backend ignores it.`,
        );
      }
      return new ScriptedPlanBackend();
    case "scripted-execute":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the scripted-execute backend ignores it.`,
        );
      }
      return new ScriptedExecuteBackend();
    case "scripted-finalize":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the scripted-finalize backend ignores it.`,
        );
      }
      return new ScriptedFinalizeBackend();
    case "contracts-build":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the contracts-build backend ignores it.`,
        );
      }
      return new ContractsBuildBackend();
    case "omnia-build":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the omnia-build backend ignores it.`,
        );
      }
      return new OmniaBuildBackend();
    case "vectis-build":
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the vectis-build backend ignores it.`,
        );
      }
      return new VectisBuildBackend();
    case "agent":
      // C12: --operator-results <path> is the operator-manual driver
      // input. When omitted the AgentBackend skips with
      // pending-operator so the smoke driver can exit-0 in CI.
      return new AgentBackend({
        operatorResultsPath: args.operatorResults,
      });
    case "recorded":
      // C15: --recorded-trace <path>.jsonl is the load-bearing
      // input. Without it the RecordedBackend skips with
      // pending-operator so the smoke driver can exit-0 in CI when
      // no trace has been checked in.
      if (args.operatorResults) {
        throw new Error(
          `--operator-results is only valid with --backend manual or --backend agent; the recorded backend ignores it.`,
        );
      }
      return new RecordedBackend({ tracePath: args.recordedTrace });
    default:
      throw new Error(`unknown backend: ${name}`);
  }
}

function applyFilters(scenarios: Scenario[], args: ParsedArgs): Scenario[] {
  return scenarios.filter((s) => {
    if (args.scenario && s.frontmatter.id !== args.scenario) return false;
    if (args.suite) {
      if (s.source.kind !== "suite" || s.source.suite !== args.suite) return false;
    }
    return true;
  });
}

async function listScenarios(args: ParsedArgs): Promise<number> {
  const all = await discoverScenarios(REPO_ROOT);
  const filtered = applyFilters(all, args);
  if (filtered.length === 0) {
    console.log("(no scenarios discovered for the given filters)");
    return 0;
  }
  for (const s of filtered) {
    const fm = s.frontmatter;
    console.log(
      `${fm.id}\t${fm.kind}\t${fm.backend}\t${bucketFor(s)}\t${s.relPath}`,
    );
  }
  return 0;
}

async function runScenario(scenario: Scenario, args: ParsedArgs): Promise<number> {
  const backend = selectBackend(args.backend, args);

  // Hard error on backend mismatch with frontmatter, unless the
  // operator opts out via --allow-backend-mismatch (used by
  // `make acceptance-smoke` to drive a manual scenario through the
  // fixture backend for non-interactive smoke coverage).
  if (backend.name !== scenario.frontmatter.backend) {
    const msg =
      `backend mismatch: scenario '${scenario.frontmatter.id}' declares ` +
      `backend='${scenario.frontmatter.backend}' but runner was invoked ` +
      `with --backend ${backend.name}.`;
    if (!args.allowBackendMismatch) {
      console.error(`error: ${msg}`);
      console.error(
        `       Re-invoke with --allow-backend-mismatch if this is intentional ` +
          `(e.g. running a manual scenario through the fixture backend for smoke coverage).`,
      );
      return 2;
    }
    console.warn(`warning: ${msg} Continuing because --allow-backend-mismatch was set.`);
  }

  const startedAt = new Date().toISOString();
  const paths = await createRunPaths(scenario);

  const ctx: RunContext = {
    scenario,
    paths,
    options: { preserve: args.preserve },
    startedAt,
  };

  let backendResult: BackendResult;
  let setupFailed = false;
  try {
    await initEmptyEvidence(paths);
    await writeScenarioCopy(paths, scenario);
    await backend.prepare(ctx);
    backendResult = await backend.invoke(ctx);
  } catch (e: unknown) {
    setupFailed = true;
    const msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    const faultDomain: FaultDomain = "runner-setup";
    backendResult = {
      verdict: "error",
      faultDomain,
      notes: `Runner setup failed before assertions could run: ${msg}`,
      assertions: [
        {
          id: "runner-setup",
          description: "Runner setup completed successfully.",
          verdict: "fail",
          evidence: msg,
          "fault-domain": faultDomain,
        },
      ],
    };
    try {
      await Deno.writeTextFile(paths.stderrLog, msg + "\n", { append: true });
    } catch {
      // best-effort; do not mask the original failure
    }
  }

  // Runner-owned `assertions` stage. Runs after `invoke` and before
  // `teardown`. Skipped on setup failure: there is no workspace state
  // worth probing, and the runner-setup fault-domain hint is already
  // set. Also skipped when a backend explicitly opts out via
  // `evidence.extras.skipAssertions === true` — used by the stub
  // backend on systems without `specify` so the smoke target does not
  // fail in CI when the dev tool is missing.
  let mergedAssertions = backendResult.assertions;
  let helperVerdictKind: string = "no-helpers";
  let firstFailureSummary: string | null = null;
  const backendSkippedAssertions = Boolean(
    (backendResult.evidence?.extras as
      | { skipAssertions?: boolean }
      | undefined)?.skipAssertions,
  );

  if (!setupFailed && !backendSkippedAssertions) {
    try {
      const stage = await runAssertions(ctx, backendResult);
      mergedAssertions = stage.records;
      helperVerdictKind = stage.helperVerdict.kind;
      if (stage.helperVerdict.kind === "failed") {
        firstFailureSummary =
          `${stage.helperVerdict.firstFailure.id}: ${stage.helperVerdict.firstFailure.evidence}`;
      }
    } catch (e) {
      const msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      mergedAssertions = [
        ...backendResult.assertions,
        {
          id: "assertions-stage",
          description: "Runner-owned assertions stage threw before completing.",
          verdict: "fail",
          evidence: msg,
          "fault-domain": "runner-setup",
        },
      ];
      helperVerdictKind = "failed";
      firstFailureSummary = `assertions-stage: ${msg}`;
    }
  }

  // Upgrade pending-operator -> passed/failed when helpers produced a
  // real verdict. The fixture backend already reports passed/failed,
  // so the upgrade only fires for the manual/fixture flows where the
  // backend deferred to the assertion stage.
  let finalVerdict = backendResult.verdict;
  let finalFault = backendResult.faultDomain;
  if (helperVerdictKind === "failed") {
    finalVerdict = "failed";
    if (!finalFault) finalFault = mergedAssertions.find((r) => r.verdict === "fail")?.["fault-domain"] ?? "unknown";
  } else if (helperVerdictKind === "passed" && finalVerdict === "pending-operator") {
    finalVerdict = "passed";
    finalFault = null;
  } else if (helperVerdictKind === "passed" && finalVerdict === "passed") {
    // already passed; nothing to upgrade.
  }

  const finalResult: BackendResult = {
    ...backendResult,
    verdict: finalVerdict,
    faultDomain: finalFault,
    assertions: mergedAssertions,
  };

  try {
    await backend.teardown(ctx);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    try {
      await Deno.writeTextFile(paths.stderrLog, `teardown error: ${msg}\n`, {
        append: true,
      });
    } catch { /* ignore */ }
  }

  await writeAssertions(paths, finalResult.assertions);
  await writeFinalTree(paths);
  await writeStubActions(paths.runDir, finalResult);
  await writeScriptedPlanActions(paths.runDir, finalResult);
  await writeScriptedExecuteActions(paths.runDir, finalResult);
  await writeScriptedFinalizeActions(paths.runDir, finalResult);
  await writeRecordedReplayedActions(paths.runDir, finalResult);
  await writeSummary(ctx, finalResult, new Date().toISOString());

  console.log("");
  console.log(`Run directory: ${paths.runDir}`);
  console.log(`Workspace:     ${paths.workspace}`);
  console.log(
    `Verdict:       ${finalResult.verdict}${finalResult.faultDomain ? ` (fault-domain=${finalResult.faultDomain})` : ""}`,
  );

  if (finalResult.verdict === "failed") {
    console.log("");
    console.log("Failures:");
    const failed = finalResult.assertions.filter((r) => r.verdict === "fail");
    if (failed.length === 0 && firstFailureSummary) {
      console.log(`  - ${firstFailureSummary}`);
    } else {
      for (const r of failed) {
        const fault = r["fault-domain"] ?? "unknown";
        console.log(`  - [${fault}] ${r.id}: ${r.evidence}`);
      }
    }
  }

  // Retention. `pending-operator` is treated as not-passed so the
  // evidence sticks around for the operator to read.
  const passed = finalResult.verdict === "passed";
  const decision = teardownDecision(passed, args.preserve);
  await applyTeardown(paths, decision);

  // Exit codes:
  //   0 — passed OR pending-operator (manual backend cannot decide pass/fail)
  //   1 — failed assertion verdict (backend or runner-owned helpers)
  //   2 — runner setup error / mismatched arguments
  if (setupFailed) return 2;
  if (finalResult.verdict === "failed") return 1;
  return 0;
}

async function main(): Promise<number> {
  let args: ParsedArgs;
  try {
    args = parseCli(Deno.args);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.error(msg);
    console.error("");
    console.error(HELP);
    return 2;
  }

  if (args.help) {
    console.log(HELP);
    return 0;
  }

  if (args.list) {
    return await listScenarios(args);
  }

  if (!args.scenario && !args.suite) {
    console.error(
      "error: must pass --scenario <id> or --suite <name> (or --list to discover)",
    );
    console.error("");
    console.error(HELP);
    return 2;
  }

  const all = await discoverScenarios(REPO_ROOT);
  const filtered = applyFilters(all, args);

  if (filtered.length === 0) {
    console.error("error: no scenarios matched the given filters");
    return 2;
  }
  if (filtered.length > 1) {
    console.error(
      `error: filters matched ${filtered.length} scenarios; narrow with --scenario <id>`,
    );
    for (const s of filtered) console.error(`  - ${s.frontmatter.id}\t${s.relPath}`);
    return 2;
  }

  return await runScenario(filtered[0], args);
}

/**
 * Persist the stub backend's per-action log into `stub-actions.jsonl`
 * next to `assertions.json`. The file is one JSON record per line so a
 * reader can `grep` / `jq` over it without parsing a tree. No-op when
 * the run did not use the stub backend.
 */
async function writeStubActions(
  runDir: string,
  result: BackendResult,
): Promise<void> {
  const stubbed = result.evidence?.extras?.stubbed as
    | StubEvidence
    | undefined;
  if (!stubbed) return;
  const path = `${runDir}/stub-actions.jsonl`;
  const header = {
    kind: "stub-actions-header",
    scenario: stubbed.scenario,
    slice: stubbed.slice,
    stubbedStages: stubbed.stubbedStages,
    skipped: stubbed.skipped ?? false,
    reason: stubbed.reason ?? null,
    actionCount: stubbed.actions.length,
  };
  const lines = [JSON.stringify(header)];
  for (const a of stubbed.actions) lines.push(JSON.stringify(a));
  await Deno.writeTextFile(path, lines.join("\n") + "\n");
}

/**
 * Persist the scripted-plan backend's per-action log into
 * `scripted-plan-actions.jsonl`. One JSON record per line so a reader
 * can `grep` / `jq` over the sequence of `specify change plan` calls.
 * No-op when the run did not use the scripted-plan backend.
 */
async function writeScriptedPlanActions(
  runDir: string,
  result: BackendResult,
): Promise<void> {
  const sp = result.evidence?.extras?.scriptedPlan as
    | ScriptedPlanEvidence
    | undefined;
  if (!sp) return;
  const path = `${runDir}/scripted-plan-actions.jsonl`;
  const header = {
    kind: "scripted-plan-actions-header",
    changeName: sp.changeName,
    hubName: sp.hubName,
    hubDir: sp.hubDir,
    slices: sp.slices,
    briefHubPath: sp.briefHubPath,
    briefSourcePath: sp.briefSourcePath,
    actionCount: sp.actions.length,
  };
  const lines = [JSON.stringify(header)];
  for (const a of sp.actions) lines.push(JSON.stringify(a));
  await Deno.writeTextFile(path, lines.join("\n") + "\n");
}

/**
 * Persist the scripted-execute backend's loop-driver step log into
 * `scripted-execute-loop.jsonl`. One JSON record per line so a reader
 * can correlate each plan-next iteration with the routed project +
 * stub action count it produced. No-op when the run did not use the
 * scripted-execute backend.
 */
async function writeScriptedExecuteActions(
  runDir: string,
  result: BackendResult,
): Promise<void> {
  const se = result.evidence?.extras?.scriptedExecute as
    | ScriptedExecuteEvidence
    | undefined;
  if (!se) return;
  const path = `${runDir}/scripted-execute-loop.jsonl`;
  const header = {
    kind: "scripted-execute-loop-header",
    changeName: se.changeName,
    hubName: se.hubName,
    hubDir: se.hubDir,
    slices: se.slices,
    briefHubPath: se.briefHubPath,
    briefSourcePath: se.briefSourcePath,
    finalNextReason: se.finalNextReason,
    loopStepCount: se.loopSteps.length,
  };
  const lines = [JSON.stringify(header)];
  for (const s of se.loopSteps) lines.push(JSON.stringify(s));
  await Deno.writeTextFile(path, lines.join("\n") + "\n");
}

/**
 * Persist the scripted-finalize backend's per-step log into
 * `scripted-finalize-actions.jsonl`. One JSON record per line so a
 * reader can correlate each push/mark-merged/finalize CLI step with
 * its captured exit code. No-op when the run did not use the
 * scripted-finalize backend.
 */
async function writeScriptedFinalizeActions(
  runDir: string,
  result: BackendResult,
): Promise<void> {
  const sf = result.evidence?.extras?.scriptedFinalize as
    | ScriptedFinalizeEvidence
    | undefined;
  if (!sf) return;
  const path = `${runDir}/scripted-finalize-actions.jsonl`;
  const header = {
    kind: "scripted-finalize-actions-header",
    changeName: sf.changeName,
    hubName: sf.hubName,
    hubDir: sf.hubDir,
    slices: sf.slices,
    briefHubPath: sf.briefHubPath,
    briefSourcePath: sf.briefSourcePath,
    prNumbers: sf.prNumbers,
    prRepoKeys: sf.prRepoKeys,
    preMergeProbeRan: sf.preMergeProbeRan,
    finalizeRefusedPreMerge: sf.finalizeRefusedPreMerge,
    pushOutputJsonPath: sf.pushOutputJsonPath,
    finalizeOutputJsonPath: sf.finalizeOutputJsonPath,
    finalizeSecondCallJsonPath: sf.finalizeSecondCallJsonPath,
    finalizePreMergeJsonPath: sf.finalizePreMergeJsonPath,
    stepCount: sf.finalizeSteps.length,
  };
  const lines = [JSON.stringify(header)];
  for (const s of sf.finalizeSteps) lines.push(JSON.stringify(s));
  await Deno.writeTextFile(path, lines.join("\n") + "\n");
}

/**
 * Persist the recorded backend's per-step replay log into
 * `replayed-actions.jsonl`. One JSON record per line: the recorded
 * action plus the live replay outcome. No-op when the run did not
 * use the recorded backend.
 */
async function writeRecordedReplayedActions(
  runDir: string,
  result: BackendResult,
): Promise<void> {
  const ev = result.evidence?.extras?.recorded as
    | RecordedEvidence
    | undefined;
  if (!ev) return;
  const path = `${runDir}/replayed-actions.jsonl`;
  const header = {
    kind: "replayed-actions-header",
    tracePath: ev.tracePath,
    schemaVersion: ev.schemaVersion,
    sourceHeader: ev.header,
    actionCount: ev.actionCount,
    replayedCommandCount: ev.replayedCommandCount,
    syntheticSkippedCount: ev.syntheticSkippedCount,
    firstMismatch: ev.firstMismatch,
    finalState: ev.finalState,
  };
  const lines = [JSON.stringify(header)];
  for (const r of ev.replayedActions) lines.push(JSON.stringify(r));
  await Deno.writeTextFile(path, lines.join("\n") + "\n");
}

if (import.meta.main) {
  Deno.exit(await main());
}
