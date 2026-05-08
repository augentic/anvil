// Contracts-build assertion handlers (RM-01 plan, C13).
//
// Implements the five C13 assertion ids that probe the contract slice
// after the `ContractsBuildPhaseDriver` has run. Together they form
// the oracle that says "the contract slice produced realistic OpenAPI
// 3.1 + JSON Schema YAML, the contracts WASI tool agrees the bundle
// is clean, and the merge promoted those files to the baseline path
// the implementation slices read from":
//
//   * `contract-slice-emits-yaml-artifacts`        — at least one
//                                                    `.yaml` file
//                                                    landed under
//                                                    `<hub>/contracts/**`.
//   * `contract-slice-yaml-validates-via-tool`     — `specify tool run
//                                                    contract` exited
//                                                    cleanly (parsed
//                                                    JSON `ok: true`).
//                                                    Skips with
//                                                    `cli-substrate`
//                                                    when the contract
//                                                    WASM is
//                                                    unavailable.
//   * `contract-slice-includes-openapi-or-asyncapi`— at least one file
//                                                    under `contracts/
//                                                    http/` OR
//                                                    `contracts/
//                                                    messages/`.
//   * `contract-slice-includes-required-schemas`   — the OAuth-relevant
//                                                    JSON-Schemas
//                                                    (token request /
//                                                    response, error
//                                                    response) exist
//                                                    under
//                                                    `contracts/
//                                                    schemas/`.
//   * `contract-baseline-files-present`            — every emitted
//                                                    contract YAML
//                                                    survived the merge
//                                                    and lives at the
//                                                    expected baseline
//                                                    path.
//
// Cascade-skip policy:
//   * upstream `setup-*` failure       → all five → `skip`.
//   * upstream `plan-*` failure        → all five → `skip`.
//   * `ctx.run.executeState` undefined → all five → `skip` (a plan-
//     only backend ran, e.g. `scripted-plan`).
//
// The validator handler builds a one-shot `runContractValidator`
// invocation per run (memoised across handler calls so we only pay
// the WASM resolution cost once). Other handlers re-use the parsed
// result if it surfaces useful detail, but the YAML-on-disk handlers
// stay independent so a missing-WASM skip does not cascade-suppress
// "did the bundle land on disk" coverage.

import { exists } from "jsr:@std/fs@1";
import { join } from "jsr:@std/path@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import {
  CONTRACT_YAML_PATHS,
} from "../runner/backends/contracts-build-driver.ts";
import {
  assertVerifierStatus,
  runContractValidator,
} from "./verifier.ts";
import type { ValidatorRun } from "./verifier.ts";
import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";

/** Stable id list — used by the smoke driver's `expected` set. */
export const CONTRACTS_BUILD_ASSERTION_IDS = [
  "contract-slice-emits-yaml-artifacts",
  "contract-slice-yaml-validates-via-tool",
  "contract-slice-includes-openapi-or-asyncapi",
  "contract-slice-includes-required-schemas",
  "contract-baseline-files-present",
] as const;

export type ContractsBuildAssertionId =
  typeof CONTRACTS_BUILD_ASSERTION_IDS[number];

/** Inputs shared across the contracts-build handlers. */
export interface ContractsBuildAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary; the validator handler calls `tool run contract`. */
  specifyBin: SpecifyBin;
  /** Per-run Git env. Reserved for parity with the other family inputs. */
  env: GitEnv;
  /** Optional override for the contract WASM path; defaults to env+autodiscovery. */
  contractWasmPath?: string;
}

/** Build the contracts-build dispatch fragment. */
export function contractsBuildHandlers(
  inputs: ContractsBuildAssertionInputs,
): Map<ContractsBuildAssertionId, AssertionHandler> {
  const cache: ValidatorCache = {};
  const map = new Map<ContractsBuildAssertionId, AssertionHandler>();
  map.set("contract-slice-emits-yaml-artifacts", makeEmitsYaml(inputs));
  map.set(
    "contract-slice-yaml-validates-via-tool",
    makeValidatesViaTool(inputs, cache),
  );
  map.set(
    "contract-slice-includes-openapi-or-asyncapi",
    makeIncludesOpenapiOrAsyncapi(inputs),
  );
  map.set(
    "contract-slice-includes-required-schemas",
    makeIncludesRequiredSchemas(inputs),
  );
  map.set("contract-baseline-files-present", makeBaselineFilesPresent(inputs));
  return map;
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

function makeEmitsYaml(
  inputs: ContractsBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };

    const contractsDir = join(inputs.setup.hubDir, "contracts");
    const yamls = await collectYamlFiles(contractsDir);
    if (yamls.length === 0) {
      // Skip rather than fail when the run produced zero contract
      // YAML at all — that is the "wrong backend" signal (the
      // `scripted-execute` / `scripted-finalize` / `agent` backends
      // do not drive contracts-build). The C13 contracts-build smoke
      // upgrades this to a fail-on-zero by setting
      // `assert: contracts-build` (see scenario frontmatter docs);
      // the other smokes leave it as a clean skip.
      return {
        records: [
          skip(
            id,
            "No contract YAML emitted by this backend; contract-build assertions are only meaningful under the `contracts-build` backend.",
            `no .yaml under ${contractsDir}`,
          ),
        ],
      };
    }
    return {
      records: [
        pass(id, `Contract slice emitted YAML artifacts.`, {
          summary: `${yamls.length} file(s) under contracts/`,
          paths: yamls.slice(0, 8),
        }),
      ],
    };
  };
}

