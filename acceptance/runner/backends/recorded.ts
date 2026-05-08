// Recorded transcript backend (RM-01 plan, C15).
//
// Purpose: replay a previously trusted RM-01 plan-level / execute-level
// run cheaply, by re-executing the recorded `specify ...` argv list
// against a freshly bootstrapped hub and comparing exit codes against
// the trace's recorded values.
//
// Why this is cheaper than `scripted-execute` even though both shell
// out to `specify`: the recorded backend never re-derives the CLI
// sequence from a brief or a loop driver. The trace is the source of
// truth for argv ordering, so a regression in any of the scripted-*
// backends' loop logic does NOT cause this smoke to fail — only a
// regression in the underlying `specify` CLI substrate (an exit-code
// drift, a removed verb, a new flag refusal) does. That makes the
// smoke a tight `cli-substrate` regression gate.
//
// Architecture:
//
//   1. `prepare(ctx)` reads `--recorded-trace <path>.jsonl`, validates
//      every line as a `RecordedAction` (or one of the optional
//      header records), and runs `prepareScriptedHub(ctx)` from
//      `scripted-shared.ts` to land hub + bare remotes + fake `gh`.
//      The trace does NOT record hub creation; setup is deterministic
//      and skipped from replay.
//
//   2. `invoke(ctx)` walks the action list in order. For each entry
//      with a `command` (CLI argv): substitute the recorded `cwd`
//      placeholder with the new `setup.hubDir`, then run via
//      `runSpecify({...})`. Compare the live `exitCode` to the
//      recorded `exitCode`. On mismatch, fail with:
//        * `cli-substrate` when recorded was 0 and live is non-zero
//          (the CLI substrate regressed),
//        * `live-agent-nondeterminism` when recorded was non-zero and
//          live differs (typically a wrapper or env drift),
//        * `live-agent-nondeterminism` for any other delta.
//      Actions without a `command` (synthetic loop markers from
//      `scripted-execute-loop.jsonl`) are recorded with verdict
//      `synthetic-skipped` — they are book-keeping, not CLI work.
//
//   3. `teardown(ctx)` calls `collectEvidence` like the other scripted
//      backends so a maintainer reading a failed run dir gets the
//      same file set as for `scripted-plan`.
//
// **Boundary.** Recorded replay is *complementary* to live runs, not
// a replacement. The trace is only as trustworthy as the run that
// produced it; a corrupted recording will pass replay vacuously. The
// smoke target documents this in the run note. C15 explicitly defers
// the live-agent recording path (Cursor SDK / agent transcripts) to
// a future change; today's records come from the existing
// `scripted-plan` / `scripted-execute` / `scripted-finalize` /
// `stub` JSONL writers.

import { exists } from "jsr:@std/fs@1";
import { isAbsolute, join, resolve } from "jsr:@std/path@1";

import { collectEvidence } from "../evidence-collectors.ts";
import { appendLog } from "../evidence.ts";
import { runSpecify, SpecifyCommandError } from "../specify-cli.ts";
import type { SpecifyBin, SpecifyRun } from "../specify-cli.ts";
import {
  CHANGE_NAME,
  HUB_NAME,
  prepareScriptedHub,
  readIfExists,
  SLICE_BACKEND,
  SLICE_CONTRACT,
  SLICE_MOBILE,
  type ScriptedHubState,
} from "./scripted-shared.ts";
import type {
  Backend,
  BackendName,
  BackendResult,
  RecordedEvidenceRef,
  RunContext,
  SetupHubResult,
} from "../types.ts";

/**
 * Header record optionally written as the first JSONL line of a
 * recorded trace file. Carries provenance metadata so a reader can
 * tell which scenario / source backend / source run produced the
 * trace without re-deriving it from the file path. Optional —
 * traces without a header still parse, but the smoke target prints
 * a warning so an operator regenerating the trace can add one.
 */
