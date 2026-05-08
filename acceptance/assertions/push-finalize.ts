// Push/Finalize assertion handlers (RM-01 plan, C11).
//
// Implements the assertion ids reserved by C06 for the cross-repo
// landing path (push → external merge simulation → finalize):
//
//   * `push-opens-pr-per-project`               — every routed
//                                                 project has a fake
//                                                 `gh` PR file in
//                                                 `OPEN` state on the
//                                                 expected branch.
//   * `push-output-json-shape-clean`            — `cli-substrate`
//                                                 fault-domain pin
//                                                 against `specify
//                                                 --format json
//                                                 workspace push`
//                                                 shape drift.
//   * `finalize-archives-plan`                  — `plan.yaml` is gone
//                                                 from the live tree
//                                                 and the archived
//                                                 plan exists under
//                                                 `.specify/archive/
//                                                 plans/<YYYYMMDD>-
//                                                 <change>/`.
//   * `finalize-output-json-shape-clean`        — `cli-substrate`
//                                                 fault-domain pin
//                                                 against `specify
//                                                 --format json
//                                                 change finalize`
//                                                 shape drift.
//   * `finalize-second-call-returns-plan-not-found`
//                                                 — second `change
//                                                 finalize` exits
//                                                 non-zero with
//                                                 `error: plan-not-
//                                                 found`.
//   * `finalize-runs-before-prs-merged`         — pre-merge
//                                                 `change finalize`
//                                                 refuses; logs a
//                                                 `cli-substrate`
//                                                 finding (never a
//                                                 hard fail) when the
//                                                 CLI accepts.
//
// Cascade-skip policy:
//   * upstream `setup-*` failure        → all six → `skip`
//   * upstream `plan-*` failure         → all six → `skip`
//   * upstream `execute-*` failure      → all six → `skip` (the loop
//     driver consumed bad state; push/finalize evidence is untrustworthy)
//   * `ctx.run.finalizeState` undefined → all six → `skip` (an
//     execute-only backend ran, e.g. `scripted-execute`)
//   * push handlers gate on `finalizeState.pushOutput` being present;
//     finalize handlers gate on `finalizeState.finalizeOutput`. Each
//     missing slot demotes its handler to `skip`.
//
// Wiring contract:
//   The runner's `defaultDispatch` registers these handlers behind
//   the same `ctx.setup && ctx.specifyBin` guard as the plan-* /
//   execute-* families (see `acceptance/runner/assertions.ts`).
//   Handlers self-skip when `ctx.run.finalizeState` is missing rather
//   than relying on the dispatcher to gate registration — that way the
//   same scenario file can run under either `scripted-execute`
//   (push/finalize-* skip) or `scripted-finalize` (push/finalize-*
//   assert) without re-shaping its assertion list.

import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import { readAllPrStates } from "../runner/fake-gh.ts";
import type { PrState } from "../runner/fake-gh.ts";
import type {
  FinalizeState,
  GitEnv,
  SetupHubResult,
  SpecifyBin,
} from "../runner/types.ts";

/** Stable id list — useful for the smoke driver's `expected` set. */
export const PUSH_FINALIZE_ASSERTION_IDS = [
  "push-opens-pr-per-project",
  "push-output-json-shape-clean",
  "finalize-archives-plan",
  "finalize-output-json-shape-clean",
  "finalize-second-call-returns-plan-not-found",
  "finalize-runs-before-prs-merged",
] as const;

export type PushFinalizeAssertionId =
  typeof PUSH_FINALIZE_ASSERTION_IDS[number];

/** Inputs the push/finalize handlers need beyond the standard context. */
export interface PushFinalizeAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary (kept for parity; unused by these handlers). */
  specifyBin: SpecifyBin;
  /** Per-run Git env (kept for parity; unused by these handlers). */
  env: GitEnv;
  /** Umbrella change name. Defaults to `oauth-login`. */
  changeName?: string;
  /** Expected push branch. Defaults to `specify/<changeName>`. */
  expectedBranch?: string;
}

/**
 * Build the push/finalize dispatch fragment. Returned as a `Map` so
 * the runner's default dispatch can `.set(...)` over it without
 * touching upstream registrations.
 */