function makeValidatesViaTool(
  inputs: ContractsBuildAssertionInputs,
  cache: ValidatorCache,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };

    // Skip when no contract YAML on disk — same "wrong backend"
    // signal as `contract-slice-emits-yaml-artifacts`.
    const contractsDir = join(inputs.setup.hubDir, "contracts");
    const yamls = await collectYamlFiles(contractsDir);
    if (yamls.length === 0) {
      return {
        records: [
          skip(
            id,
            "No contract YAML to validate; skipping validator invocation.",
            `no .yaml under ${contractsDir}`,
          ),
        ],
      };
    }

    const run = await runValidatorOnce(inputs, ctx, cache);
    if (run.kind === "skip") {
      return {
        records: [
          skip(
            id,
            `Contract validator unavailable — skipping clean-status check.`,
            run.reason,
          ),
        ],
      };
    }

    // Non-zero exit codes other than 1 (which means findings) usually
    // signal a substrate problem (binary mis-resolved, capability
    // sidecar wrong, etc.). Surface them distinctly so the operator
    // can attribute the failure to `cli-substrate` rather than
    // `specialist-generation`.
    if (run.exitCode !== 0 && run.exitCode !== 1) {
      return {
        records: [
          fail(
            id,
            `Contract validator failed to run cleanly (substrate-level error).`,
            {
              summary:
                `specify tool run contract exited ${run.exitCode}; ` +
                `stderr: ${run.stderr.slice(0, 240)}`,
              paths: [run.contractsArg],
            },
            "cli-substrate",
          ),
        ],
      };
    }

    // Schema parse + status check via the existing helper.
    const record = assertVerifierStatus({
      id,
      contractsDir: run.contractsArg,
      stdout: run.stdout,
      expected: "clean",
    });
    return { records: [record] };
  };
}

function makeIncludesOpenapiOrAsyncapi(
  inputs: ContractsBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };

    const contractsDir = join(inputs.setup.hubDir, "contracts");
    const httpDir = join(contractsDir, "http");
    const messagesDir = join(contractsDir, "messages");
    const httpYamls = await collectYamlFiles(httpDir);
    const messagesYamls = await collectYamlFiles(messagesDir);
    const all = [...httpYamls, ...messagesYamls];
    if (all.length === 0) {
      // Skip rather than fail when the contracts dir is empty (the
      // "wrong backend" signal documented under
      // `contract-slice-emits-yaml-artifacts`). A backend that
      // emits some YAML but none under http/ or messages/ would
      // still skip here — that case is rare enough that the cost
      // of weakening the failure mode is negligible.
      const contractsDir = join(inputs.setup.hubDir, "contracts");
      const anyYaml = await collectYamlFiles(contractsDir);
      if (anyYaml.length === 0) {
        return {
          records: [
            skip(
              id,
              "No contract YAML emitted by this backend; OpenAPI / AsyncAPI check is only meaningful under the `contracts-build` backend.",
              `no .yaml under ${contractsDir}`,
            ),
          ],
        };
      }
      return {
        records: [
          fail(
            id,
            `Contract slice must produce at least one OpenAPI (\`contracts/http/*.yaml\`) or AsyncAPI (\`contracts/messages/*.yaml\`) document.`,
            {
              summary:
                `no YAML under contracts/http/ or contracts/messages/`,
              paths: [httpDir, messagesDir],
            },
            "specialist-generation",
          ),
        ],
      };
    }
    return {
      records: [
        pass(id, `Contract slice produced OpenAPI / AsyncAPI artifacts.`, {
          summary:
            `${httpYamls.length} http + ${messagesYamls.length} messages YAML(s)`,
          paths: all.slice(0, 8),
        }),
      ],
    };
  };
}