export interface RecordedTraceHeader {
  kind: "recorded-trace-header";
  schemaVersion: 1;
  sourceBackend: BackendName;
  sourceRunId: string;
  sourceTimestamp: string;
  scenarioId: string;
}

/**
 * Optional trailing record declaring the structural final state the
 * replay must reach. The `recorded-trace-final-state-matches`
 * assertion handler reads this to pin a list of hub-relative paths
 * that must exist on disk after the replay finishes. Skip the record
 * to disable that assertion's structural check (it then only verifies
 * the action-level replay matched).
 */
export interface RecordedTraceFinalState {
  kind: "recorded-trace-final-state";
  /** Paths relative to `setup.hubDir` that must exist after replay. */
  expectedPaths: string[];
}

/**
 * Normalised replay action. Each line of a trace file is one of
 * these (or a header / final-state record). Existing JSONL writers
 * (`stub-actions.jsonl`, `scripted-plan-actions.jsonl`,
 * `scripted-execute-loop.jsonl`, `scripted-finalize-actions.jsonl`)
 * map onto this shape via the `kind` field; the recorded format is
 * the union of those four shapes plus a `synthetic` slot for
 * book-keeping records that are not CLI invocations.
 */
export interface RecordedAction {
  kind:
    | "stub-action"
    | "scripted-plan-action"
    | "scripted-execute-action"
    | "scripted-finalize-action"
    | "synthetic";
  ts?: string;
  /** Lifecycle phase (`setup` | `define` | `build` | `merge`); free-form for the synthetic kind. */
  phase?: string;
  /** Slice the action targeted, when applicable. */
  slice?: string;
  /** Human-readable label (e.g. `specify-slice-create`, `workspace-push`). */
  action?: string;
  /** Argv passed to `specify` when this action invoked the CLI. */
  command?: string[];
  /**
   * Working directory the action was recorded against. The literal
   * placeholder `<hubDir>` is substituted with the new `setup.hubDir`
   * during replay so the trace is portable across machines. Absent /
   * arbitrary cwds default to the new hub dir.
   */
  cwd?: string;
  /** Recorded exit code; replay fails when the live run does not match. */
  exitCode?: number;
  /** Workspace-relative paths the recorded run wrote. Informational. */
  artifacts?: string[];
  /** Backend-specific extensions tolerated by the parser. */
  extras?: Record<string, unknown>;
}

/** Per-replay outcome record persisted into evidence. */
export interface ReplayedActionRecord {
  step: number;
  recorded: RecordedAction;
  /**
   * `replayed` is `null` for synthetic / no-command records that
   * the backend deliberately did not re-execute.
   */
  replayed: ReplayedExecution | null;
  /**
   * Verdict: `pass` when recorded.exitCode === replayed.exitCode (or
   * the action was synthetic); `mismatch` when exit codes diverged;
   * `error` when the live run threw before producing an exit code.
   */
  outcome: "pass" | "mismatch" | "error" | "synthetic-skipped";
  /** Optional fault domain when `outcome !== "pass"`. */
  faultDomain?:
    | "cli-substrate"
    | "live-agent-nondeterminism"
    | "runner-setup"
    | "unknown";
  /** Free-form note for evidence (`mismatch: recorded=0 live=1`, etc.). */
  note?: string;
}

interface ReplayedExecution {
  args: string[];
  cwd: string;
  exitCode: number;
}

/** Evidence shape stored on `BackendResult.evidence.extras.recorded`. */
export interface RecordedEvidence {
  tracePath: string;
  schemaVersion: number;
  header: RecordedTraceHeader | null;
  finalState: RecordedTraceFinalState | null;
  actionCount: number;
  /** Number of actions with a `command` (CLI argv) the backend replayed. */
  replayedCommandCount: number;
  /** Number of synthetic / no-command records the backend skipped. */
  syntheticSkippedCount: number;
  /** First mismatch summary (or null when the replay was clean). */
  firstMismatch: string | null;
  /** Per-step outcomes (parallel JSONL written to `replayed-actions.jsonl`). */
  replayedActions: ReplayedActionRecord[];
}

