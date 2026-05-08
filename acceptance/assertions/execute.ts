// Execute-* assertion handlers (RM-01 plan, C10).
//
// Implements the four execution-stage assertion ids reserved by C06's
// `expected/plan-roles.md` and asserted by C10:
//
//   * `branch-prepared`              — every routed clone is on
//                                      `specify/<change>` after the
//                                      loop driver finishes.
//   * `baseline-merge-commit-clean`  — the per-slice baseline merge
//                                      commit (HEAD~1 in each routed
//                                      clone) touches only
//                                      `.specify/specs/` /
//                                      `.specify/archive/`.
//   * `residue-commit-non-empty`     — the per-slice residue commit
//                                      (HEAD in each routed clone) is
//                                      non-empty and touches paths
//                                      outside `.specify/`.
//   * `workspace-clean-before-push`  — `git status --porcelain` is
//                                      empty in every routed clone.
//
// Cascade-skip policy:
//   * upstream `setup-*` failure              → all four → `skip`
//   * upstream `plan-*` failure of any kind   → all four → `skip`
//     (the loop driver consumed a malformed plan; clone state is
//     untrustworthy)
//   * `ctx.run.executeState` undefined        → all four → `skip`
//     (a plan-only backend ran, e.g. `scripted-plan`)
//
// Wiring contract:
//   The runner's `defaultDispatch` registers these handlers behind
//   the same `ctx.setup && ctx.specifyBin` guard as the plan-* family
//   (see `acceptance/runner/assertions.ts`). Handlers self-skip when
//   `ctx.run.executeState` is missing rather than relying on the
//   dispatcher to gate registration — that way the same scenario file
//   can run under either `scripted-plan` (skips) or
//   `scripted-execute` (asserts) without re-shaping its assertion
//   list.

import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";
import { gitOutput, runGit, GitCommandError } from "../runner/git.ts";

/** Stable id list — useful for the smoke driver's `expected` set. */
export const EXECUTE_ASSERTION_IDS = [
  "branch-prepared",
  "baseline-merge-commit-clean",
  "residue-commit-non-empty",
  "workspace-clean-before-push",
] as const;

export type ExecuteAssertionId = typeof EXECUTE_ASSERTION_IDS[number];

/** Inputs the execute handlers need beyond the standard context. */
export interface ExecuteAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary (kept for parity with plan-roles, unused today). */
  specifyBin: SpecifyBin;
  /** Per-run Git env. Used for every `git` shell-out below. */
  env: GitEnv;
}

/**
 * Build the execute dispatch fragment. Returned as a `Map` so the
 * runner's default dispatch can `.set(...)` over it without touching
 * upstream registrations.
 */
export function executeHandlers(
  inputs: ExecuteAssertionInputs,
): Map<ExecuteAssertionId, AssertionHandler> {
  const map = new Map<ExecuteAssertionId, AssertionHandler>();
  map.set("branch-prepared", makeBranchPrepared(inputs));
  map.set("baseline-merge-commit-clean", makeBaselineMergeClean(inputs));
  map.set("residue-commit-non-empty", makeResidueNonEmpty(inputs));
  map.set("workspace-clean-before-push", makeWorkspaceClean(inputs));
  return map;
}

// -- Individual handlers -------------------------------------------------

function makeBranchPrepared(inputs: ExecuteAssertionInputs): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const exec = ctx.run.executeState!;
    const records: AssertionRecord[] = [];
    for (const project of exec.routedProjects) {
      const slot = join(exec.hubDir, ".specify", "workspace", project);
      try {
        const branch = await gitOutput(
          slot,
          ["branch", "--show-current"],
          inputs.env,
        );
        if (branch === exec.branch) {
          records.push(
            pass(id, `Routed clone is on \`${exec.branch}\`.`, {
              summary: `${project}: ${branch}`,
              paths: [slot],
            }),
          );
        } else {
          records.push(
            fail(
              id,
              `Routed clone is on \`${exec.branch}\`.`,
              {
                summary: `${project} on '${branch}' (expected '${exec.branch}')`,
                paths: [slot],
              },
              "cli-substrate",
            ),
          );
        }
      } catch (e) {
        records.push(
          fail(
            id,
            `Routed clone has a readable current branch.`,
            {
              summary: `${project}: ${oneLine(toMessage(e))}`,
              paths: [slot],
            },
            "runner-setup",
          ),
        );
      }
    }
    return { records };
  };
}