function makeIncludesRequiredSchemas(
  inputs: ContractsBuildAssertionInputs,
): AssertionHandler {
  // OAuth-relevant schemas the C13 fixture must produce. The list
  // matches the canonical paths emitted by `ContractsBuildPhaseDriver`
  // so the assertion stays in sync with the driver without hard-
  // coding paths in two places — the driver is the source of truth.
  const required = CONTRACT_YAML_PATHS.filter((p) => p.startsWith("schemas/"));

  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };

    const contractsDir = join(inputs.setup.hubDir, "contracts");
    const anyYaml = await collectYamlFiles(contractsDir);
    if (anyYaml.length === 0) {
      return {
        records: [
          skip(
            id,
            "No contract YAML emitted by this backend; required-schemas check is only meaningful under the `contracts-build` backend.",
            `no .yaml under ${contractsDir}`,
          ),
        ],
      };
    }
    const missing: string[] = [];
    for (const rel of required) {
      const abs = join(contractsDir, rel);
      if (!(await exists(abs))) missing.push(rel);
    }
    if (missing.length > 0) {
      return {
        records: [
          fail(
            id,
            `Contract slice must include OAuth-relevant JSON Schemas (token request, token response, error response).`,
            {
              summary: `missing: ${missing.join(", ")}`,
              paths: missing.map((rel) => join(contractsDir, rel)),
            },
            "specialist-generation",
          ),
        ],
      };
    }
    return {
      records: [
        pass(id, `Contract slice includes required OAuth schemas.`, {
          summary: `${required.length} schema(s) present`,
          paths: required.map((rel) => join(contractsDir, rel)),
        }),
      ],
    };
  };
}

function makeBaselineFilesPresent(
  inputs: ContractsBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = gateOrSkip(id, ctx);
    if (gate) return { records: [gate] };

    // Post-merge baseline location is the hub's `contracts/` tree —
    // same place the driver wrote into, but conceptually checked
    // here AFTER the merge step would have run. The contract slice
    // is projectless so there is no routed-clone path to confirm.
    const contractsDir = join(inputs.setup.hubDir, "contracts");
    const anyYaml = await collectYamlFiles(contractsDir);
    if (anyYaml.length === 0) {
      return {
        records: [
          skip(
            id,
            "No contract YAML emitted by this backend; baseline-files-present check is only meaningful under the `contracts-build` backend.",
            `no .yaml under ${contractsDir}`,
          ),
        ],
      };
    }
    const records: AssertionRecord[] = [];
    for (const rel of CONTRACT_YAML_PATHS) {
      const abs = join(contractsDir, rel);
      if (await exists(abs)) {
        records.push(
          pass(id, `Baseline contract YAML present.`, {
            summary: `contracts/${rel}`,
            paths: [abs],
          }),
        );
      } else {
        records.push(
          fail(
            id,
            `Baseline contract YAML missing after merge.`,
            { summary: `contracts/${rel} not found`, paths: [abs] },
            "skill-orchestration",
          ),
        );
      }
    }
    return { records };
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface ValidatorCache {
  result?: ValidatorRun;
}

async function runValidatorOnce(
  inputs: ContractsBuildAssertionInputs,
  ctx: AssertionContext,
  cache: ValidatorCache,
): Promise<ValidatorRun> {
  if (cache.result) return cache.result;
  const contractsDir = join(inputs.setup.hubDir, "contracts");
  const result = await runContractValidator({
    specifyBin: inputs.specifyBin.path,
    contractsDir,
    contractWasmPath: inputs.contractWasmPath,
    scratchRoot: join(ctx.run.paths.runDir, "contract-validator-scratch"),
    logger: (line) => {
      // Best-effort; failure to log is non-fatal.
      console.error(`  ${line}`);
    },
  });
  cache.result = result;
  return result;
}

/** Recursively collect `.yaml` / `.yml` files under `root`. */
async function collectYamlFiles(root: string): Promise<string[]> {
  const out: string[] = [];
  try {
    for await (const entry of Deno.readDir(root)) {
      const full = join(root, entry.name);
      if (entry.isDirectory) {
        out.push(...(await collectYamlFiles(full)));
      } else if (
        entry.isFile &&
        (entry.name.endsWith(".yaml") || entry.name.endsWith(".yml"))
      ) {
        out.push(full);
      }
    }
  } catch {
    // Missing directory → empty list.
  }
  out.sort();
  return out;
}

/** Cascade-skip gate. */
function gateOrSkip(
  id: string,
  ctx: AssertionContext,
): AssertionRecord | null {
  if (ctx.prior.some((r) => r.id.startsWith("setup-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `setup-*` assertion failed.",
      "upstream setup-* failure",
    );
  }
  if (ctx.prior.some((r) => r.id.startsWith("plan-") && r.verdict === "fail")) {
    return skip(
      id,
      "Skipped because an upstream `plan-*` assertion failed.",
      "upstream plan-* failure",
    );
  }
  if (!ctx.run.executeState) {
    return skip(
      id,
      "Skipped because no execute backend ran (e.g. plan-only smoke).",
      "ctx.executeState absent",
    );
  }
  return null;
}