/** Constructor options for `RecordedBackend`. */
export interface RecordedBackendOptions {
  /** Absolute or cwd-relative path to the recorded trace `.jsonl`. */
  tracePath?: string;
}

/** Result of the JSONL parse step. */
interface ParsedTrace {
  header: RecordedTraceHeader | null;
  actions: RecordedAction[];
  finalState: RecordedTraceFinalState | null;
  /** Parse errors (line + message) — empty on success. */
  errors: Array<{ line: number; message: string; raw: string }>;
}

const HUB_DIR_PLACEHOLDER = "<hubDir>";

export class RecordedBackend implements Backend {
  readonly name = "recorded" as const;

  private readonly options: RecordedBackendOptions;
  private state: {
    hub?: ScriptedHubState;
    parsed?: ParsedTrace;
    replayed: ReplayedActionRecord[];
    skipReason: string | null;
    workspaceStatusJson?: unknown;
  } = { replayed: [], skipReason: null };

  constructor(options: RecordedBackendOptions = {}) {
    this.options = options;
  }

  async prepare(ctx: RunContext): Promise<void> {
    if (!this.options.tracePath) {
      this.state.skipReason =
        "RecordedBackend requires --recorded-trace <path>.jsonl. " +
        "The C15 smoke target wraps a missing trace file with an exit-0 skip so CI " +
        "stays green when no recording is checked in. Pass an absolute path or a " +
        "path relative to the current working directory.";
      await appendLog(ctx.paths.stdoutLog, `[recorded] ${this.state.skipReason}\n`);
      return;
    }

    const tracePath = isAbsolute(this.options.tracePath)
      ? this.options.tracePath
      : resolve(this.options.tracePath);

    if (!(await exists(tracePath))) {
      this.state.skipReason =
        `--recorded-trace file not found: ${tracePath}. ` +
        `Generate one by running \`make acceptance-cross-repo-execute-smoke\` (or ` +
        `another scripted-* / agent smoke), copying the resulting JSONL writer files ` +
        `(scripted-plan-actions.jsonl + scripted-execute-loop.jsonl) into a single ` +
        `concatenated trace, and prefixing a recorded-trace-header line. See ` +
        `\`acceptance/runner/backends/README.md\` §Regenerating A Recorded Trace.`;
      await appendLog(ctx.paths.stdoutLog, `[recorded] ${this.state.skipReason}\n`);
      return;
    }

    const raw = await Deno.readTextFile(tracePath);
    const parsed = parseTrace(raw);
    if (parsed.errors.length > 0) {
      const summary = parsed.errors
        .slice(0, 5)
        .map((e) => `  line ${e.line}: ${e.message}`)
        .join("\n");
      throw new Error(
        `RecordedBackend: trace file ${tracePath} has ${parsed.errors.length} ` +
          `malformed record(s):\n${summary}` +
          (parsed.errors.length > 5 ? `\n  …(${parsed.errors.length - 5} more)` : ""),
      );
    }
    this.state.parsed = parsed;
    (this.state as { tracePath?: string }).tracePath = tracePath;

    const hub = await prepareScriptedHub(ctx);
    this.state.hub = hub;
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    if (this.state.skipReason || !this.state.hub || !this.state.parsed) {
      const note = `Recorded backend skipped: ${
        this.state.skipReason ?? "prepare did not run"
      }.`;
      return {
        verdict: "pending-operator",
        faultDomain: null,
        notes: note,
        assertions: [],
        evidence: { extras: { skipAssertions: true } },
      };
    }

    const { setup, bin } = this.state.hub;
    const { actions, header, finalState } = this.state.parsed;
    const tracePath = (this.state as { tracePath?: string }).tracePath ?? "";

    let firstMismatch: string | null = null;
    let firstFaultDomain: ReplayedActionRecord["faultDomain"] | null = null;
    let replayedCommands = 0;
    let synthetic = 0;

    for (let i = 0; i < actions.length; i++) {
      const action = actions[i];
      const step = i + 1;
      const cwd = resolveRecordedCwd(action.cwd, setup.hubDir);

      if (!action.command || action.command.length === 0) {
        synthetic += 1;
        this.state.replayed.push({
          step,
          recorded: action,
          replayed: null,
          outcome: "synthetic-skipped",
          note: synthethicNote(action),
        });
        continue;
      }

      replayedCommands += 1;
      let liveRun: SpecifyRun | null = null;
      let liveError: SpecifyCommandError | null = null;
      try {
        liveRun = await runSpecify({
          bin,
          cwd,
          args: stripGlobalFormatJson(action.command),
          env: setup.env,
        });
      } catch (e) {
        if (e instanceof SpecifyCommandError) {
          liveError = e;
          liveRun = e.run;
        } else {
          const msg = e instanceof Error ? e.message : String(e);
          this.state.replayed.push({
            step,
            recorded: action,
            replayed: null,
            outcome: "error",
            faultDomain: "runner-setup",
            note: `replay raised before producing an exit code: ${msg}`,
          });
          if (firstMismatch === null) {
            firstMismatch = `step ${step}: ${msg}`;
            firstFaultDomain = "runner-setup";
          }
          continue;
        }
      }

      const liveExit = liveRun!.exitCode;
      const recordedExit = typeof action.exitCode === "number"
        ? action.exitCode
        : 0;

      if (liveExit === recordedExit) {
        this.state.replayed.push({
          step,
          recorded: action,
          replayed: { args: liveRun!.args, cwd, exitCode: liveExit },
          outcome: "pass",
        });
        continue;
      }

      // Mismatch attribution:
      //   recorded 0, live non-zero  → cli-substrate (a regression that
      //     didn't exist when the trace was captured).
      //   recorded non-zero, live 0  → live-agent-nondeterminism (the
      //     CLI got more permissive; trace expected refusal).
      //   any other delta            → live-agent-nondeterminism.
      const fault: ReplayedActionRecord["faultDomain"] =
        recordedExit === 0 && liveExit !== 0
          ? "cli-substrate"
          : "live-agent-nondeterminism";
      const note =
        `mismatch: recorded exit ${recordedExit}, live exit ${liveExit} ` +
        `(\`specify ${action.command.join(" ")}\` in ${cwd})`;
      this.state.replayed.push({
        step,
        recorded: action,
        replayed: { args: liveRun!.args, cwd, exitCode: liveExit },
        outcome: "mismatch",
        faultDomain: fault,
        note,
      });
      if (firstMismatch === null) {
        firstMismatch = `step ${step}: ${note}`;
        firstFaultDomain = fault;
      }
      void liveError; // captured into liveRun above; keep eslint happy
    }

    // Best-effort: capture final workspace status JSON for evidence
    // parity with the scripted-* backends (no assertion gate).
    try {
      const { run } = await runSpecifyJsonNoThrow({
        bin,
        cwd: setup.hubDir,
        args: ["change", "plan", "status"],
        env: setup.env,
      });
      void run;
    } catch {
      /* non-fatal */
    }

    const evidence: RecordedEvidence = {
      tracePath,
      schemaVersion: header?.schemaVersion ?? 1,
      header,
      finalState,
      actionCount: actions.length,
      replayedCommandCount: replayedCommands,
      syntheticSkippedCount: synthetic,
      firstMismatch,
      replayedActions: this.state.replayed.slice(),
    };

    // Promote evidence onto RunContext so the runner-owned assertion
    // stage can read the replay outcomes without going through
    // `BackendResult.evidence` (which the AssertionContext does not
    // carry today). Mirrors how the C10/C11 backends stash
    // `executeState` / `finalizeState`.
    const evidenceRef: RecordedEvidenceRef = {
      tracePath: evidence.tracePath,
      schemaVersion: evidence.schemaVersion,
      header: evidence.header,
      finalState: evidence.finalState,
      actionCount: evidence.actionCount,
      replayedCommandCount: evidence.replayedCommandCount,
      syntheticSkippedCount: evidence.syntheticSkippedCount,
      firstMismatch: evidence.firstMismatch,
      replayedActions: evidence.replayedActions.map((r) => ({
        step: r.step,
        recorded: r.recorded as unknown as Record<string, unknown>,
        replayed: r.replayed,
        outcome: r.outcome,
        faultDomain: r.faultDomain,
        note: r.note,
      })),
    };
    (ctx as { recordedEvidence?: RecordedEvidenceRef }).recordedEvidence =
      evidenceRef;

    const passed = firstMismatch === null;
    return {
      verdict: passed ? "passed" : "failed",
      faultDomain: passed ? null : (firstFaultDomain ?? "unknown"),
      notes: passed
        ? `Recorded backend replayed ${replayedCommands} CLI argv(s) and ${synthetic} ` +
          `synthetic record(s) from ${tracePath} cleanly. ` +
          `recorded-trace-* assertions decide the final verdict.`
        : `Recorded backend replay diverged from the trace at ${firstMismatch}. ` +
          `See replayed-actions.jsonl for the full per-step delta.`,
      assertions: [],
      evidence: { extras: { recorded: evidence } },
    };
  }

