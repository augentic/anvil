// Shared types for the acceptance runner skeleton (RM-01 plan, C04).
//
// The Backend interface in this file is load-bearing for later changes:
// C08 (deterministic stub) and C15 (recorded transcript) plug into it
// without changes to the runner core. Keep it minimal and well-documented.

import type { SetupHubResult } from "./hub.ts";
import type { SpecifyBin } from "./specify-cli.ts";
import type { GitEnv } from "./git.ts";

export type { GitEnv, SetupHubResult, SpecifyBin };

/** Frontmatter shape carried by every opted-in scenario file. */
export interface ScenarioFrontmatter {
  id: string;
  owner: string;
  kind: "capability" | "capability-boundary" | "suite" | "skill" | string;
  capability?: string;
  backend: BackendName;
  entrypoint: string;
  stages: ReadonlyArray<"define" | "build" | "merge" | "drop">;
  isolation: "fresh-project" | "shared-baseline" | "shared-slice" | string;
  "authorship-mode"?: string;
  assertions?: string[];
  "expected-artifacts"?: string[];
  "negative-expectations"?: string[];
  /**
   * C08 stub backend: subset of `stages` the deterministic stub drives
   * via `specify` CLI. Cross-field invariants (subset of `stages`,
   * only valid when `backend: stub`) are enforced statically in
   * `scripts/checks.ts`.
   */
  "stubbed-stages"?: ReadonlyArray<"define" | "build" | "merge">;
  /**
   * C08 stub backend: optional per-stage fixture directory (repo-relative)
   * the stub copies into the workspace at the start of the stubbed stage.
   * Keys are stage names; values resolve under the repo root.
   */
  "stub-fixtures"?: Partial<Record<"define" | "build" | "merge", string>>;
}

/** Where the scenario file lives, used to bucket run directories. */
export type ScenarioSource =
  | { kind: "suite"; suite: string }
  | { kind: "capability-flat"; capability: string }
  | { kind: "capability-dir"; capability: string; scenarioDir: string }
  | { kind: "skill-fixture"; plugin: string; skill: string; scenarioDir: string };

/** Body sections lifted from a scenario file. */
export interface ScenarioBody {
  title: string;
  intent: string;
  workspace: string;
  inputs: string;
  invocation: string;
  expectedArtifacts: string;
  assertions: string;
  negativeExpectations: string;
  cleanup: string;
  raw: string;
}

/** A discovered scenario, ready to be executed. */
export interface Scenario {
  frontmatter: ScenarioFrontmatter;
  body: ScenarioBody;
  /** Absolute path on disk. */
  filePath: string;
  /** Path relative to the repo root. */
  relPath: string;
  /** Discovery source bucket. */
  source: ScenarioSource;
}

export type BackendName =
  | "manual"
  | "stub"
  | "agent"
  | "recorded"
  | "fixture"
  | "scripted-plan"
  | "scripted-execute"
  | "scripted-finalize"
  | "contracts-build"
  | "omnia-build"
  | "vectis-build";

/**
 * Cross-repo execute state populated by the C10 `scripted-execute`
 * backend (and any future backend that drives `/change:execute loop`).
 * The execute-* assertion handlers gate themselves on this field —
 * when it is `undefined` (e.g. a plan-only run via `scripted-plan`)
 * they emit `skip` records with a clear "no execute backend ran"
 * rationale.
 */
export interface ExecuteState {
  /** Umbrella change name (`oauth-login` for the RM-01 fixture). */
  changeName: string;
  /** Hub directory the loop driver operated against. */
  hubDir: string;
  /**
   * Names of routed projects the loop driver visited. Each must have
   * a workspace clone at `<hubDir>/.specify/workspace/<name>/` after
   * `prepare-branch` ran. Order matches loop-iteration order.
   */
  routedProjects: string[];
  /** Branch every routed clone is expected to be on (`specify/<change>`). */
  branch: string;
  /**
   * C12 amendment: per-slice metadata the define-* assertion handlers
   * read so they do not have to import suite-specific constants. The
   * loop driver populates this in iteration order. Optional for
   * backwards compatibility with C10/C11 backends that did not set it
   * (the new handlers cleanly skip when empty).
   */
  slices?: SliceInfo[];
}

