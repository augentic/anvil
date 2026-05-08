// Plan-role assertion handlers (RM-01 plan, C09).
//
// Implements the nine `plan-*` assertion ids the C06 scenario pack
// declares for the cross-repo happy path. The rules themselves come
// from `acceptance/suites/rm01-cross-repo/expected/plan-roles.md` —
// that file is the canonical source of truth and the assertion ids
// here are 1:1 with the `Rule:` blocks there.
//
// Ids handled here:
//   * `plan-yaml-exists`
//   * `plan-validate-clean`
//   * `plan-has-one-contract-slice`
//   * `plan-has-one-backend-slice`
//   * `plan-has-one-mobile-slice`
//   * `backend-slice-routed-to-shop-backend`
//   * `mobile-slice-routed-to-shop-mobile`
//   * `implementation-slices-depend-on-contract`
//   * `contract-slice-projectless`
//
// Wiring contract:
//   1. The runner builds a `RunContext` with `setup?: SetupHubResult`
//      and `specifyBin?: SpecifyBin` populated by the cross-repo
//      backend's `prepare` step (see `backends/scripted-plan.ts`).
//   2. `planRoleHandlers(inputs)` returns a dispatch fragment the
//      runner merges into its default dispatch table for the RM-01
//      suite. Each handler shares one memoised plan parse + one
//      memoised contract-entry resolution.
//   3. Setup-* failures upstream demote every plan-* id to `skip`
//      (not `fail`) so failure attribution stays clean. Plan-yaml
//      missing demotes the rest of the plan-* ids to `skip` too.
//
// Soft "extra-entry" warning policy:
//   The role-based assertions count entries by *role*. If the planner
//   produces additional entries that match none of the three roles
//   (e.g. mobile split into iOS + Android), the rules still pass on
//   the role count; the extras are surfaced in the plan-yaml-exists
//   record with a `live-agent-nondeterminism` note. They never become
//   a hard failure here — that is C09's deliberate choice; tightening
//   should land alongside the agent backend.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";
import { runSpecify, SpecifyCommandError } from "../runner/specify-cli.ts";

/** Stable id list — useful for the smoke driver's `expected` set. */
export const PLAN_ROLE_ASSERTION_IDS = [
  "plan-yaml-exists",
  "plan-validate-clean",
  "plan-has-one-contract-slice",
  "plan-has-one-backend-slice",
  "plan-has-one-mobile-slice",
  "backend-slice-routed-to-shop-backend",
  "mobile-slice-routed-to-shop-mobile",
  "implementation-slices-depend-on-contract",
  "contract-slice-projectless",
] as const;

export type PlanRoleAssertionId = typeof PLAN_ROLE_ASSERTION_IDS[number];

/** Inputs the plan-role handlers need beyond the standard context. */
export interface PlanRoleAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary (for `plan-validate-clean`). */
  specifyBin: SpecifyBin;
  /** Per-run Git env. Used for the `specify change plan validate` shell-out. */
  env: GitEnv;
  /** Expected backend project name. Defaults to `shop-backend`. */
  backendProject?: string;
  /** Expected mobile project name. Defaults to `shop-mobile`. */
  mobileProject?: string;
}

/** One entry in `plan.yaml` after parsing. */
interface PlanEntry {
  name: string;
  project: string | null;
  schema: string | null;
  status: string | null;
  dependsOn: string[];
  description: string | null;
}

interface ParsedPlan {
  /** Absolute path the entries were read from. */
  path: string;
  /** Top-level `name:` if present (the change name). */
  name: string | null;
  entries: PlanEntry[];
}

/** Internal memoiser shared across every plan-role handler in a run. */
interface PlanCache {
  plan?: ParsedPlan | { error: AssertionRecord };
  contract?: PlanEntry | null;
}