  async teardown(ctx: RunContext): Promise<void> {
    if (!this.state.hub) return;
    const { setup } = this.state.hub;

    // Mirror C10/C11: prefer workspace clones for git-log capture
    // when they exist (the recorded backend may have driven workspace
    // sync), otherwise fall back to the source repos.
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
        planYamlBeforeFinalize: await readIfExists(
          join(setup.hubDir, "plan.yaml"),
        ),
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await appendLog(
        ctx.paths.stderrLog,
        `[recorded] evidence collection error: ${msg}\n`,
      );
    }
  }
}

// --- Free helpers ---------------------------------------------------

/**
 * Parse a recorded-trace JSONL string. The parser is permissive about
 * unknown fields (extras / future shapes) but strict about the
 * presence of a `kind` discriminator on every record. Returns parse
 * errors per-line so the prepare() error message can summarise them
 * without aborting on the first bad line.
 */
export function parseTrace(raw: string): ParsedTrace {
  const lines = raw.split("\n");
  const out: ParsedTrace = {
    header: null,
    actions: [],
    finalState: null,
    errors: [],
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.length === 0) continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch (e) {
      out.errors.push({
        line: i + 1,
        message: `not valid JSON (${e instanceof Error ? e.message : String(e)})`,
        raw: line,
      });
      continue;
    }
    if (!parsed || typeof parsed !== "object") {
      out.errors.push({ line: i + 1, message: "expected JSON object", raw: line });
      continue;
    }
    const kind = (parsed as { kind?: unknown }).kind;
    if (typeof kind !== "string") {
      out.errors.push({
        line: i + 1,
        message: "missing string `kind` field",
        raw: line,
      });
      continue;
    }

    if (kind === "recorded-trace-header") {
      const validation = validateHeader(parsed as Record<string, unknown>);
      if (validation.error) {
        out.errors.push({ line: i + 1, message: validation.error, raw: line });
        continue;
      }
      out.header = validation.header!;
      continue;
    }
    if (kind === "recorded-trace-final-state") {
      const validation = validateFinalState(parsed as Record<string, unknown>);
      if (validation.error) {
        out.errors.push({ line: i + 1, message: validation.error, raw: line });
        continue;
      }
      out.finalState = validation.finalState!;
      continue;
    }

    // Convert the four existing JSONL writer shapes into RecordedAction.
    // Pass-through `kind` literals mirror the writer file names.
    const action = coerceAction(kind, parsed as Record<string, unknown>);
    if ("error" in action) {
      out.errors.push({ line: i + 1, message: action.error, raw: line });
      continue;
    }
    out.actions.push(action.action);
  }

  return out;
}

