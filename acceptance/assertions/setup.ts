// Setup-time assertion handlers (RM-01 plan, C07).
//
// These are the four `setup-*` invariant gates reserved by C06's
// `expected/registry.yaml.skeleton.md`. They run after the C07 setup
// primitives finish but BEFORE any plan-level / execute-level rule, so
// downstream failures can be cleanly attributed to "the planner did
// the wrong thing" vs. "we never had a clean hub to begin with".
//
// Assertion ids landed here:
//   * `setup-hub-project-yaml-has-hub-true-and-no-capability`
//   * `setup-registry-has-two-entries`
//   * `setup-registry-entries-have-non-empty-descriptions`
//   * `setup-registry-validate-clean`
//
// The handlers read on-disk hub state directly (no `specify` calls)
// for the structural checks; the validate-clean check shells out to
// `specify registry validate` so we observe the verdict the CLI itself
// would surface.
//
// C05 has landed `acceptance/assertions/types.ts` (record + handler
// shape) but no dispatch table yet. The handlers below conform to
// C05's `AssertionHandler` so C09 can wire them through whatever
// dispatch C05 ultimately ships, without having to rename or reshape
// anything here.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import type { GitEnv } from "../runner/git.ts";
import { runSpecify } from "../runner/specify-cli.ts";
import { SpecifyCommandError } from "../runner/specify-cli.ts";
import type { SpecifyBin } from "../runner/specify-cli.ts";

/** Stable list of setup assertion ids — useful for the smoke driver. */
export const SETUP_ASSERTION_IDS = [
  "setup-hub-project-yaml-has-hub-true-and-no-capability",
  "setup-registry-has-two-entries",
  "setup-registry-entries-have-non-empty-descriptions",
  "setup-registry-validate-clean",
] as const;

export type SetupAssertionId = typeof SETUP_ASSERTION_IDS[number];

/**
 * Inputs the setup handlers need beyond the standard
 * `AssertionContext`. The C09 wiring is expected to thread these
 * through `RunContext` — until then, the C07 smoke driver passes them
 * via `runSetupAssertions` below.
 */
export interface SetupAssertionInputs {
  hubDir: string;
  /** Resolved `specify` binary (for `setup-registry-validate-clean`). */
  specifyBin: SpecifyBin;
  /** Per-run Git env (for the `specify registry validate` subprocess). */
  env: GitEnv;
  /** Optional override for "two entries" — defaults to 2. */
  expectedEntryCount?: number;
}

/** Dispatch table mapping assertion id → handler factory. */
export function setupHandlers(
  inputs: SetupAssertionInputs,
): Record<SetupAssertionId, AssertionHandler> {
  return {
    "setup-hub-project-yaml-has-hub-true-and-no-capability":
      makeHubProjectYamlHandler(inputs),
    "setup-registry-has-two-entries":
      makeRegistryEntryCountHandler(inputs),
    "setup-registry-entries-have-non-empty-descriptions":
      makeRegistryDescriptionsHandler(inputs),
    "setup-registry-validate-clean":
      makeRegistryValidateCleanHandler(inputs),
  };
}

/**
 * Run all four setup assertions in order and return the records.
 *
 * Used by the C07 smoke target. C09 should drop this in favour of
 * whatever dispatch table C05 lands; the handlers themselves are
 * stable.
 */
export async function runSetupAssertions(
  inputs: SetupAssertionInputs,
  ctx: AssertionContext,
): Promise<AssertionRecord[]> {
  const handlers = setupHandlers(inputs);
  const records: AssertionRecord[] = [];
  for (const id of SETUP_ASSERTION_IDS) {
    const handlerCtx: AssertionContext = { ...ctx, prior: records.slice() };
    const result = await handlers[id](id, handlerCtx);
    records.push(...result.records);
  }
  return records;
}

// -- Individual handlers -------------------------------------------------

function makeHubProjectYamlHandler(
  inputs: SetupAssertionInputs,
): AssertionHandler {
  return async (id, _ctx): Promise<AssertionResult> => {
    const path = join(inputs.hubDir, ".specify", "project.yaml");
    let body: string;
    try {
      body = await Deno.readTextFile(path);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return {
        records: [
          fail(
            id,
            "Hub project.yaml is readable.",
            `${path} not readable: ${msg}`,
            "runner-setup",
          ),
        ],
      };
    }

    let parsed: Record<string, unknown>;
    try {
      parsed = parseYaml(body) as Record<string, unknown>;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return {
        records: [
          fail(
            id,
            "Hub project.yaml parses as YAML.",
            { summary: `invalid YAML: ${msg}`, paths: [path] },
            "cli-substrate",
          ),
        ],
      };
    }

    const issues: string[] = [];
    if (parsed.hub !== true) {
      issues.push(`hub != true (got ${JSON.stringify(parsed.hub ?? null)})`);
    }
    if (parsed.capability !== undefined) {
      issues.push(
        `capability is present (got ${JSON.stringify(parsed.capability)}); hub project.yaml must omit capability per RFC-9 §1D`,
      );
    }

    if (issues.length > 0) {
      return {
        records: [
          fail(
            id,
            "Hub project.yaml carries `hub: true` and omits `capability:`.",
            { summary: issues.join("; "), paths: [path] },
            "cli-substrate",
          ),
        ],
      };
    }
    return {
      records: [
        pass(
          id,
          "Hub project.yaml carries `hub: true` and omits `capability:`.",
          { summary: "hub: true; no capability field", paths: [path] },
        ),
      ],
    };
  };
}