/**
 * Build the plan-role dispatch fragment. Each handler shares a single
 * memoised plan parse so the file is read at most once per run.
 *
 * Returns a `Map` so callers can `.set(...)` to merge with the
 * runner's `defaultDispatch()`. The setup-* handlers run before the
 * plan-* handlers per the C09 amendment; this function does not
 * register them.
 */
export function planRoleHandlers(
  inputs: PlanRoleAssertionInputs,
): Map<PlanRoleAssertionId, AssertionHandler> {
  const cache: PlanCache = {};
  const map = new Map<PlanRoleAssertionId, AssertionHandler>();

  map.set("plan-yaml-exists", makePlanYamlExists(inputs, cache));
  map.set("plan-validate-clean", makePlanValidateClean(inputs, cache));
  map.set("plan-has-one-contract-slice", makeOneContractSlice(inputs, cache));
  map.set("plan-has-one-backend-slice", makeOneRoleSlice("plan-has-one-backend-slice", inputs, cache, "backend"));
  map.set("plan-has-one-mobile-slice", makeOneRoleSlice("plan-has-one-mobile-slice", inputs, cache, "mobile"));
  map.set("backend-slice-routed-to-shop-backend", makeRoutedTo("backend-slice-routed-to-shop-backend", inputs, cache, "backend"));
  map.set("mobile-slice-routed-to-shop-mobile", makeRoutedTo("mobile-slice-routed-to-shop-mobile", inputs, cache, "mobile"));
  map.set("implementation-slices-depend-on-contract", makeImplDependsOnContract(inputs, cache));
  map.set("contract-slice-projectless", makeContractProjectless(inputs, cache));

  return map;
}

// -- Individual handlers -------------------------------------------------

function makePlanYamlExists(
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) {
      return { records: [skipUpstreamSetup(id)] };
    }
    const planPath = await resolvePlanPath(inputs, ctx);
    const parsed = await loadPlan(planPath, cache);
    if ("error" in parsed) return { records: [parsed.error] };

    const extras = describeExtras(parsed, inputs);
    const archived = planPath !== planPathFor(inputs);
    const note = archived ? " (loaded from archive — `change finalize` ran)" : "";
    const summary = extras
      ? `plan.yaml present (${parsed.entries.length} entries — ${extras})${note}`
      : `plan.yaml present (${parsed.entries.length} entries)${note}`;
    return {
      records: [
        pass(id, "Hub `plan.yaml` exists and is readable.", {
          summary,
          paths: [planPath],
        }),
      ],
    };
  };
}

function makePlanValidateClean(
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) {
      return { records: [skipUpstreamSetup(id)] };
    }
    if (skipBecausePlanMissing(ctx)) {
      return { records: [skipUpstreamPlan(id)] };
    }
    const livePlanPath = planPathFor(inputs);
    const liveExists = await pathReadable(livePlanPath);
    // C11: post-finalize the live plan.yaml is gone; `change plan
    // validate` would correctly refuse with no-plan. Skip-with-rationale
    // — the plan content was already validated upstream while live by
    // the C09/C10 smoke runs (and by `plan-yaml-exists` here through
    // the archive fallback).
    if (!liveExists) {
      return {
        records: [
          skip(
            id,
            "`specify change plan validate` is skipped post-finalize: live `plan.yaml` was archived by `change finalize`. Plan content already validated by upstream `plan-*` rules via the archive fallback in `resolvePlanPath`.",
            "live plan.yaml absent (post-finalize)",
          ),
        ],
      };
    }
    // Force the plan to be loaded so a missing plan.yaml fails this id
    // even when callers run it without `plan-yaml-exists` first.
    const planPath = livePlanPath;
    const parsed = await loadPlan(planPath, cache);
    if ("error" in parsed) return { records: [parsed.error] };

    try {
      const run = await runSpecify({
        bin: inputs.specifyBin,
        cwd: inputs.setup.hubDir,
        args: ["change", "plan", "validate"],
        env: inputs.env,
      });
      return {
        records: [
          pass(id, "`specify change plan validate` exits clean.", {
            summary: `exit 0; ${oneLine(run.stdout) || "(no stdout)"}`,
            paths: [planPath],
          }),
        ],
      };
    } catch (e) {
      if (e instanceof SpecifyCommandError) {
        return {
          records: [
            fail(
              id,
              "`specify change plan validate` exits clean.",
              {
                summary: `exit ${e.run.exitCode}; ${oneLine(e.run.stderr) || oneLine(e.run.stdout) || "(no output)"}`,
                paths: [planPath],
              },
              "cli-substrate",
            ),
          ],
        };
      }
      const msg = e instanceof Error ? e.message : String(e);
      return {
        records: [
          fail(
            id,
            "`specify change plan validate` exits clean.",
            { summary: `runner failed to invoke specify: ${msg}` },
            "runner-setup",
          ),
        ],
      };
    }
  };
}