function validateHeader(
  obj: Record<string, unknown>,
): { header?: RecordedTraceHeader; error?: string } {
  const schemaVersion = obj.schemaVersion;
  if (typeof schemaVersion !== "number" || schemaVersion !== 1) {
    return { error: `recorded-trace-header.schemaVersion must be 1; got ${String(schemaVersion)}` };
  }
  const sourceBackend = obj.sourceBackend;
  const sourceRunId = obj.sourceRunId;
  const sourceTimestamp = obj.sourceTimestamp;
  const scenarioId = obj.scenarioId;
  if (typeof sourceBackend !== "string") return { error: "header.sourceBackend missing" };
  if (typeof sourceRunId !== "string") return { error: "header.sourceRunId missing" };
  if (typeof sourceTimestamp !== "string") return { error: "header.sourceTimestamp missing" };
  if (typeof scenarioId !== "string") return { error: "header.scenarioId missing" };
  return {
    header: {
      kind: "recorded-trace-header",
      schemaVersion: 1,
      sourceBackend: sourceBackend as BackendName,
      sourceRunId,
      sourceTimestamp,
      scenarioId,
    },
  };
}

function validateFinalState(
  obj: Record<string, unknown>,
): { finalState?: RecordedTraceFinalState; error?: string } {
  const expectedPaths = obj.expectedPaths;
  if (!Array.isArray(expectedPaths)) {
    return { error: "recorded-trace-final-state.expectedPaths must be an array of strings" };
  }
  const cleaned: string[] = [];
  for (const p of expectedPaths) {
    if (typeof p !== "string" || p.length === 0) {
      return { error: "recorded-trace-final-state.expectedPaths entries must be non-empty strings" };
    }
    if (isAbsolute(p) || p.startsWith("..")) {
      return {
        error:
          `recorded-trace-final-state.expectedPaths entry '${p}' must be a hub-relative path`,
      };
    }
    cleaned.push(p);
  }
  return {
    finalState: { kind: "recorded-trace-final-state", expectedPaths: cleaned },
  };
}