export function pushFinalizeHandlers(
  inputs: PushFinalizeAssertionInputs,
): Map<PushFinalizeAssertionId, AssertionHandler> {
  const map = new Map<PushFinalizeAssertionId, AssertionHandler>();
  map.set("push-opens-pr-per-project", makePushOpensPrPerProject(inputs));
  map.set("push-output-json-shape-clean", makePushOutputShape(inputs));
  map.set("finalize-archives-plan", makeFinalizeArchivesPlan(inputs));
  map.set(
    "finalize-output-json-shape-clean",
    makeFinalizeOutputShape(inputs),
  );
  map.set(
    "finalize-second-call-returns-plan-not-found",
    makeFinalizeSecondCallNotFound(inputs),
  );
  map.set(
    "finalize-runs-before-prs-merged",
    makeFinalizeRunsBeforePrsMerged(inputs),
  );
  return map;
}

// -- Push handlers ------------------------------------------------------

function makePushOpensPrPerProject(
  inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "push");
    if (gate) return { records: [gate] };
    const expectedBranch = expectedBranchFor(inputs);
    const finalizeState = ctx.run.finalizeState!;
    const expectedProjects = Object.keys(inputs.setup.projectDirs);
    let states: PrState[];
    try {
      states = await readAllPrStates(inputs.setup.fakeGhStateDir);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return {
        records: [
          fail(
            id,
            `Fake-gh PR-state directory is readable.`,
            {
              summary: `cannot read ${inputs.setup.fakeGhStateDir}: ${msg}`,
              paths: [inputs.setup.fakeGhStateDir],
            },
            "external-fake-boundary",
          ),
        ],
      };
    }
    if (states.length !== expectedProjects.length) {
      return {
        records: [
          fail(
            id,
            `Fake-gh has one PR file per routed project.`,
            {
              summary:
                `expected ${expectedProjects.length} PR files (${expectedProjects.join(", ")}), ` +
                `got ${states.length} (${states.map((s) => s.repoKey).join(", ") || "<none>"})`,
              paths: [inputs.setup.fakeGhStateDir],
            },
            "cli-substrate",
          ),
        ],
      };
    }
    const records: AssertionRecord[] = [];
    // Cross-check each project against the prNumbers map captured
    // from the push-output JSON.
    for (const project of expectedProjects) {
      const expectedPr = finalizeState.prNumbers[project];
      const matching = states.find(
        (s) => s.repoKey.endsWith(`_${project}`) || s.repoKey === project,
      );
      if (!matching) {
        records.push(
          fail(
            id,
            `Fake-gh has a PR file for project \`${project}\`.`,
            {
              summary:
                `no PR file matches '${project}' (saw: ${states.map((s) => s.repoKey).join(", ")})`,
              paths: [inputs.setup.fakeGhStateDir],
            },
            "cli-substrate",
          ),
        );
        continue;
      }
      // After push (and BEFORE the backend marks PRs merged) the
      // PR-state file should be `OPEN`; AFTER the backend marks
      // them merged (which happens between push and finalize) the
      // file is `MERGED`. The handler runs at end-of-run, so we
      // accept either — the load-bearing invariant for this id is
      // "push opened a PR per project on the expected branch", not
      // "the PR is still open at the end of the run". The shape pin
      // for "still open right after push" lives in
      // `push-output-json-shape-clean` instead, which inspects the
      // captured push JSON (taken before the mark-merged step).
      const stateOk = matching.state === "OPEN" || matching.state === "MERGED";
      const branchOk = matching.branch === expectedBranch;
      const numberOk = expectedPr === undefined || matching.number === expectedPr;
      const issues: string[] = [];
      if (!stateOk) issues.push(`state=${matching.state}`);
      if (!branchOk) {
        issues.push(`branch=${matching.branch} (expected ${expectedBranch})`);
      }
      if (!numberOk) {
        issues.push(`number=${matching.number} (push reported ${expectedPr})`);
      }
      if (issues.length === 0) {
        records.push(
          pass(id, `Fake-gh PR file for \`${project}\` is well-formed.`, {
            summary:
              `${project}: PR #${matching.number} (${matching.state}) on ${matching.branch}`,
            paths: [matching.sourcePath],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Fake-gh PR file for \`${project}\` matches the post-push shape.`,
            {
              summary: `${project}: ${issues.join("; ")}`,
              paths: [matching.sourcePath],
            },
            "cli-substrate",
          ),
        );
      }
    }
    return { records };
  };
}

