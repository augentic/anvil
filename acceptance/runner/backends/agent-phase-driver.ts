// Agent phase driver (RM-01 plan, C12).
//
// Plugs into the same `PhaseDriver` interface the C10 / C11 stub
// driver uses, but produces define-stage artifact bodies from operator-
// supplied `--operator-results <path>.json` rather than fixed
// `STUB:`-marker text. The shared lifecycle code in `phase-driver.ts`
// (`driveSliceWithBodies`) handles every CLI transition, the baseline
// merge commit, the residue commit, and the per-slice action log; the
// agent driver is responsible for *only* the artifact bodies.
//
// Two driver shapes ship today (option B is the reliable default):
//
//   * **Operator-manual / pre-collected results.** The operator runs
//     `/spec:define <slice>` themselves (or replays a recorded JSON
//     transcript) and hands the runner an `OperatorResults` file via
//     `--operator-results <path>.json`. The driver reads the file once
//     in `prepare`, looks up bodies per slice, and writes them through
//     the shared lifecycle. Missing bodies for a slice fall back to
//     stub bodies so the smoke target still produces a clean evidence
//     surface for the assertion plumbing.
//
//   * **Cursor SDK (option A) — DEFERRED.** Programmatically invoking
//     `/spec:define <slice>` per slice via `@cursor/sdk` is the
//     intended future path. C12 documents the integration shape in
//     `backends/README.md` §Agent Backend but does not ship the SDK
//     wiring — the operator-manual path is the load-bearing fallback,
//     and the SDK driver should land as a follow-up amendment without
//     re-shaping this driver's public surface.
//
// The driver is CLI-authoritative just like the stub: only artifact
// bodies are written here. Every plan-entry transition goes through
// `specify change plan transition`; baseline + residue commits go
// through `git`. The driver never hand-edits `.specify/` lifecycle
// metadata.

import {
  capabilityRequiresDesign,
  driveSliceWithBodies,
  stubBodyFactory,
} from "./phase-driver.ts";
import type {
  DefineBodies,
  DriveSliceOpts,
  DriveSliceResult,
  PhaseDriver,
} from "./phase-driver.ts";

/**
 * Per-slice define artifact bodies the operator (or upstream agent
 * recording) supplies. Every field is optional: a `null`/missing value
 * falls back to the stub body so the assertion plumbing stays exercised
 * even when the operator only authored a subset of artifacts.
 */
export interface OperatorSliceBodies {
  /** `proposal.md` body. */
  proposal?: string;
  /** `spec.md` body. */
  spec?: string;
  /** `tasks.md` body. */
  tasks?: string;
  /**
   * `design.md` body. Set to `null` to explicitly opt out (e.g. for
   * the `contracts` capability whose brief has no design step).
   * Omit the field to fall back to the capability-driven default.
   */
  design?: string | null;
  /**
   * Residue file body for routed slices. Falls back to a `STUB:`
   * marker when omitted; the C10 `residue-commit-non-empty` assertion
   * accepts any non-empty body.
   */
  residue?: string;
}

/**
 * On-disk shape of the JSON file `--operator-results <path>` consumes
 * for the agent backend. The file is the bridge between a real
 * operator-driven `/spec:define` session (or a recorded SDK
 * transcript) and the runner's deterministic per-slice loop.
 *
 * The shape intentionally mirrors `OperatorResults` in `manual.ts`
 * (`scenario`, `completed`, `notes`) and adds a `slices:` map keyed by
 * slice name. `assertions:` is forwarded into the runner's
 * `BackendResult.assertions` payload — operators can pre-record a
 * verdict for any assertion id; the runner-owned assertion stage
 * still re-runs every handler against the on-disk workspace and its
 * record wins on collision.
 *
 * Schema lives at `.cursor/schemas/operator-results.schema.json`.
 */
export interface AgentOperatorResults {
  /** Optional scenario id; checked against `ctx.scenario.frontmatter.id` when present. */
  scenario?: string;
  /** Whether the operator considers the run complete. */
  completed?: boolean;
  /** Free-form operator notes surfaced in `summary.md`. */
  notes?: string;
  /** Per-slice bodies. Keys are slice names (`oauth-login-contract`, …). */
  slices?: Record<string, OperatorSliceBodies>;
  /** Optional pre-recorded assertion verdicts. */
  assertions?: Array<{
    id: string;
    verdict: "pass" | "fail" | "skip";
    evidence?: string;
    description?: string;
    "fault-domain"?:
      | "cli-substrate"
      | "skill-orchestration"
      | "capability-brief"
      | "specialist-generation"
      | "runner-setup"
      | "external-fake-boundary"
      | "live-agent-nondeterminism"
      | "unknown"
      | null;
  }>;
}

/**
 * Construct an `AgentPhaseDriver` from a parsed operator-results
 * payload. The factory accepts the parsed JSON (rather than a path)
 * so the `AgentBackend` can validate the file once during `prepare`
 * and surface a clean `runner-setup` failure when it is malformed.
 */
export class AgentPhaseDriver implements PhaseDriver {
  readonly name = "agent" as const;
  private readonly results: AgentOperatorResults;

  constructor(results: AgentOperatorResults) {
    this.results = results;
  }

  driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult> {
    return driveSliceWithBodies(opts, (o) => this.bodyFactory(o));
  }

  /**
   * Per-slice body factory. Looks up the operator-supplied bodies for
   * the slice; when a body is missing, falls back to the stub body
   * for that field so the C12 define-* assertions still see a
   * well-formed artifact set. Operators that want to opt out of
   * `design.md` (e.g. for the contracts capability) set `design: null`
   * explicitly; absence of the field defers to the capability brief.
   */
  private bodyFactory(opts: DriveSliceOpts): DefineBodies {
    const stubBodies = stubBodyFactory(opts);
    const slice = this.results.slices?.[opts.sliceName] ?? {};

    let design: string | null = stubBodies.design;
    if (slice.design === null) {
      design = null;
    } else if (typeof slice.design === "string") {
      design = slice.design;
    } else if (!capabilityRequiresDesign(opts.capabilityName)) {
      design = null;
    }

    return {
      proposal: slice.proposal ?? stubBodies.proposal,
      spec: slice.spec ?? stubBodies.spec,
      tasks: slice.tasks ?? stubBodies.tasks,
      design,
      residue: slice.residue,
    };
  }
}