function coerceAction(
  kind: string,
  obj: Record<string, unknown>,
): { action: RecordedAction } | { error: string } {
  // Recognise existing JSONL writer shapes plus the normalised kinds.
  switch (kind) {
    case "scripted-plan-actions-header":
    case "scripted-execute-loop-header":
    case "scripted-finalize-actions-header":
    case "stub-actions-header":
      // Origin-writer headers are book-keeping; we ignore them so a
      // trace produced by concatenating writer files Just Works.
      return {
        action: {
          kind: "synthetic",
          action: kind,
          extras: redactHubPaths(obj),
        },
      };
    case "stub-action":
    case "scripted-plan-action":
    case "scripted-execute-action":
    case "scripted-finalize-action":
    case "synthetic":
      return { action: normaliseAction(kind, obj) };
    default: {
      // Permissive: treat unknown writer shapes as scripted-plan-action
      // when they carry an `args` array (the load-bearing field for
      // replay), otherwise as a synthetic record.
      if (Array.isArray(obj.args)) {
        return {
          action: normaliseAction("scripted-plan-action", { ...obj, kind }),
        };
      }
      if (typeof obj.argv === "object" && Array.isArray(obj.argv)) {
        return {
          action: normaliseAction("scripted-finalize-action", { ...obj, kind }),
        };
      }
      return {
        action: { kind: "synthetic", action: kind, extras: redactHubPaths(obj) },
      };
    }
  }
}