function makePushOutputShape(
  inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "push");
    if (gate) return Promise.resolve({ records: [gate] });
    const expectedBranch = expectedBranchFor(inputs);
    const fs = ctx.run.finalizeState!;
    const issues = pinPushShape(fs.pushOutput, expectedBranch);
    if (issues.length === 0) {
      return Promise.resolve({
        records: [
          pass(id, `\`workspace push\` JSON shape matches the C06 contract.`, {
            summary: `valid push output shape; pr numbers: ${
              Object.entries(fs.prNumbers)
                .map(([k, v]) => `${k}=${v}`)
                .join(", ") || "<none>"
            }`,
            paths: fs.pushOutputJson ? [fs.pushOutputJson] : [],
          }),
        ],
      });
    }
    return Promise.resolve({
      records: [
        fail(
          id,
          `\`workspace push\` JSON shape matches the C06 contract.`,
          {
            summary: issues.join("; "),
            paths: fs.pushOutputJson ? [fs.pushOutputJson] : [],
          },
          "cli-substrate",
        ),
      ],
    });
  };
}

// -- Finalize handlers --------------------------------------------------

function makeFinalizeArchivesPlan(
  inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "finalize");
    if (gate) return { records: [gate] };
    const fs = ctx.run.finalizeState!;
    const finalizeOut = fs.finalizeOutput as
      | { finalized?: unknown; archived?: unknown }
      | undefined;

    const livePlanPath = join(inputs.setup.hubDir, "plan.yaml");
    const livePlanGone = !(await pathExists(livePlanPath));

    const archivedPath = typeof finalizeOut?.archived === "string"
      ? finalizeOut.archived
      : null;
    const archivedExists = archivedPath ? await pathExists(archivedPath) : false;

    // Mirror cross_repo.rs: also assert there's at least one entry
    // under .specify/archive/plans/ that begins with "<change>-".
    const archiveDir = join(inputs.setup.hubDir, ".specify", "archive", "plans");
    const changeName = inputs.changeName ?? "oauth-login";
    let archiveDirHasChange = false;
    try {
      for await (const entry of Deno.readDir(archiveDir)) {
        if (entry.name.startsWith(`${changeName}-`)) {
          archiveDirHasChange = true;
          break;
        }
      }
    } catch {
      // dir missing — keep flag false; surfaced below
    }

    const issues: string[] = [];
    if (finalizeOut?.finalized !== true) {
      issues.push(`finalize JSON 'finalized' is not true`);
    }
    if (!livePlanGone) {
      issues.push(`live plan.yaml still exists at ${livePlanPath}`);
    }
    if (!archivedPath) {
      issues.push(`finalize JSON did not report an 'archived' path`);
    } else if (!archivedExists) {
      issues.push(`reported archived plan ${archivedPath} does not exist on disk`);
    }
    if (!archiveDirHasChange) {
      issues.push(
        `${archiveDir} has no entry starting with '${changeName}-'`,
      );
    }

    if (issues.length === 0) {
      return {
        records: [
          pass(id, `Finalize archived the plan.`, {
            summary:
              `plan.yaml moved → ${archivedPath}; archive dir contains '${changeName}-*' entry`,
            paths: archivedPath ? [archivedPath, archiveDir] : [archiveDir],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          id,
          `Finalize archived the plan and removed the live \`plan.yaml\`.`,
          {
            summary: issues.join("; "),
            paths: archivedPath ? [livePlanPath, archivedPath, archiveDir] : [livePlanPath, archiveDir],
          },
          "cli-substrate",
        ),
      ],
    };
  };
}

function makeFinalizeOutputShape(
  inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "finalize");
    if (gate) return Promise.resolve({ records: [gate] });
    const expectedBranch = expectedBranchFor(inputs);
    const expectedChange = inputs.changeName ?? "oauth-login";
    const fs = ctx.run.finalizeState!;
    const issues = pinFinalizeShape(fs.finalizeOutput, {
      expectedChange,
      expectedBranch,
      expectedProjectCount: Object.keys(inputs.setup.projectDirs).length,
    });
    if (issues.length === 0) {
      return Promise.resolve({
        records: [
          pass(id, `\`change finalize\` JSON shape matches the C06 contract.`, {
            summary: `valid finalize output shape (initiative=${expectedChange})`,
            paths: fs.finalizeOutputJson ? [fs.finalizeOutputJson] : [],
          }),
        ],
      });
    }
    return Promise.resolve({
      records: [
        fail(
          id,
          `\`change finalize\` JSON shape matches the C06 contract.`,
          {
            summary: issues.join("; "),
            paths: fs.finalizeOutputJson ? [fs.finalizeOutputJson] : [],
          },
          "cli-substrate",
        ),
      ],
    });
  };
}