function makeOneContractSlice(
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) return { records: [skipUpstreamSetup(id)] };
    if (skipBecausePlanMissing(ctx)) return { records: [skipUpstreamPlan(id)] };

    const plan = await loadPlan(await resolvePlanPath(inputs, ctx), cache);
    if ("error" in plan) return { records: [plan.error] };

    const contracts = plan.entries.filter(isContractRole);
    cache.contract = contracts.length === 1 ? contracts[0] : null;

    if (contracts.length === 1) {
      return {
        records: [
          pass(id, "Plan has exactly one contract-role entry.", {
            summary: `1 contract entry: ${contracts[0].name} (schema=${contracts[0].schema})`,
            paths: [plan.path],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          id,
          "Plan has exactly one entry matching the contract role (schema=^contracts@v\\d+$, no project, empty depends-on).",
          {
            summary: contracts.length === 0
              ? "0 contract-role entries; planner skipped the contract slice"
              : `${contracts.length} contract-role entries: ${contracts.map((e) => e.name).join(", ")}`,
            paths: [plan.path],
          },
          "skill-orchestration",
        ),
      ],
    };
  };
}

function makeOneRoleSlice(
  id: PlanRoleAssertionId,
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
  role: "backend" | "mobile",
): AssertionHandler {
  return async (rid, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) return { records: [skipUpstreamSetup(rid)] };
    if (skipBecausePlanMissing(ctx)) return { records: [skipUpstreamPlan(rid)] };

    const plan = await loadPlan(await resolvePlanPath(inputs, ctx), cache);
    if ("error" in plan) return { records: [plan.error] };

    const target = projectFor(inputs, role);
    const matches = plan.entries.filter((e) => e.project === target);
    const desc =
      `Plan has exactly one entry with \`project: ${target}\`.`;
    if (matches.length === 1) {
      return {
        records: [
          pass(rid, desc, {
            summary: `1 ${role} entry: ${matches[0].name}`,
            paths: [plan.path],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          rid,
          desc,
          {
            summary: matches.length === 0
              ? `0 entries routed to ${target}`
              : `${matches.length} entries routed to ${target}: ${matches.map((e) => e.name).join(", ")}`,
            paths: [plan.path],
          },
          "skill-orchestration",
        ),
      ],
    };
  };
}

function makeRoutedTo(
  id: PlanRoleAssertionId,
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
  role: "backend" | "mobile",
): AssertionHandler {
  return async (rid, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) return { records: [skipUpstreamSetup(rid)] };
    if (skipBecausePlanMissing(ctx)) return { records: [skipUpstreamPlan(rid)] };

    const plan = await loadPlan(await resolvePlanPath(inputs, ctx), cache);
    if ("error" in plan) return { records: [plan.error] };

    const target = projectFor(inputs, role);
    const matches = plan.entries.filter((e) =>
      e.project === target || (e.project !== null && hintsAtRole(e, role))
    );
    if (matches.length !== 1) {
      // If the plan doesn't have exactly one role entry, the upstream
      // counting rule already failed. Skip routing to avoid double
      // reporting and to keep the failure surface focused.
      return {
        records: [
          skip(
            rid,
            `Routing rule skipped because plan-has-one-${role}-slice did not identify a unique ${role} entry.`,
            {
              summary: `${matches.length} candidate ${role} entries`,
              paths: [plan.path],
            },
          ),
        ],
      };
    }
    const entry = matches[0];
    if (entry.project === target) {
      return {
        records: [
          pass(rid, `Unique ${role} entry is routed to \`${target}\`.`, {
            summary: `${entry.name}.project == ${target}`,
            paths: [plan.path],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          rid,
          `Unique ${role} entry is routed to \`${target}\`.`,
          {
            summary: `${entry.name}.project = ${entry.project ?? "<unset>"} (expected ${target})`,
            paths: [plan.path],
          },
          "skill-orchestration",
        ),
      ],
    };
  };
}

function makeImplDependsOnContract(
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
): AssertionHandler {
  return async (rid, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) return { records: [skipUpstreamSetup(rid)] };
    if (skipBecausePlanMissing(ctx)) return { records: [skipUpstreamPlan(rid)] };

    const plan = await loadPlan(await resolvePlanPath(inputs, ctx), cache);
    if ("error" in plan) return { records: [plan.error] };

    const contracts = plan.entries.filter(isContractRole);
    if (contracts.length !== 1) {
      return {
        records: [
          skip(
            rid,
            "Dependency rule skipped: plan-has-one-contract-slice did not identify a unique contract entry.",
            {
              summary: `${contracts.length} contract-role entries`,
              paths: [plan.path],
            },
          ),
        ],
      };
    }
    const contractId = contracts[0].name;
    const backend = plan.entries.find((e) => e.project === projectFor(inputs, "backend"));
    const mobile = plan.entries.find((e) => e.project === projectFor(inputs, "mobile"));
    if (!backend || !mobile) {
      return {
        records: [
          skip(
            rid,
            "Dependency rule skipped: backend or mobile entry missing (upstream role rules will surface the cause).",
            {
              summary: `backend=${backend?.name ?? "<missing>"}, mobile=${mobile?.name ?? "<missing>"}`,
              paths: [plan.path],
            },
          ),
        ],
      };
    }
    const offenders: string[] = [];
    if (!backend.dependsOn.includes(contractId)) offenders.push(`${backend.name}: depends-on=${formatDeps(backend.dependsOn)}`);
    if (!mobile.dependsOn.includes(contractId)) offenders.push(`${mobile.name}: depends-on=${formatDeps(mobile.dependsOn)}`);
    if (offenders.length === 0) {
      return {
        records: [
          pass(rid, "Both implementation slices declare the contract entry as a dependency.", {
            summary:
              `contract=${contractId}; backend=${backend.name} depends-on contract; mobile=${mobile.name} depends-on contract`,
            paths: [plan.path],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          rid,
          "Both implementation slices must declare the contract entry as a dependency.",
          {
            summary: `missing dep on '${contractId}': ${offenders.join("; ")}`,
            paths: [plan.path],
          },
          "skill-orchestration",
        ),
      ],
    };
  };
}

function makeContractProjectless(
  inputs: PlanRoleAssertionInputs,
  cache: PlanCache,
): AssertionHandler {
  return async (rid, ctx): Promise<AssertionResult> => {
    if (skipBecauseSetupFailed(ctx)) return { records: [skipUpstreamSetup(rid)] };
    if (skipBecausePlanMissing(ctx)) return { records: [skipUpstreamPlan(rid)] };

    const plan = await loadPlan(await resolvePlanPath(inputs, ctx), cache);
    if ("error" in plan) return { records: [plan.error] };

    const contracts = plan.entries.filter(isContractRole);
    if (contracts.length !== 1) {
      return {
        records: [
          skip(
            rid,
            "Projectless rule skipped: plan-has-one-contract-slice did not identify a unique contract entry.",
            {
              summary: `${contracts.length} contract-role entries`,
              paths: [plan.path],
            },
          ),
        ],
      };
    }
    const entry = contracts[0];
    const projectIsAbsent = entry.project === null || entry.project === "";
    if (projectIsAbsent) {
      return {
        records: [
          pass(rid, "Contract entry has no `project:` field.", {
            summary: `${entry.name}.project = null`,
            paths: [plan.path],
          }),
        ],
      };
    }
    return {
      records: [
        fail(
          rid,
          "Contract entry has no `project:` field (or its value is null/empty).",
          {
            summary: `${entry.name}.project = ${entry.project}`,
            paths: [plan.path],
          },
          "skill-orchestration",
        ),
      ],
    };
  };
}

// -- Helpers -------------------------------------------------------------

function planPathFor(inputs: PlanRoleAssertionInputs): string {
  return join(inputs.setup.hubDir, "plan.yaml");
}

/**
 * Resolve the best plan.yaml source for handlers that read the plan
 * AFTER `change finalize` may have archived the live file.
 *
 * Search order:
 *   1. live `<hubDir>/plan.yaml` (pre-finalize / post-create);
 *   2. snapshot at `<runDir>/plan.yaml.before-finalize` written by the
 *      C11 `scripted-finalize` backend before it ran finalize;
 *   3. archived `<hubDir>/.specify/archive/plans/<change>-<ts>/plan.yaml`
 *      (matches `cross_repo.rs` post-finalize archive layout).
 *
 * Returns the live path unchanged when no fallback exists, so the
 * downstream `loadPlan` failure path still surfaces a clean
 * `plan-yaml-exists` failure for genuinely missing plans.
 */
async function resolvePlanPath(
  inputs: PlanRoleAssertionInputs,
  ctx: AssertionContext,
): Promise<string> {
  const live = planPathFor(inputs);
  if (await pathReadable(live)) return live;
  const snapshot = join(ctx.run.paths.runDir, "plan.yaml.before-finalize");
  if (await pathReadable(snapshot)) return snapshot;
  const archiveDir = join(
    inputs.setup.hubDir,
    ".specify",
    "archive",
    "plans",
  );
  try {
    for await (const entry of Deno.readDir(archiveDir)) {
      if (!entry.isDirectory) continue;
      const candidate = join(archiveDir, entry.name, "plan.yaml");
      if (await pathReadable(candidate)) return candidate;
    }
  } catch {
    // archive dir absent — fall through to live path
  }
  return live;
}

async function pathReadable(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}

function projectFor(inputs: PlanRoleAssertionInputs, role: "backend" | "mobile"): string {
  return role === "backend"
    ? (inputs.backendProject ?? "shop-backend")
    : (inputs.mobileProject ?? "shop-mobile");
}

/** Heuristic — the live agent might pick a different project name. */
function hintsAtRole(entry: PlanEntry, role: "backend" | "mobile"): boolean {
  if (entry.project === null) return false;
  return role === "backend"
    ? /backend|api|server/i.test(entry.project)
    : /mobile|client|app/i.test(entry.project);
}

function isContractRole(e: PlanEntry): boolean {
  if (e.project !== null && e.project !== "") return false;
  if (!e.schema) return false;
  if (!/^contracts@v\d+$/.test(e.schema)) return false;
  if (e.dependsOn.length !== 0) return false;
  return true;
}

function describeExtras(
  plan: ParsedPlan,
  inputs: PlanRoleAssertionInputs,
): string | null {
  const contracts = plan.entries.filter(isContractRole);
  const backend = plan.entries.filter((e) => e.project === projectFor(inputs, "backend"));
  const mobile = plan.entries.filter((e) => e.project === projectFor(inputs, "mobile"));
  const accountedFor = new Set([
    ...contracts.map((e) => e.name),
    ...backend.map((e) => e.name),
    ...mobile.map((e) => e.name),
  ]);
  const extras = plan.entries.filter((e) => !accountedFor.has(e.name));
  if (extras.length === 0) return null;
  return `extras: ${extras.map((e) => e.name).join(", ")} (live-agent-nondeterminism note; role rules unaffected)`;
}

async function loadPlan(
  path: string,
  cache: PlanCache,
): Promise<ParsedPlan | { error: AssertionRecord }> {
  if (cache.plan) {
    if ("error" in cache.plan) return cache.plan;
    return cache.plan;
  }
  let body: string;
  try {
    body = await Deno.readTextFile(path);
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) {
      const rec = fail(
        "plan-yaml-exists",
        "Hub `plan.yaml` exists and is readable.",
        { summary: `plan.yaml not found`, paths: [path] },
        "skill-orchestration",
      );
      cache.plan = { error: rec };
      return cache.plan;
    }
    const msg = e instanceof Error ? e.message : String(e);
    const rec = fail(
      "plan-yaml-exists",
      "Hub `plan.yaml` exists and is readable.",
      { summary: `cannot read plan.yaml: ${msg}`, paths: [path] },
      "runner-setup",
    );
    cache.plan = { error: rec };
    return cache.plan;
  }
  let parsed: unknown;
  try {
    parsed = parseYaml(body);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const rec = fail(
      "plan-yaml-exists",
      "Hub `plan.yaml` parses as YAML.",
      { summary: `invalid YAML: ${msg}`, paths: [path] },
      "cli-substrate",
    );
    cache.plan = { error: rec };
    return cache.plan;
  }
  const plan = coercePlan(parsed, path);
  cache.plan = plan;
  return plan;
}

function coercePlan(parsed: unknown, path: string): ParsedPlan {
  const obj = (parsed ?? {}) as Record<string, unknown>;
  // The CLI's `plan.yaml` shape carries entries under `changes:`; older
  // shapes used `entries:`. Accept either to stay forward-compatible.
  const rawEntries = Array.isArray(obj.changes)
    ? obj.changes
    : Array.isArray(obj.entries)
    ? obj.entries
    : [];
  const entries: PlanEntry[] = rawEntries.map((raw): PlanEntry => {
    const r = (raw ?? {}) as Record<string, unknown>;
    const dependsOnRaw = r["depends-on"];
    const dependsOn = Array.isArray(dependsOnRaw)
      ? dependsOnRaw.filter((v): v is string => typeof v === "string")
      : [];
    return {
      name: typeof r.name === "string" ? r.name : "",
      project: typeof r.project === "string" && r.project !== ""
        ? r.project
        : null,
      schema: typeof r.schema === "string" ? r.schema : null,
      status: typeof r.status === "string" ? r.status : null,
      dependsOn,
      description: typeof r.description === "string" ? r.description : null,
    };
  });
  return {
    path,
    name: typeof obj.name === "string" ? obj.name : null,
    entries,
  };
}

function skipBecauseSetupFailed(ctx: AssertionContext): boolean {
  return ctx.prior.some((r) => r.id.startsWith("setup-") && r.verdict === "fail");
}

function skipBecausePlanMissing(ctx: AssertionContext): boolean {
  const planExists = ctx.prior.find((r) => r.id === "plan-yaml-exists");
  return Boolean(planExists && planExists.verdict === "fail");
}

function skipUpstreamSetup(id: string): AssertionRecord {
  return skip(
    id,
    "Skipped because an upstream `setup-*` assertion failed; plan-level evidence is not trustworthy.",
    "upstream setup-* failure",
  );
}

function skipUpstreamPlan(id: string): AssertionRecord {
  return skip(
    id,
    "Skipped because `plan-yaml-exists` failed; downstream plan-level rules cannot be evaluated.",
    "plan.yaml absent or unreadable",
  );
}

function formatDeps(deps: string[]): string {
  return deps.length === 0 ? "[]" : `[${deps.join(", ")}]`;
}

function oneLine(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}