function normaliseAction(
  kind: RecordedAction["kind"],
  obj: Record<string, unknown>,
): RecordedAction {
  const out: RecordedAction = { kind };
  if (typeof obj.ts === "string") out.ts = obj.ts;
  if (typeof obj.phase === "string") out.phase = obj.phase;
  if (typeof obj.slice === "string") out.slice = obj.slice;
  if (typeof obj.sliceName === "string" && !out.slice) out.slice = obj.sliceName;
  if (typeof obj.action === "string") out.action = obj.action;
  if (Array.isArray(obj.command)) {
    out.command = obj.command.filter((s): s is string => typeof s === "string");
  } else if (Array.isArray(obj.args)) {
    out.command = obj.args.filter((s): s is string => typeof s === "string");
  } else if (Array.isArray(obj.argv)) {
    out.command = obj.argv.filter((s): s is string => typeof s === "string");
  }
  if (typeof obj.cwd === "string") out.cwd = obj.cwd;
  if (typeof obj.exitCode === "number") out.exitCode = obj.exitCode;
  if (Array.isArray(obj.artifacts)) {
    out.artifacts = obj.artifacts.filter((s): s is string => typeof s === "string");
  }
  // Surface remaining fields under extras for forward-compatibility.
  const known = new Set([
    "kind",
    "ts",
    "phase",
    "slice",
    "sliceName",
    "action",
    "command",
    "args",
    "argv",
    "cwd",
    "exitCode",
    "artifacts",
    "step",
  ]);
  const extras: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (!known.has(k)) extras[k] = v;
  }
  if (Object.keys(extras).length > 0) out.extras = extras;
  return out;
}

function redactHubPaths(obj: Record<string, unknown>): Record<string, unknown> {
  // Copy through; we don't actually rewrite hub paths in extras —
  // they're informational. Future enrichment can hook here.
  return { ...obj };
}

function resolveRecordedCwd(
  recordedCwd: string | undefined,
  liveHubDir: string,
): string {
  if (!recordedCwd) return liveHubDir;
  if (recordedCwd === HUB_DIR_PLACEHOLDER) return liveHubDir;
  // For first-cut convenience: if the recorded cwd looks like an
  // absolute temp-dir-style path that ends in `<hubName>` (the
  // RM-01 hub), substitute the live hub dir. Otherwise honour the
  // recorded cwd verbatim — operator can override via the
  // `<hubDir>` placeholder for forward compatibility.
  if (recordedCwd.endsWith(`/${HUB_NAME}`)) return liveHubDir;
  return recordedCwd;
}

function stripGlobalFormatJson(args: string[]): string[] {
  // The plan-actions writer captures `--format json` as part of the
  // recorded argv. `runSpecify` forwards args verbatim, so pass them
  // through unchanged. (Older traces may contain `--format json` as
  // either a global prefix or a subcommand suffix; both are valid
  // CLI syntax. Kept as a single funnel for future normalisation.)
  return args;
}

function synthethicNote(action: RecordedAction): string {
  const slice = action.slice ?? action.action ?? "(unlabelled)";
  return `synthetic record (${action.kind}, ${slice}); no command to replay`;
}

/**
 * Best-effort `runSpecifyJson` that swallows non-zero exits. Used
 * for end-of-run probes that should never fail the replay.
 */
async function runSpecifyJsonNoThrow(opts: {
  bin: SpecifyBin;
  cwd: string;
  args: string[];
  env: SetupHubResult["env"];
}): Promise<{ run: SpecifyRun }> {
  try {
    const run = await runSpecify(opts);
    return { run };
  } catch (e) {
    if (e instanceof SpecifyCommandError) return { run: e.run };
    throw e;
  }
}

// --- Stable export of slice/change names for assertion handlers -----
//
// The assertion handlers may want to compare the trace's recorded
// slice names against the suite's canonical names; export them here
// so the handler module does not have to import from
// `scripted-shared.ts` directly.
export const RECORDED_SLICE_NAMES = {
  contract: SLICE_CONTRACT,
  backend: SLICE_BACKEND,
  mobile: SLICE_MOBILE,
  change: CHANGE_NAME,
} as const;