function makeFinalizeSecondCallNotFound(
  _inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "finalize-second");
    if (gate) return Promise.resolve({ records: [gate] });
    const fs = ctx.run.finalizeState!;
    const second = fs.finalizeSecondOutput as
      | { error?: unknown; "exit-code"?: unknown }
      | undefined;
    const error = typeof second?.error === "string" ? second.error : null;
    if (error === "plan-not-found") {
      return Promise.resolve({
        records: [
          pass(
            id,
            `Second \`change finalize\` returns \`error: plan-not-found\`.`,
            {
              summary: `error=plan-not-found`,
              paths: fs.finalizeSecondCallJson ? [fs.finalizeSecondCallJson] : [],
            },
          ),
        ],
      });
    }
    return Promise.resolve({
      records: [
        fail(
          id,
          `Second \`change finalize\` returns \`error: plan-not-found\`.`,
          {
            summary: error
              ? `got error='${error}', expected 'plan-not-found'`
              : `second-call output had no 'error' field`,
            paths: fs.finalizeSecondCallJson ? [fs.finalizeSecondCallJson] : [],
          },
          "cli-substrate",
        ),
      ],
    });
  };
}

function makeFinalizeRunsBeforePrsMerged(
  _inputs: PushFinalizeAssertionInputs,
): AssertionHandler {
  return (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx, "finalize-pre-merge");
    if (gate) return Promise.resolve({ records: [gate] });
    const fs = ctx.run.finalizeState!;
    if (fs.finalizeRefusedPreMerge === undefined) {
      return Promise.resolve({
        records: [
          skip(
            id,
            `Pre-merge negative probe was not exercised by the backend.`,
            {
              summary: `finalizeRefusedPreMerge is undefined; backend did not run the probe`,
            },
          ),
        ],
      });
    }
    if (fs.finalizeRefusedPreMerge) {
      return Promise.resolve({
        records: [
          pass(
            id,
            `Pre-merge \`change finalize\` refuses while PRs are still open.`,
            {
              summary: `CLI exited non-zero with PRs OPEN — load-bearing RFC-14 guard holds`,
              paths: fs.finalizePreMergeJson ? [fs.finalizePreMergeJson] : [],
            },
          ),
        ],
      });
    }
    // The CLI accepted the call. Per C11 amendment we surface this
    // as a `cli-substrate` finding, not a hard suite failure — the
    // assertion is a forward-looking pin on RFC-14 guard behaviour
    // and the suite as a whole is still healthy.
    return Promise.resolve({
      records: [
        fail(
          id,
          `Pre-merge \`change finalize\` refuses while PRs are still open.`,
          {
            summary:
              `CLI exited 0 with PRs still OPEN — RFC-14 guard regressed; ` +
              `file a specify-cli follow-up. The suite did not abort: this is the ` +
              `negative-expectation pin from C06 §Negative Expectations.`,
            paths: fs.finalizePreMergeJson ? [fs.finalizePreMergeJson] : [],
          },
          "cli-substrate",
        ),
      ],
    });
  };
}

// -- Helpers ----------------------------------------------------------

function expectedBranchFor(inputs: PushFinalizeAssertionInputs): string {
  return inputs.expectedBranch ??
    `specify/${inputs.changeName ?? "oauth-login"}`;
}

/**
 * Decide whether to short-circuit a handler. `kind` selects which
 * `finalizeState` slot must be populated:
 *   * `push`              → handler reads the captured push JSON.
 *   * `finalize`          → handler reads the first-call finalize JSON.
 *   * `finalize-second`   → handler reads the second-call finalize JSON.
 *   * `finalize-pre-merge`→ handler reads the optional pre-merge probe.
 */