function makeRegistryEntryCountHandler(
  inputs: SetupAssertionInputs,
): AssertionHandler {
  const expected = inputs.expectedEntryCount ?? 2;
  return async (id, _ctx): Promise<AssertionResult> => {
    const result = await readRegistry(inputs.hubDir, id);
    if (result.kind === "error") return { records: [result.record] };
    const actual = result.projects.length;
    if (actual === expected) {
      return {
        records: [
          pass(
            id,
            `Registry lists ${expected} project entries.`,
            {
              summary: `${actual} entries: ${result.projects.map((p) => p.name).join(", ")}`,
              paths: [result.path],
            },
          ),
        ],
      };
    }
    return {
      records: [
        fail(
          id,
          `Registry lists exactly ${expected} project entries.`,
          {
            summary: `expected ${expected}, got ${actual} (${result.projects.map((p) => p.name).join(", ") || "<none>"})`,
            paths: [result.path],
          },
          "runner-setup",
        ),
      ],
    };
  };
}

function makeRegistryDescriptionsHandler(
  inputs: SetupAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const priorCountFailed = ctx.prior.some(
      (r) => r.id === "setup-registry-has-two-entries" && r.verdict === "fail",
    );
    const result = await readRegistry(inputs.hubDir, id);
    if (result.kind === "error") return { records: [result.record] };

    const missing = result.projects.filter(
      (p) => typeof p.description !== "string" || p.description.trim() === "",
    );
    if (priorCountFailed && result.projects.length === 0) {
      return {
        records: [
          skip(
            id,
            "Skipped because the registry has no entries to describe.",
            { summary: "no entries", paths: [result.path] },
          ),
        ],
      };
    }
    if (missing.length === 0) {
      return {
        records: [
          pass(
            id,
            "All registry entries carry a non-empty description.",
            {
              summary: `${result.projects.length} entries described`,
              paths: [result.path],
            },
          ),
        ],
      };
    }
    return {
      records: [
        fail(
          id,
          "Every registry entry carries a non-empty description.",
          {
            summary: `entries missing description: ${missing.map((p) => p.name).join(", ")}`,
            paths: [result.path],
          },
          "runner-setup",
        ),
      ],
    };
  };
}

function makeRegistryValidateCleanHandler(
  inputs: SetupAssertionInputs,
): AssertionHandler {
  return async (id, _ctx): Promise<AssertionResult> => {
    try {
      const run = await runSpecify({
        bin: inputs.specifyBin,
        cwd: inputs.hubDir,
        args: ["registry", "validate"],
        env: inputs.env,
      });
      return {
        records: [
          pass(
            id,
            "`specify registry validate` exits clean.",
            {
              summary: `exit 0; ${oneLine(run.stdout) || "(no stdout)"}`,
            },
          ),
        ],
      };
    } catch (e) {
      if (e instanceof SpecifyCommandError) {
        return {
          records: [
            fail(
              id,
              "`specify registry validate` exits clean.",
              {
                summary: `exit ${e.run.exitCode}; ${oneLine(e.run.stderr) || "(no stderr)"}`,
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
            "`specify registry validate` exits clean.",
            { summary: `runner failed to invoke specify: ${msg}` },
            "runner-setup",
          ),
        ],
      };
    }
  };
}

// -- Internal helpers ----------------------------------------------------

interface RegistryProjectEntry {
  name: string;
  description?: string;
  url?: string;
  schema?: string;
}

type RegistryReadResult =
  | { kind: "ok"; path: string; projects: RegistryProjectEntry[] }
  | { kind: "error"; record: AssertionRecord };

async function readRegistry(
  hubDir: string,
  id: string,
): Promise<RegistryReadResult> {
  const path = join(hubDir, "registry.yaml");
  let body: string;
  try {
    body = await Deno.readTextFile(path);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return {
      kind: "error",
      record: fail(
        id,
        "Hub registry.yaml is readable.",
        `${path} not readable: ${msg}`,
        "runner-setup",
      ),
    };
  }
  let parsed: Record<string, unknown>;
  try {
    parsed = parseYaml(body) as Record<string, unknown>;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return {
      kind: "error",
      record: fail(
        id,
        "Hub registry.yaml parses as YAML.",
        { summary: `invalid YAML: ${msg}`, paths: [path] },
        "cli-substrate",
      ),
    };
  }

  const rawProjects = Array.isArray(parsed.projects) ? parsed.projects : [];
  const projects: RegistryProjectEntry[] = rawProjects.map((p) => {
    const r = (p ?? {}) as Record<string, unknown>;
    return {
      name: typeof r.name === "string" ? r.name : "",
      description: typeof r.description === "string" ? r.description : undefined,
      url: typeof r.url === "string" ? r.url : undefined,
      schema: typeof r.schema === "string" ? r.schema : undefined,
    };
  });

  return { kind: "ok", path, projects };
}

function oneLine(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}
