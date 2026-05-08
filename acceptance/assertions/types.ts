// Shared types for acceptance assertion helpers (RM-01 plan, C05).
//
// Helpers under `acceptance/assertions/` are the pass/fail oracle for a
// scenario run. The runner (see `acceptance/runner/`) collects evidence;
// the helpers here turn evidence into structured `AssertionRecord`s that
// land in `assertions.json` and the run summary.
//
// These types intentionally re-export the runner's `AssertionRecord` and
// `FaultDomain` so suites and helpers share one vocabulary. Helpers must
// not invent new fault-domain strings; the runner taxonomy is the
// authoritative list (see `acceptance/runner/README.md` §Failure
// Reporting).

export type {
  AssertionRecord,
  FaultDomain,
} from "../runner/types.ts";

import type { AssertionRecord, FaultDomain } from "../runner/types.ts";
import type { RunContext } from "../runner/types.ts";

/**
 * Structured payload an assertion handler may attach to its evidence
 * pointer. Helpers are free to flatten this into a single string for
 * `AssertionRecord.evidence`; this type just documents the shape we
 * expect across handlers so summary rendering can stay consistent.
 */
export interface AssertionEvidence {
  /** One-line summary suitable for `assertions.json` `evidence`. */
  summary: string;
  /** Optional list of file paths the operator should inspect. */
  paths?: string[];
  /** Optional structured detail (kept short — full output goes in logs). */
  details?: Record<string, unknown>;
}

/**
 * Context passed to every assertion handler. Helpers see only what they
 * need: the scenario, the workspace, the run context, and (for
 * cross-handler signalling) the partial set of records already produced
 * by earlier handlers in this run.
 */
export interface AssertionContext {
  /** The full run context the runner built for this scenario. */
  run: RunContext;
  /** Convenience alias for `run.paths.workspace`. */
  workspace: string;
  /**
   * Records produced earlier in the assertions stage. Mostly used by
   * follow-up handlers that want to skip themselves when a prerequisite
   * already failed. Read-only.
   */
  prior: ReadonlyArray<AssertionRecord>;
}

/**
 * Result a single helper returns. The runner merges these into the
 * existing `assertions.json` payload. A helper may return multiple
 * records for one assertion id when the id maps to several checks
 * (e.g. one record per missing path).
 */
export interface AssertionResult {
  /** Records to append. Order is preserved in `assertions.json`. */
  records: AssertionRecord[];
}

/**
 * Stable signature for a handler registered in the dispatch table.
 *
 * `id` is the assertion id from the scenario frontmatter. Handlers are
 * pure with respect to the runner: they read on-disk evidence, they do
 * not start backends or mutate scenario state.
 */
export type AssertionHandler = (
  id: string,
  ctx: AssertionContext,
) => Promise<AssertionResult>;

/**
 * Helper for builders to produce a passing record without rewriting the
 * verbose object literal.
 */
export function pass(
  id: string,
  description: string,
  evidence: AssertionEvidence | string,
): AssertionRecord {
  return {
    id,
    description,
    verdict: "pass",
    evidence: renderEvidence(evidence),
    "fault-domain": null,
  };
}

/** Helper for builders to produce a failing record. */
export function fail(
  id: string,
  description: string,
  evidence: AssertionEvidence | string,
  faultDomain: FaultDomain,
): AssertionRecord {
  return {
    id,
    description,
    verdict: "fail",
    evidence: renderEvidence(evidence),
    "fault-domain": faultDomain,
  };
}

/** Helper for builders to produce a skipped record (with rationale). */
export function skip(
  id: string,
  description: string,
  evidence: AssertionEvidence | string,
): AssertionRecord {
  return {
    id,
    description,
    verdict: "skip",
    evidence: renderEvidence(evidence),
    "fault-domain": null,
  };
}

function renderEvidence(e: AssertionEvidence | string): string {
  if (typeof e === "string") return e;
  if (e.paths && e.paths.length > 0) {
    return `${e.summary} [${e.paths.join(", ")}]`;
  }
  return e.summary;
}