function makeBaselineMergeClean(
  inputs: ExecuteAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const exec = ctx.run.executeState!;
    const records: AssertionRecord[] = [];
    for (const project of exec.routedProjects) {
      const slot = join(exec.hubDir, ".specify", "workspace", project);
      const inspection = await inspectTopTwoCommits(slot, inputs.env);
      if (inspection.kind === "error") {
        records.push(
          fail(
            id,
            `Per-slice baseline merge commit is reachable.`,
            { summary: `${project}: ${inspection.message}`, paths: [slot] },
            "runner-setup",
          ),
        );
        continue;
      }
      const baseline = inspection.baseline;
      const offenders = baseline.files.filter(
        (f) => !isUnderSpecifyMeta(f),
      );
      if (offenders.length === 0) {
        records.push(
          pass(
            id,
            `Baseline merge commit touches only \`.specify/specs/\` and \`.specify/archive/\`.`,
            {
              summary:
                `${project}: ${baseline.subject} (${baseline.files.length} paths, all under .specify/)`,
              paths: [slot],
            },
          ),
        );
      } else {
        records.push(
          fail(
            id,
            `Baseline merge commit (\`specify: merge <slice>\`) must touch only \`.specify/specs/\` and \`.specify/archive/\`.`,
            {
              summary: `${project}: ${baseline.subject} also touches: ${
                offenders.slice(0, 5).join(", ")
              }${offenders.length > 5 ? "…" : ""}`,
              paths: [slot],
            },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

function makeResidueNonEmpty(
  inputs: ExecuteAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const exec = ctx.run.executeState!;
    const records: AssertionRecord[] = [];
    for (const project of exec.routedProjects) {
      const slot = join(exec.hubDir, ".specify", "workspace", project);
      const inspection = await inspectTopTwoCommits(slot, inputs.env);
      if (inspection.kind === "error") {
        records.push(
          fail(
            id,
            `Per-slice residue commit is reachable.`,
            { summary: `${project}: ${inspection.message}`, paths: [slot] },
            "runner-setup",
          ),
        );
        continue;
      }
      const residue = inspection.residue;
      if (residue.files.length === 0) {
        records.push(
          fail(
            id,
            `Residue commit (\`specify: residue <slice>\`) is non-empty.`,
            {
              summary: `${project}: ${residue.subject} touched 0 files`,
              paths: [slot],
            },
            "skill-orchestration",
          ),
        );
        continue;
      }
      const baselineLeaks = residue.files.filter(isUnderSpecifyMeta);
      if (baselineLeaks.length > 0) {
        records.push(
          fail(
            id,
            `Residue commit must not touch \`.specify/specs/\` or \`.specify/archive/\`.`,
            {
              summary: `${project}: ${residue.subject} also touches: ${
                baselineLeaks.slice(0, 5).join(", ")
              }${baselineLeaks.length > 5 ? "…" : ""}`,
              paths: [slot],
            },
            "skill-orchestration",
          ),
        );
        continue;
      }
      records.push(
        pass(
          id,
          `Residue commit is non-empty and lies entirely outside \`.specify/\`.`,
          {
            summary:
              `${project}: ${residue.subject} (${residue.files.length} path(s): ${
                residue.files.slice(0, 3).join(", ")
              }${residue.files.length > 3 ? "…" : ""})`,
            paths: [slot],
          },
        ),
      );
    }
    return { records };
  };
}

function makeWorkspaceClean(inputs: ExecuteAssertionInputs): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const exec = ctx.run.executeState!;
    const records: AssertionRecord[] = [];
    for (const project of exec.routedProjects) {
      const slot = join(exec.hubDir, ".specify", "workspace", project);
      try {
        const status = await gitOutput(
          slot,
          ["status", "--porcelain"],
          inputs.env,
        );
        if (status === "") {
          records.push(
            pass(id, `Routed clone is clean before push.`, {
              summary: `${project}: porcelain output empty`,
              paths: [slot],
            }),
          );
        } else {
          records.push(
            fail(
              id,
              `Routed clone has empty \`git status --porcelain\` output before push.`,
              {
                summary: `${project}: ${
                  status.split("\n").slice(0, 3).join(" / ")
                }${status.split("\n").length > 3 ? "…" : ""}`,
                paths: [slot],
              },
              "skill-orchestration",
            ),
          );
        }
      } catch (e) {
        records.push(
          fail(
            id,
            `Routed clone status is readable.`,
            {
              summary: `${project}: ${oneLine(toMessage(e))}`,
              paths: [slot],
            },
            "runner-setup",
          ),
        );
      }
    }
    return { records };
  };
}

// -- Helpers ------------------------------------------------------------

interface CommitInspection {
  subject: string;
  files: string[];
}

type InspectionResult =
  | { kind: "ok"; baseline: CommitInspection; residue: CommitInspection }
  | { kind: "error"; message: string };

/**
 * Read the top two commits on the current branch and return their
 * subject lines + touched paths. Convention from `cross_repo.rs`:
 *   HEAD     — `specify: residue <slice>`
 *   HEAD~1   — `specify: merge   <slice>`
 */
async function inspectTopTwoCommits(
  cwd: string,
  env: GitEnv,
): Promise<InspectionResult> {
  try {
    const subjects = await gitOutput(
      cwd,
      ["log", "--format=%s", "-2", "HEAD"],
      env,
    );
    const lines = subjects.split("\n").filter((s) => s.length > 0);
    if (lines.length < 2) {
      return {
        kind: "error",
        message: `expected at least 2 commits; saw ${lines.length} (${
          lines.join(" / ") || "<none>"
        })`,
      };
    }
    const residueSubject = lines[0];
    const baselineSubject = lines[1];

    const residueFiles = await listChangedFiles(cwd, "HEAD", env);
    const baselineFiles = await listChangedFiles(cwd, "HEAD~1", env);

    return {
      kind: "ok",
      baseline: { subject: baselineSubject, files: baselineFiles },
      residue: { subject: residueSubject, files: residueFiles },
    };
  } catch (e) {
    if (e instanceof GitCommandError) {
      return {
        kind: "error",
        message: `git log/diff failed: ${oneLine(e.run.stderr || e.run.stdout)}`,
      };
    }
    return { kind: "error", message: toMessage(e) };
  }
}

async function listChangedFiles(
  cwd: string,
  rev: string,
  env: GitEnv,
): Promise<string[]> {
  // `git show --name-only --format=` returns the touched paths only.
  const run = await runGit(
    cwd,
    ["show", "--name-only", "--format=", rev],
    env,
  );
  return run.stdout
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function isUnderSpecifyMeta(path: string): boolean {
  return path.startsWith(".specify/specs/") ||
    path.startsWith(".specify/archive/");
}

/** Decide whether to short-circuit a handler with a `skip` record. */
function gateOrSkip(
  id: string,
  ctx: AssertionContext,
): AssertionRecord | null {
  if (ctx.prior.some((r) => r.id.startsWith("setup-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `setup-*` assertion failed; execution evidence is not trustworthy.",
      "upstream setup-* failure",
    );
  }
  if (ctx.prior.some((r) => r.id.startsWith("plan-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `plan-*` assertion failed; the loop driver consumed a malformed plan.",
      "upstream plan-* failure",
    );
  }
  if (!ctx.run.executeState) {
    return skip(
      id,
      "Skipped because no execute backend ran (e.g. plan-only smoke via `scripted-plan`).",
      "ctx.executeState absent",
    );
  }
  return null;
}

function toMessage(e: unknown): string {
  return e instanceof Error ? `${e.name}: ${e.message}` : String(e);
}

function oneLine(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}