/** Per-slice metadata threaded onto `ExecuteState.slices`. */
export interface SliceInfo {
  /** Plan-entry name (e.g. `oauth-login-contract`). */
  name: string;
  /** Routed project, or `null` for projectless contract slices. */
  project: string | null;
  /** Capability brief routing the slice (`contracts`, `omnia`, `vectis`). */
  capability?: string;
}

/**
 * Cross-repo push/finalize state populated by the C11
 * `scripted-finalize` backend (and any future agent backend that
 * drives the landing path). The push-* and finalize-* assertion
 * handlers gate themselves on this field — when it is `undefined`
 * (e.g. an execute-only run via `scripted-execute`) they emit `skip`
 * records with a clear "no finalize backend ran" rationale.
 *
 * The shape is populated incrementally inside `invoke`:
 *   1. After `specify workspace push` runs: `pushOutputJson`,
 *      `pushOutput`, `prNumbers` are set.
 *   2. After the optional pre-merge negative probe runs:
 *      `finalizePreMergeJson`, `finalizePreMergeOutput`,
 *      `finalizeRefusedPreMerge` are set.
 *   3. After fake PRs are flipped to MERGED and `specify change
 *      finalize` runs: `finalizeOutputJson`, `finalizeOutput` are
 *      set.
 *   4. After the idempotency probe runs: `finalizeSecondCallJson`,
 *      `finalizeSecondOutput` are set.
 *
 * The push-* handlers gate on `pushOutput`; the finalize-* handlers
 * gate on `finalizeOutput`; the idempotency handler gates on
 * `finalizeSecondOutput`. Handlers downgrade to `skip` (not `fail`)
 * when their slot is missing — that way the same scenario file can
 * run under either `scripted-execute` (push/finalize ids skip) or
 * `scripted-finalize` (push/finalize ids assert) without re-shaping
 * its assertion list.
 */
export interface FinalizeState {
  /** Path to the captured `specify --format json workspace push` JSON. */
  pushOutputJson?: string;
  /** Parsed push JSON (for handlers that read structured fields). */
  pushOutput?: unknown;
  /**
   * Map of routed project name → PR number reported by `workspace
   * push` (e.g. `shop-backend → 41`, `shop-mobile → 18`). Empty
   * record when push has not yet run.
   */
  prNumbers: Record<string, number>;
  /**
   * Path to the captured first `specify --format json change finalize`
   * JSON output (success path).
   */
  finalizeOutputJson?: string;
  /** Parsed first-finalize JSON. */
  finalizeOutput?: unknown;
  /**
   * Path to the captured second-call `change finalize` JSON output
   * (idempotency probe — expected to exit non-zero with
   * `error: plan-not-found`).
   */
  finalizeSecondCallJson?: string;
  /** Parsed second-finalize JSON. */
  finalizeSecondOutput?: unknown;
  /**
   * Path to the captured pre-merge `change finalize` JSON output
   * from the negative-expectation probe (only set when the backend
   * exercised the `finalize-runs-before-prs-merged` path).
   */
  finalizePreMergeJson?: string;
  /** Parsed pre-merge finalize JSON. */
  finalizePreMergeOutput?: unknown;
  /**
   * Whether the CLI refused to finalize before PRs were merged.
   * `true` means non-zero exit (the expected behaviour); `false`
   * means the CLI accepted the call (a `cli-substrate` finding
   * the negative-test handler reports rather than failing the
   * suite).
   */
  finalizeRefusedPreMerge?: boolean;
}

/**
 * Fault-domain hint surfaced on failure. Taxonomy from
 * `acceptance/runner/README.md` §Failure Reporting. Kept as a string union
 * so later assertion modules cannot invent vocabulary.
 */
export type FaultDomain =
  | "cli-substrate"
  | "skill-orchestration"
  | "capability-brief"
  | "specialist-generation"
  | "runner-setup"
  | "external-fake-boundary"
  | "live-agent-nondeterminism"
  | "unknown";

