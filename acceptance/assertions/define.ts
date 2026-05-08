// Define-stage assertion handlers (RM-01 plan, C12).
//
// Implements the seven assertion ids the C12 amendment reserved for
// the cross-repo define + merge stages:
//
//   * `slice-has-proposal`                          — every plan entry
//                                                     produced a
//                                                     `proposal.md`.
//   * `slice-has-spec`                              — every plan entry
//                                                     produced a
//                                                     `spec.md`
//                                                     (or, for
//                                                     forward-compat,
//                                                     `specs/main.md`).
//   * `slice-has-design-when-required`              — every plan entry
//                                                     whose capability
//                                                     declares a
//                                                     `design` brief
//                                                     produced
//                                                     `design.md`.
//   * `slice-has-tasks`                             — every plan entry
//                                                     produced a
//                                                     `tasks.md`.
//   * `slice-baseline-promoted`                     — every plan
//                                                     entry's spec
//                                                     directory lives
//                                                     in the baseline
//                                                     tree (routed
//                                                     clone for impl
//                                                     slices, hub for
//                                                     the contract
//                                                     slice).
//   * `slice-archived`                              — every plan
//                                                     entry's archive
//                                                     directory exists
//                                                     under
//                                                     `.specify/archive/<slice>/`.
//   * `implementation-slice-reads-baseline-contract`
//                                                   — every
//                                                     implementation
//                                                     slice's
//                                                     `proposal.md` or
//                                                     `design.md`
//                                                     references the
//                                                     baseline
//                                                     `contracts/`
//                                                     tree (the
//                                                     load-bearing
//                                                     RM-01
//                                                     contract-first
//                                                     invariant).
//
// Per-capability `design.md` policy:
//   * `contracts` → no `design` brief; the assertion records `pass`
//     for contract-role slices regardless of file presence.
//   * `omnia`, `vectis` → `design` brief required; missing file is a
//     `capability-brief` failure (the slice's body factory dropped
//     the artifact).
//   The map lives in `phase-driver.ts::CAPABILITY_REQUIRES_DESIGN` and
//   is reused here so adding a capability stays a one-line change.
//
// Cascade-skip policy:
//   * upstream `setup-*` failure       → all seven → `skip`
//   * upstream `plan-*` failure        → all seven → `skip` (the loop
//     driver consumed a malformed plan; per-slice evidence is
//     untrustworthy)
//   * `ctx.run.executeState` undefined → all seven → `skip` (a plan-
//     only backend ran, e.g. `scripted-plan`)
//
// File-search policy:
//   The handlers prefer `<root>/.specify/specs/<slice>/<file>` (the
//   real `/spec:define` output location) and fall back to
//   `<root>/.specify/archive/<slice>/<file>` (the location the C10
//   stub baseline merge commit also writes into). Either location
//   counts as "the artifact exists". `<root>` is the routed clone
//   for impl slices and the hub root for the contract slice.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import { capabilityRequiresDesign } from "../runner/backends/phase-driver.ts";
import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";

/** Stable id list — useful for the smoke driver's `expected` set. */
export const DEFINE_ASSERTION_IDS = [
  "slice-has-proposal",
  "slice-has-spec",
  "slice-has-design-when-required",
  "slice-has-tasks",
  "slice-baseline-promoted",
  "slice-archived",
  "implementation-slice-reads-baseline-contract",
] as const;

export type DefineAssertionId = typeof DEFINE_ASSERTION_IDS[number];

/** Inputs the define handlers need beyond the standard context. */
export interface DefineAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary (kept for parity; unused today). */
  specifyBin: SpecifyBin;
  /** Per-run Git env (kept for parity; unused today). */
  env: GitEnv;
  /**
   * Per-slice capability lookup. The handlers consult this for the
   * `slice-has-design-when-required` decision. Defaults to the RM-01
   * fixture map (contract → `contracts`, shop-backend → `omnia`,
   * shop-mobile → `vectis`).
   */
  capabilityForSlice?: (sliceName: string, project: string | null) =>
    | string
    | undefined;
}