function gateOrSkip(
  id: string,
  ctx: AssertionContext,
  kind: "push" | "finalize" | "finalize-second" | "finalize-pre-merge",
): AssertionRecord | null {
  if (ctx.prior.some((r) => r.id.startsWith("setup-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `setup-*` assertion failed; landing evidence is not trustworthy.",
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
  if (
    ctx.prior.some((r) =>
      EXECUTE_PREREQ_IDS.has(r.id) && r.verdict === "fail"
    )
  ) {
    return skip(
      id,
      "Skipped because an upstream `execute-*` assertion failed; the workspace was not ready for push.",
      "upstream execute-* failure",
    );
  }
  if (!ctx.run.finalizeState) {
    return skip(
      id,
      "Skipped because no finalize backend ran (e.g. execute-only smoke via `scripted-execute`).",
      "ctx.finalizeState absent",
    );
  }
  const fs = ctx.run.finalizeState as FinalizeState;
  if (kind === "push" && fs.pushOutput === undefined) {
    return skip(
      id,
      "Skipped because the backend did not capture push JSON output.",
      "finalizeState.pushOutput absent",
    );
  }
  if (kind === "finalize" && fs.finalizeOutput === undefined) {
    return skip(
      id,
      "Skipped because the backend did not capture first-finalize JSON output.",
      "finalizeState.finalizeOutput absent",
    );
  }
  if (kind === "finalize-second" && fs.finalizeSecondOutput === undefined) {
    return skip(
      id,
      "Skipped because the backend did not run the idempotency probe.",
      "finalizeState.finalizeSecondOutput absent",
    );
  }
  // The pre-merge probe is intentionally optional. We only skip when
  // it never ran AND `finalizeRefusedPreMerge` is undefined; the
  // handler itself handles the populated cases.
  return null;
}

const EXECUTE_PREREQ_IDS = new Set<string>([
  "branch-prepared",
  "baseline-merge-commit-clean",
  "residue-commit-non-empty",
  "workspace-clean-before-push",
]);

/**
 * Pin the `workspace push` JSON shape per the Layer 0 substrate test:
 * top-level `projects` array, each element with `name` (string),
 * `status` (string), `branch` (string), `pr` (positive number).
 */
function pinPushShape(out: unknown, expectedBranch: string): string[] {
  const issues: string[] = [];
  if (!out || typeof out !== "object") {
    issues.push(`top-level not an object`);
    return issues;
  }
  const top = out as Record<string, unknown>;
  if (!Array.isArray(top.projects)) {
    issues.push(`top-level 'projects' is not an array`);
    return issues;
  }
  if (top.projects.length === 0) {
    issues.push(`top-level 'projects' is empty`);
  }
  for (let i = 0; i < top.projects.length; i++) {
    const raw = top.projects[i];
    if (!raw || typeof raw !== "object") {
      issues.push(`projects[${i}] is not an object`);
      continue;
    }
    const p = raw as Record<string, unknown>;
    if (typeof p.name !== "string" || p.name === "") {
      issues.push(`projects[${i}].name missing or non-string`);
    }
    if (p.status !== "pushed") {
      issues.push(`projects[${i}].status='${p.status}' (expected 'pushed')`);
    }
    if (typeof p.branch !== "string" || p.branch !== expectedBranch) {
      issues.push(
        `projects[${i}].branch='${p.branch}' (expected '${expectedBranch}')`,
      );
    }
    if (typeof p.pr !== "number" || p.pr <= 0) {
      issues.push(`projects[${i}].pr missing or non-positive`);
    }
  }
  return issues;
}

/**
 * Pin the `change finalize` JSON shape per the Layer 0 substrate
 * test: `initiative` (string), `finalized` (true), `projects` (array
 * matching expectedProjectCount, each merged), `summary.merged`
 * (number), `archived` (string path).
 */
function pinFinalizeShape(
  out: unknown,
  opts: {
    expectedChange: string;
    expectedBranch: string;
    expectedProjectCount: number;
  },
): string[] {
  const issues: string[] = [];
  if (!out || typeof out !== "object") {
    issues.push(`top-level not an object`);
    return issues;
  }
  const top = out as Record<string, unknown>;
  if (top.initiative !== opts.expectedChange) {
    issues.push(
      `initiative='${top.initiative}' (expected '${opts.expectedChange}')`,
    );
  }
  if (top.finalized !== true) {
    issues.push(`finalized=${JSON.stringify(top.finalized)} (expected true)`);
  }
  if (typeof top.archived !== "string" || top.archived === "") {
    issues.push(`archived missing or non-string`);
  }
  if (!Array.isArray(top.projects)) {
    issues.push(`projects is not an array`);
  } else {
    if (top.projects.length !== opts.expectedProjectCount) {
      issues.push(
        `projects.length=${top.projects.length} (expected ${opts.expectedProjectCount})`,
      );
    }
    for (let i = 0; i < top.projects.length; i++) {
      const raw = top.projects[i];
      if (!raw || typeof raw !== "object") {
        issues.push(`projects[${i}] is not an object`);
        continue;
      }
      const p = raw as Record<string, unknown>;
      if (typeof p.name !== "string" || p.name === "") {
        issues.push(`projects[${i}].name missing or non-string`);
      }
      if (p.status !== "merged") {
        issues.push(`projects[${i}].status='${p.status}' (expected 'merged')`);
      }
    }
  }
  const summary = top.summary as Record<string, unknown> | undefined;
  if (!summary || typeof summary !== "object") {
    issues.push(`summary missing or non-object`);
  } else if (
    typeof summary.merged !== "number" ||
    summary.merged !== opts.expectedProjectCount
  ) {
    issues.push(
      `summary.merged=${JSON.stringify(summary.merged)} (expected ${opts.expectedProjectCount})`,
    );
  }
  return issues;
}

async function pathExists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}