/** A single assertion record written into `assertions.json`. */
export interface AssertionRecord {
  /** Stable id from the scenario, e.g. `files-exist`. */
  id: string;
  /** Short human-readable description. */
  description?: string;
  verdict: "pass" | "fail" | "skip";
  /** Pointer to the evidence that proves the verdict (path or short string). */
  evidence: string;
  /** Fault-domain attribution for failing assertions. `null` for pass/skip. */
  "fault-domain": FaultDomain | null;
}

/** Per-run filesystem layout the runner hands to backends and writers. */
export interface RunPaths {
  /** Run directory under the OS temp root holding evidence files. */
  runDir: string;
  /** Temp project root the backend operates in. Sibling of runDir. */
  workspace: string;
  /** Captured streams. */
  stdoutLog: string;
  stderrLog: string;
  /** Reserved evidence file names (skeleton does not write them). */
  transcriptMd: string;
  toolCallsJsonl: string;
  /** Always-written evidence files. */
  summaryMd: string;
  scenarioMd: string;
  assertionsJson: string;
  finalTreeTxt: string;
}

export interface RunOptions {
  /** Operator opted into preserving the run directory regardless of outcome. */
  preserve: boolean;
}

/**
 * Context passed to every backend method. Intentionally narrow: backends
 * see only the scenario, the run paths, and their options. They never see
 * the discovery list or runner internals.
 *
 * The cross-repo fields (`setup`, `specifyBin`) are populated by backends
 * that build a `setupHub` result during `prepare` and stash it back on
 * the context so the runner-owned `assertions` stage can read hub state
 * (`ctx.setup.hubDir`, `ctx.setup.env`) without re-deriving it. C09
 * (RM-01 plan-level outside-in) is the first consumer; cross-repo
 * scenarios that do not need a hub leave both fields `undefined`.
 *
 * Forward-declared types here to keep the import graph minimal — the
 * actual `SetupHubResult` and `SpecifyBin` shapes live in
 * `acceptance/runner/hub.ts` and `acceptance/runner/specify-cli.ts`.
 */
export interface RunContext {
  scenario: Scenario;
  paths: RunPaths;
  options: RunOptions;
  /** Wall-clock start of this run, in ISO 8601. */
  startedAt: string;
  /**
   * Cross-repo hub setup produced by the backend's `prepare` step. The
   * runner stashes this here so plan-/registry-level assertion handlers
   * can read `setup.hubDir`, `setup.env`, `setup.projectDirs`, etc.
   * without re-running `setupHub`. Optional because most scenarios are
   * single-repo and have no hub.
   */
  setup?: SetupHubResult;
  /**
   * Resolved `specify` binary the backend used. Threaded onto the
   * context so handlers (e.g. `specify change plan validate`) can drive
   * the same binary that produced the workspace state. Optional for the
   * same reason as `setup` — single-repo scenarios may not call the
   * CLI at all.
   */
  specifyBin?: SpecifyBin;
  /**
   * C10 cross-repo execute state. Populated by the `scripted-execute`
   * backend (and any future agent backend that drives
   * `/change:execute loop`). The execute-* assertion handlers read
   * this to decide between running their checks and emitting a skip
   * record. `undefined` for plan-only runs (`scripted-plan`).
   */
  executeState?: ExecuteState;
  /**
   * C11 cross-repo push/finalize state. Populated by the
   * `scripted-finalize` backend (and any future agent backend that
   * drives the landing path). The push-* and finalize-* assertion
   * handlers read this to decide between running their checks and
   * emitting a skip record. `undefined` for execute-only runs
   * (`scripted-execute`).
   */
  finalizeState?: FinalizeState;
  /**
   * C15 recorded backend evidence. Populated by the `RecordedBackend`
   * during `invoke`; the `recorded-trace-*` assertion handlers read
   * this to score the replay outcome. `undefined` for runs not
   * driven by the recorded backend, in which case the handlers
   * cleanly self-skip.
   */
  recordedEvidence?: RecordedEvidenceRef;
}

/**
 * Forward-declared shape the recorded backend stashes on
 * `RunContext`. Mirrors `RecordedEvidence` in
 * `acceptance/runner/backends/recorded.ts`; kept structural here so
 * the runner core does not have to import the backend module.
 */