/** Build the define dispatch fragment. */
export function defineHandlers(
  inputs: DefineAssertionInputs,
): Map<DefineAssertionId, AssertionHandler> {
  const cache: SliceCache = {};
  const map = new Map<DefineAssertionId, AssertionHandler>();
  map.set("slice-has-proposal", makeSliceHasFile(inputs, cache, "proposal.md"));
  map.set("slice-has-spec", makeSliceHasSpec(inputs, cache));
  map.set(
    "slice-has-design-when-required",
    makeSliceHasDesignWhenRequired(inputs, cache),
  );
  map.set("slice-has-tasks", makeSliceHasFile(inputs, cache, "tasks.md"));
  map.set("slice-baseline-promoted", makeBaselinePromoted(inputs, cache));
  map.set("slice-archived", makeArchived(inputs, cache));
  map.set(
    "implementation-slice-reads-baseline-contract",
    makeReadsBaseline(inputs, cache),
  );
  return map;
}

// --- Handlers -------------------------------------------------------

function makeSliceHasFile(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
  fileName: "proposal.md" | "tasks.md",
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const records: AssertionRecord[] = [];
    for (const slice of plan.slices) {
      const root = sliceRoot(inputs, slice);
      const located = await findArtifact(root, slice.name, fileName);
      if (located) {
        records.push(
          pass(id, `Slice produced \`${fileName}\`.`, {
            summary: `${slice.name}: ${located.relPath}`,
            paths: [located.absPath],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Slice produced \`${fileName}\` under \`.specify/specs/<slice>/\` or \`.specify/archive/<slice>/\`.`,
            {
              summary:
                `${slice.name}: ${fileName} missing from both ` +
                `\`.specify/specs/${slice.name}/\` and ` +
                `\`.specify/archive/${slice.name}/\``,
              paths: [root],
            },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

function makeSliceHasSpec(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const records: AssertionRecord[] = [];
    for (const slice of plan.slices) {
      const root = sliceRoot(inputs, slice);
      // Real `/spec:define` writes either `spec.md` or
      // `specs/main.md` depending on capability brief style; both are
      // valid. The C10 stub writes `spec.md` only.
      const located = (await findArtifact(root, slice.name, "spec.md")) ??
        (await findArtifact(root, slice.name, join("specs", "main.md")));
      if (located) {
        records.push(
          pass(id, `Slice produced \`spec.md\` (or \`specs/main.md\`).`, {
            summary: `${slice.name}: ${located.relPath}`,
            paths: [located.absPath],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Slice produced \`spec.md\` (or \`specs/main.md\`) under \`.specify/specs/<slice>/\` or \`.specify/archive/<slice>/\`.`,
            {
              summary:
                `${slice.name}: neither spec.md nor specs/main.md found ` +
                `under \`.specify/specs/${slice.name}/\` or ` +
                `\`.specify/archive/${slice.name}/\``,
              paths: [root],
            },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

function makeSliceHasDesignWhenRequired(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const lookup = inputs.capabilityForSlice ?? defaultCapabilityForSlice;

    const records: AssertionRecord[] = [];
    for (const slice of plan.slices) {
      const capability = lookup(slice.name, slice.project);
      if (!capabilityRequiresDesign(capability) && capability) {
        records.push(
          pass(
            id,
            `Capability \`${capability}\` does not require \`design.md\`; slice is exempt.`,
            {
              summary: `${slice.name}: capability=${capability} (no design brief)`,
            },
          ),
        );
        continue;
      }
      const root = sliceRoot(inputs, slice);
      const located = await findArtifact(root, slice.name, "design.md");
      if (located) {
        records.push(
          pass(id, `Capability requires \`design.md\` and the slice produced one.`, {
            summary:
              `${slice.name} (capability=${capability ?? "<unknown>"}): ${located.relPath}`,
            paths: [located.absPath],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Capability requires \`design.md\`; slice must produce one under \`.specify/specs/<slice>/\` or \`.specify/archive/<slice>/\`.`,
            {
              summary:
                `${slice.name} (capability=${capability ?? "<unknown>"}): ` +
                `design.md missing from both define-stage locations`,
              paths: [root],
            },
            "capability-brief",
          ),
        );
      }
    }
    return { records };
  };
}

function makeBaselinePromoted(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const records: AssertionRecord[] = [];
    for (const slice of plan.slices) {
      const root = sliceRoot(inputs, slice);
      const specDir = join(root, ".specify", "specs", slice.name);
      if (await dirExists(specDir)) {
        records.push(
          pass(id, `Slice's spec directory exists in the baseline tree.`, {
            summary: `${slice.name}: .specify/specs/${slice.name}/`,
            paths: [specDir],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Slice's spec directory must exist in the baseline tree (\`.specify/specs/<slice>/\`).`,
            {
              summary: `${slice.name}: missing ${specDir}`,
              paths: [root],
            },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

function makeArchived(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const records: AssertionRecord[] = [];
    for (const slice of plan.slices) {
      const root = sliceRoot(inputs, slice);
      const archiveDir = join(root, ".specify", "archive", slice.name);
      if (await dirExists(archiveDir)) {
        records.push(
          pass(id, `Slice archive directory exists.`, {
            summary: `${slice.name}: .specify/archive/${slice.name}/`,
            paths: [archiveDir],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Slice archive directory must exist after merge (\`.specify/archive/<slice>/\`).`,
            {
              summary: `${slice.name}: missing ${archiveDir}`,
              paths: [root],
            },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

function makeReadsBaseline(
  inputs: DefineAssertionInputs,
  cache: SliceCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };
    const plan = await loadPlanSlices(inputs, ctx, cache);
    if ("error" in plan) return { records: [plan.error(id)] };

    const impls = plan.slices.filter((s) => s.project !== null);
    if (impls.length === 0) {
      return {
        records: [
          skip(
            id,
            "No implementation slices in plan; nothing to check.",
            "no impl slices",
          ),
        ],
      };
    }

    const records: AssertionRecord[] = [];
    for (const slice of impls) {
      const root = sliceRoot(inputs, slice);
      const candidates = [
        await findArtifact(root, slice.name, "design.md"),
        await findArtifact(root, slice.name, "proposal.md"),
        await findArtifact(root, slice.name, "spec.md"),
      ].filter((c): c is LocatedArtifact => c !== null);

      if (candidates.length === 0) {
        records.push(
          fail(
            id,
            `Implementation slice's define-stage artifacts must reference baseline \`contracts/\`.`,
            {
              summary:
                `${slice.name}: no proposal/spec/design artifacts to inspect`,
              paths: [root],
            },
            "skill-orchestration",
          ),
        );
        continue;
      }

      const matches: string[] = [];
      for (const c of candidates) {
        const body = await Deno.readTextFile(c.absPath);
        if (mentionsContractBaseline(body)) {
          matches.push(c.relPath);
        }
      }

      if (matches.length > 0) {
        records.push(
          pass(
            id,
            `Implementation slice's define-stage artifacts reference baseline \`contracts/\`.`,
            {
              summary: `${slice.name}: matched in ${matches.join(", ")}`,
              paths: [candidates[0].absPath],
            },
          ),
        );
      } else {
        records.push(
          fail(
            id,
            `Implementation slice must reference baseline \`contracts/\` (the load-bearing RM-01 contract-first invariant).`,
            {
              summary:
                `${slice.name}: no \`contracts/\` reference in ` +
                candidates.map((c) => c.relPath).join(", "),
              paths: candidates.map((c) => c.absPath),
            },
            "capability-brief",
          ),
        );
      }
    }
    return { records };
  };
}

// --- Helpers --------------------------------------------------------

/** A single plan entry the handlers iterate over. */
interface SliceEntry {
  name: string;
  /** `null` for the contract role; project name for impl slices. */
  project: string | null;
}

/**
 * Memoised plan parse result shared across every define-* handler in
 * one run. Failure carries an error factory so each handler can
 * report the failure under its own id.
 */
interface SliceCache {
  plan?: { slices: SliceEntry[] } | { error: (id: string) => AssertionRecord };
}

interface LocatedArtifact {
  absPath: string;
  relPath: string;
}

/** Default RM-01 fixture capability lookup. */
function defaultCapabilityForSlice(
  _sliceName: string,
  project: string | null,
): string | undefined {
  if (project === null) return "contracts";
  if (project === "shop-backend") return "omnia";
  if (project === "shop-mobile") return "vectis";
  return undefined;
}

/**
 * Resolve the on-disk root the per-slice artifacts live under.
 *   * Routed slice → `<hubDir>/.specify/workspace/<project>/`
 *   * Contract slice → `<hubDir>/`
 */
function sliceRoot(
  inputs: DefineAssertionInputs,
  slice: SliceEntry,
): string {
  if (slice.project === null) return inputs.setup.hubDir;
  return join(inputs.setup.hubDir, ".specify", "workspace", slice.project);
}

/**
 * Locate an artifact under a slice root. Search order:
 *   1. `<root>/.specify/specs/<slice>/<rel>`
 *   2. `<root>/.specify/archive/<slice>/<rel>`
 * Returns the first match, or `null` if neither exists.
 */
async function findArtifact(
  root: string,
  sliceName: string,
  rel: string,
): Promise<LocatedArtifact | null> {
  const candidates = [
    {
      absPath: join(root, ".specify", "specs", sliceName, rel),
      relPath: `.specify/specs/${sliceName}/${rel}`,
    },
    {
      absPath: join(root, ".specify", "archive", sliceName, rel),
      relPath: `.specify/archive/${sliceName}/${rel}`,
    },
  ];
  for (const c of candidates) {
    try {
      const stat = await Deno.stat(c.absPath);
      if (stat.isFile) return c;
    } catch {
      // continue
    }
  }
  return null;
}

async function dirExists(path: string): Promise<boolean> {
  try {
    const stat = await Deno.stat(path);
    return stat.isDirectory;
  } catch {
    return false;
  }
}

/**
 * Heuristic check: does the artifact body mention the baseline
 * `contracts/` tree? Accepts any of:
 *   * a literal `contracts/` path reference,
 *   * a markdown link to a `contracts/` file,
 *   * the exact phrase "baseline contracts".
 *
 * Real `/spec:define` output for the RM-01 fixture should reference
 * the merged `contracts/oauth-login.yaml` (or similar) by path; the
 * stub body factory writes "References baseline `contracts/...`" so
 * the assertion passes against stub-quality artifacts as well.
 */
function mentionsContractBaseline(body: string): boolean {
  if (/contracts\/[\w./-]+\.ya?ml/.test(body)) return true;
  if (/`contracts\//.test(body)) return true;
  if (/baseline\s+contracts/i.test(body)) return true;
  return false;
}

async function loadPlanSlices(
  inputs: DefineAssertionInputs,
  ctx: AssertionContext,
  cache: SliceCache,
): Promise<{ slices: SliceEntry[] } | { error: (id: string) => AssertionRecord }> {
  if (cache.plan) return cache.plan;
  const planPath = await resolvePlanPath(inputs, ctx);
  let body: string;
  try {
    body = await Deno.readTextFile(planPath);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const result = {
      error: (id: string): AssertionRecord =>
        fail(
          id,
          `Hub \`plan.yaml\` is readable.`,
          { summary: `cannot read plan.yaml: ${msg}`, paths: [planPath] },
          "runner-setup",
        ),
    };
    cache.plan = result;
    return result;
  }
  let parsed: unknown;
  try {
    parsed = parseYaml(body);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const result = {
      error: (id: string): AssertionRecord =>
        fail(
          id,
          `Hub \`plan.yaml\` parses as YAML.`,
          { summary: `invalid YAML: ${msg}`, paths: [planPath] },
          "cli-substrate",
        ),
    };
    cache.plan = result;
    return result;
  }
  const obj = (parsed ?? {}) as Record<string, unknown>;
  const rawEntries = Array.isArray(obj.changes)
    ? obj.changes
    : Array.isArray(obj.entries)
    ? obj.entries
    : [];
  const slices: SliceEntry[] = rawEntries.map((raw): SliceEntry => {
    const r = (raw ?? {}) as Record<string, unknown>;
    return {
      name: typeof r.name === "string" ? r.name : "",
      project: typeof r.project === "string" && r.project !== ""
        ? r.project
        : null,
    };
  }).filter((s) => s.name.length > 0);
  const result = { slices };
  cache.plan = result;
  return result;
}

/**
 * Same fallback logic as `plan-roles.ts::resolvePlanPath`: live →
 * `<runDir>/plan.yaml.before-finalize` → archived plan. Inlined so
 * the define handlers stay self-contained.
 */
async function resolvePlanPath(
  inputs: DefineAssertionInputs,
  ctx: AssertionContext,
): Promise<string> {
  const live = join(inputs.setup.hubDir, "plan.yaml");
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

/** Cascade-skip gate. */
function gateOrSkip(
  id: string,
  ctx: AssertionContext,
): AssertionRecord | null {
  if (ctx.prior.some((r) => r.id.startsWith("setup-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `setup-*` assertion failed; define-stage evidence is not trustworthy.",
      "upstream setup-* failure",
    );
  }
  if (ctx.prior.some((r) => r.id.startsWith("plan-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `plan-*` assertion failed; define-stage evidence is not trustworthy.",
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