export interface RecordedEvidenceRef {
  tracePath: string;
  schemaVersion: number;
  header: unknown;
  finalState:
    | { kind: "recorded-trace-final-state"; expectedPaths: string[] }
    | null;
  actionCount: number;
  replayedCommandCount: number;
  syntheticSkippedCount: number;
  firstMismatch: string | null;
  replayedActions: Array<{
    step: number;
    recorded: Record<string, unknown>;
    replayed: { args: string[]; cwd: string; exitCode: number } | null;
    outcome: "pass" | "mismatch" | "error" | "synthetic-skipped";
    faultDomain?:
      | "cli-substrate"
      | "live-agent-nondeterminism"
      | "runner-setup"
      | "unknown";
    note?: string;
  }>;
}


/**
 * The verdict a backend returns. The skeleton's manual backend reports
 * `pending-operator`; later backends will report `passed`/`failed`.
 *
 * `error` is reserved for runner-side setup failures and produces a
 * fault-domain hint of `runner-setup` (or `unknown`) by default.
 */
export type RunVerdict = "passed" | "failed" | "pending-operator" | "error";

export interface BackendResult {
  verdict: RunVerdict;
  /** Fault-domain hint when the verdict is not `passed`. */
  faultDomain: FaultDomain | null;
  /** Free-form notes the backend wants surfaced in `summary.md`. */
  notes: string;
  /** Structured per-assertion records to merge into `assertions.json`. */
  assertions: AssertionRecord[];
  /**
   * Optional handoff payload for the runner-owned `assertions` stage.
   * The dispatcher consumes this when handlers need backend-collected
   * evidence (e.g. captured verifier stdout, the list of paths the
   * `fixture` backend materialised). The shape is intentionally
   * free-form so backends can extend without modifying the runner.
   */
  evidence?: BackendEvidence;
}

/**
 * Optional evidence a backend can hand to the assertion stage. All
 * fields are optional so backends that have nothing extra to share can
 * omit the property entirely. The runner copies non-empty fields into
 * the run summary under "Backend Evidence".
 */
export interface BackendEvidence {
  /** Captured stdout from a verifier or CLI invocation. */
  verifierStdout?: string;
  /** Captured stderr from a verifier or CLI invocation. */
  verifierStderr?: string;
  /** Verifier exit code, when known. */
  verifierExitCode?: number;
  /** Paths the backend materialised, relative to the workspace. */
  materialisedPaths?: string[];
  /** Free-form structured payload for backend-specific notes. */
  extras?: Record<string, unknown>;
}

/**
 * Backend contract. Designed so C08 (deterministic stub), the eventual
 * agent runtime backend, and C15 (recorded transcript) plug in without
 * changes to the runner core.
 *
 * Lifecycle:
 *   1. `prepare(ctx)` — seed scenario inputs, configure fakes, etc.
 *      May read scenario body sections (Inputs, Workspace) and write into
 *      `ctx.paths.workspace`. MUST NOT hand-edit `.specify/` lifecycle
 *      state; lifecycle transitions go through the `specify` CLI.
 *   2. `invoke(ctx)` — run the scenario's invocation against the prepared
 *      workspace. Returns a `BackendResult` with assertion records the
 *      runner merges into `assertions.json`. The result may carry an
 *      optional `evidence` payload the runner-owned `assertions` stage
 *      forwards to handler functions.
 *   3. (runner-owned) `assertions` stage — between `invoke` and
 *      `teardown` the runner dispatches assertion ids declared in the
 *      scenario frontmatter to handler functions registered in
 *      `acceptance/runner/assertions.ts`. Handlers consume the
 *      workspace, the scenario frontmatter, and `BackendResult.evidence`
 *      and append `AssertionRecord`s. This stage is never on the
 *      `Backend` interface itself — backends must not run their own
 *      assertions ad hoc.
 *   4. `teardown(ctx)` — release any external resources (kill child
 *      processes, close fake `gh` sockets). Filesystem cleanup of the
 *      workspace and run directory is owned by the runner, not the
 *      backend, so retention rules stay centralised.
 */
export interface Backend {
  /** Backend taxonomy name. Must match the scenario's `backend:` field. */
  readonly name: BackendName;

  prepare(ctx: RunContext): Promise<void>;
  invoke(ctx: RunContext): Promise<BackendResult>;
  teardown(ctx: RunContext): Promise<void>;
}
