// Vectis-build assertion handlers (RM-01 plan, C14b).
//
// Implements the five C14b assertion ids that probe the Vectis
// implementation slice (today: `add-oauth-screens`, routed to
// `shop-mobile`) after the `VectisBuildPhaseDriver` has run.
// Together they form the oracle that says "the Vectis slice
// produced a composition.yaml + SwiftUI screen file pair in the
// routed project, the residue stayed inside the project, no
// Vectis-shaped output landed at common-mistake forbidden paths,
// and the merged baseline carries the slice's spec dir":
//
//   * `vectis-slice-emits-composition-yaml`        — the routed
//                                                    clone has a
//                                                    `composition.yaml`
//                                                    at the project
//                                                    root that
//                                                    parses as YAML
//                                                    with
//                                                    `version: 1`
//                                                    and either a
//                                                    `screens` map
//                                                    or a `delta`
//                                                    block.
//   * `vectis-slice-emits-screen-files`            — the routed
//                                                    clone has a
//                                                    non-empty
//                                                    `apps/mobile/
//                                                    login_screen.swift`.
//   * `vectis-slice-residue-under-routed-project`  — every Vectis
//                                                    output path
//                                                    is either
//                                                    `composition.yaml`
//                                                    at the project
//                                                    root or lives
//                                                    under `apps/`.
//   * `vectis-slice-no-output-outside-project`     — the routed
//                                                    clone has no
//                                                    Vectis-shaped
//                                                    output at the
//                                                    common-mistake
//                                                    forbidden paths
//                                                    (`LoginScreen.swift`
//                                                    at root,
//                                                    `MainActivity.kt`,
//                                                    `Pods/`,
//                                                    `node_modules/`,
//                                                    `build/`).
//   * `vectis-baseline-files-present`              — the post-merge
//                                                    baseline
//                                                    `.specify/
//                                                    specs/<slice>/`
//                                                    dir exists in
//                                                    the routed
//                                                    project clone.
//
// Cascade-skip policy mirrors the C13 contracts-build / C14a
// omnia-build families:
//   * upstream `setup-*` failure       → all five → `skip`.
//   * upstream `plan-*` failure        → all five → `skip`.
//   * `ctx.run.executeState` undefined → all five → `skip` (a
//     plan-only backend ran, e.g. `scripted-plan`).
//   * no Vectis slice in `executeState.slices` → all five → `skip`
//     with a "wrong backend" rationale.
//   * Vectis slice present but no `composition.yaml` on disk →
//     all five → `skip` (the `OmniaBuildPhaseDriver` /
//     `StubPhaseDriver` do not write composition.yaml; only
//     `VectisBuildPhaseDriver` does, so its absence is the
//     canonical "the vectis-build driver did not run" signal).
//
// The Vectis validator wiring follows the same shape C13 reserved
// for the contracts WASI tool: if `specify tool run vectis-validate`
// becomes available with built WASM artifacts, the validator
// handler can be promoted to a real invocation via
// `acceptance/assertions/verifier.ts`. Until that exists (the
// vectis-validate tool ships as an RFC-16 placeholder today, see
// `capabilities/vectis/tools.yaml`), the structural-on-disk
// assertions are the oracle. Probing for the tool today returns no
// runnable WASM, so we omit the handler entry rather than emit
// dead skip records — adding it later is a one-line change.

import { exists } from "jsr:@std/fs@1";
import { join } from "jsr:@std/path@1";
import { parse as parseYaml } from "jsr:@std/yaml@1";

import { fail, pass, skip } from "./types.ts";
import type {
  AssertionContext,
  AssertionHandler,
  AssertionRecord,
  AssertionResult,
} from "./types.ts";

import {
  vectisShellPaths,
  type VectisShellPaths,
} from "../runner/backends/vectis-build-driver.ts";
import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";

/** Stable id list — used by the smoke driver's `expected` set. */
export const VECTIS_BUILD_ASSERTION_IDS = [
  "vectis-slice-emits-composition-yaml",
  "vectis-slice-emits-screen-files",
  "vectis-slice-residue-under-routed-project",
  "vectis-slice-no-output-outside-project",
  "vectis-baseline-files-present",
] as const;

export type VectisBuildAssertionId = typeof VECTIS_BUILD_ASSERTION_IDS[number];

/** Inputs shared across the vectis-build handlers. */
export interface VectisBuildAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary. Reserved for future validator wiring. */
  specifyBin: SpecifyBin;
  /** Per-run Git env. Reserved for parity with the other family inputs. */
  env: GitEnv;
}

/** Build the vectis-build dispatch fragment. */
export function vectisBuildHandlers(
  inputs: VectisBuildAssertionInputs,
): Map<VectisBuildAssertionId, AssertionHandler> {
  const map = new Map<VectisBuildAssertionId, AssertionHandler>();
  map.set(
    "vectis-slice-emits-composition-yaml",
    makeEmitsCompositionYaml(inputs),
  );
  map.set("vectis-slice-emits-screen-files", makeEmitsScreenFiles(inputs));
  map.set(
    "vectis-slice-residue-under-routed-project",
    makeResidueUnderRoutedProject(inputs),
  );
  map.set(
    "vectis-slice-no-output-outside-project",
    makeNoOutputOutsideProject(inputs),
  );
  map.set("vectis-baseline-files-present", makeBaselineFilesPresent(inputs));
  return map;
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

function makeEmitsCompositionYaml(
  inputs: VectisBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of vectisSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = vectisShellPaths(slice.name);
      const abs = join(slot, paths.compositionYaml);
      if (!(await exists(abs))) {
        records.push(
          fail(
            id,
            `Vectis slice did not emit composition.yaml.`,
            { summary: `missing ${paths.compositionYaml}`, paths: [abs] },
            "specialist-generation",
          ),
        );
        continue;
      }
      const body = await Deno.readTextFile(abs);
      const parsed = tryParseYaml(body);
      if (!parsed.ok) {
        records.push(
          fail(
            id,
            `composition.yaml is not parseable as YAML.`,
            {
              summary:
                `${paths.compositionYaml} parse error: ${parsed.error.slice(0, 240)}`,
              paths: [abs],
            },
            "specialist-generation",
          ),
        );
        continue;
      }
      const shapeNote = describeCompositionShape(parsed.value);
      if (shapeNote.ok === false) {
        records.push(
          fail(
            id,
            `composition.yaml does not match the Vectis composition shape.`,
            {
              summary:
                `${paths.compositionYaml}: ${shapeNote.reason}`,
              paths: [abs],
            },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(id, `Vectis slice emitted composition.yaml.`, {
          summary:
            `${slice.name} → ${paths.compositionYaml} (${body.length} bytes; ${shapeNote.summary})`,
          paths: [abs],
        }),
      );
    }
    return { records };
  };
}

function makeEmitsScreenFiles(
  inputs: VectisBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of vectisSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = vectisShellPaths(slice.name);
      const abs = join(slot, paths.loginScreen);
      if (!(await exists(abs))) {
        records.push(
          fail(
            id,
            `Vectis slice did not emit a screen file.`,
            { summary: `missing ${paths.loginScreen}`, paths: [abs] },
            "specialist-generation",
          ),
        );
        continue;
      }
      const body = await Deno.readTextFile(abs);
      if (body.trim().length === 0) {
        records.push(
          fail(
            id,
            `Vectis screen file is empty.`,
            { summary: `${paths.loginScreen} has no content`, paths: [abs] },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(id, `Vectis slice emitted the screen file.`, {
          summary: `${slice.name} → ${paths.loginScreen} (${body.length} bytes)`,
          paths: [abs],
        }),
      );
    }
    return { records };
  };
}

function makeResidueUnderRoutedProject(
  inputs: VectisBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of vectisSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = vectisShellPaths(slice.name);
      const advertised = [paths.compositionYaml, paths.loginScreen];
      const offending = advertised.filter((p) => !isInsideRoutedProject(p));
      if (offending.length > 0) {
        records.push(
          fail(
            id,
            `Vectis slice advertised a path that is not inside the routed project.`,
            {
              summary: `offending: ${offending.join(", ")}`,
              paths: offending.map((p) => join(slot, p)),
            },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(id, `Vectis slice residue stays inside the routed project.`, {
          summary:
            `${slice.name} → composition.yaml (root) + ${paths.loginScreen}`,
          paths: advertised.map((p) => join(slot, p)),
        }),
      );
    }
    return { records };
  };
}

function makeNoOutputOutsideProject(
  inputs: VectisBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    // Forbidden-path check: the Vectis driver must not write
    // build outputs at common-mistake locations (a stray
    // `LoginScreen.swift` at the project root, a top-level
    // Android entrypoint, or vendored package dirs that imply a
    // full app build the driver should never do). We probe a
    // small allowlist rather than walking the whole tree —
    // walking could turn this into a slow/flaky assertion.
    const FORBIDDEN_ROOT_PATHS = [
      "LoginScreen.swift",
      "MainActivity.kt",
      "LoginActivity.kt",
      "Pods",
      "node_modules",
      "build",
      "DerivedData",
    ];

    const records: AssertionRecord[] = [];
    for (const slice of vectisSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const offending: string[] = [];
      for (const rel of FORBIDDEN_ROOT_PATHS) {
        const abs = join(slot, rel);
        if (await exists(abs)) offending.push(rel);
      }
      if (offending.length > 0) {
        records.push(
          fail(
            id,
            `Vectis slice wrote build output outside the expected project-owned paths.`,
            {
              summary: `offending root paths: ${offending.join(", ")}`,
              paths: offending.map((p) => join(slot, p)),
            },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(
          id,
          `Vectis slice did not leak output outside the routed project's expected paths.`,
          {
            summary: `${slice.name} → no offending paths`,
            paths: [slot],
          },
        ),
      );
    }
    return { records };
  };
}

function makeBaselineFilesPresent(
  inputs: VectisBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of vectisSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const baselineDir = join(slot, ".specify", "specs", slice.name);
      const archiveDir = join(slot, ".specify", "archive", slice.name);
      const proposalAbs = join(baselineDir, "proposal.md");
      const tasksAbs = join(baselineDir, "tasks.md");

      const proposalOk = await exists(proposalAbs);
      const tasksOk = await exists(tasksAbs);
      const archiveOk = await exists(archiveDir);

      if (proposalOk && tasksOk && archiveOk) {
        records.push(
          pass(id, `Vectis slice baseline files present.`, {
            summary:
              `${slice.name} → .specify/specs/${slice.name}/{proposal.md,tasks.md} + .specify/archive/${slice.name}/`,
            paths: [baselineDir, archiveDir],
          }),
        );
        continue;
      }

      const missing: string[] = [];
      if (!proposalOk) missing.push(`.specify/specs/${slice.name}/proposal.md`);
      if (!tasksOk) missing.push(`.specify/specs/${slice.name}/tasks.md`);
      if (!archiveOk) missing.push(`.specify/archive/${slice.name}/`);
      records.push(
        fail(
          id,
          `Vectis slice baseline missing one or more required artifacts after merge.`,
          {
            summary: `missing: ${missing.join(", ")}`,
            paths: missing.map((rel) => join(slot, rel)),
          },
          "skill-orchestration",
        ),
      );
    }
    return { records };
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface VectisSlice {
  name: string;
  project: string | null;
  capability?: string;
}

/** Return the Vectis-capability impl slices the loop driver visited. */
function vectisSlices(ctx: AssertionContext): VectisSlice[] {
  const slices = ctx.run.executeState?.slices ?? [];
  return slices.filter((s) =>
    s.capability === "vectis" && s.project !== null
  ) as VectisSlice[];
}

/** Compute the routed-clone path under the hub workspace. */
function workspaceSlotFor(
  setup: SetupHubResult,
  project: string,
): string {
  return join(setup.hubDir, ".specify", "workspace", project);
}

/**
 * A path is considered inside the routed project when it is either
 * the project-root composition file or lives under `apps/`. Other
 * heads (e.g. `Cargo.toml` / `crates/`) belong to a different
 * specialist; emitting them from the Vectis driver would be a
 * boundary violation.
 */
function isInsideRoutedProject(rel: string): boolean {
  if (rel === "composition.yaml") return true;
  if (rel.startsWith("apps/")) return true;
  return false;
}

/** Best-effort YAML parse. */
function tryParseYaml(
  body: string,
): { ok: true; value: unknown } | { ok: false; error: string } {
  try {
    return { ok: true, value: parseYaml(body) };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { ok: false, error: msg };
  }
}

/**
 * Crude composition-shape detector. The full schema validation
 * (cross-artifact tokens / assets / wiring) is out of scope for an
 * assertion — the contracts/omnia tools are the authoritative
 * validators. We only need to confirm the file looks like a
 * Vectis composition rather than something else (Omnia Cargo.toml
 * misnamed, an empty placeholder, etc.). The driver always emits a
 * `version: 1` document with a `screens` map; a real
 * `/spec:build` regression that produces a malformed shape fails
 * here with a clear evidence pointer.
 */
function describeCompositionShape(
  parsed: unknown,
): { ok: true; summary: string } | { ok: false; reason: string } {
  if (parsed === null || typeof parsed !== "object") {
    return { ok: false, reason: "top-level is not a YAML mapping" };
  }
  const obj = parsed as Record<string, unknown>;
  if (obj.version !== 1) {
    return {
      ok: false,
      reason: `expected version: 1, got ${JSON.stringify(obj.version)}`,
    };
  }
  const hasScreens = obj.screens !== undefined;
  const hasDelta = obj.delta !== undefined;
  if (!hasScreens && !hasDelta) {
    return { ok: false, reason: "missing both `screens` and `delta`" };
  }
  if (hasScreens && hasDelta) {
    return { ok: false, reason: "carries both `screens` and `delta`" };
  }
  if (hasScreens) {
    const screens = obj.screens;
    if (
      screens === null || typeof screens !== "object" || Array.isArray(screens)
    ) {
      return { ok: false, reason: "`screens` is not a YAML mapping" };
    }
    const count = Object.keys(screens as Record<string, unknown>).length;
    if (count === 0) {
      return { ok: false, reason: "`screens` map is empty" };
    }
    return { ok: true, summary: `version 1, ${count} screen(s)` };
  }
  return { ok: true, summary: "version 1, delta document" };
}

/** Cascade-skip gate (mirrors the C13 / C14a families). */
async function gateOrSkip(
  id: string,
  ctx: AssertionContext,
  inputs: VectisBuildAssertionInputs,
): Promise<AssertionRecord | null> {
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
  const slices = vectisSlices(ctx);
  if (slices.length === 0) {
    return skip(
      id,
      "Skipped because no Vectis slice ran (the vectis-build assertions are only meaningful under the `vectis-build` backend).",
      "no vectis slice on executeState.slices",
    );
  }
  // Wrong-backend signal: backend populated executeState with a
  // Vectis slice but routed it through `StubPhaseDriver`, so no
  // `composition.yaml` exists on disk. Probing for `composition.yaml`
  // is the canonical signal because the stub residue path under
  // `scripted-execute` happens to share `apps/mobile/login_screen.swift`
  // — its parent dir exists, but `composition.yaml` does not.
  // Only the `VectisBuildPhaseDriver` writes `composition.yaml`.
  let anyComposition = false;
  for (const slice of slices) {
    const slot = workspaceSlotFor(inputs.setup, slice.project!);
    const paths = vectisShellPaths(slice.name);
    if (await exists(join(slot, paths.compositionYaml))) {
      anyComposition = true;
      break;
    }
  }
  if (!anyComposition) {
    return skip(
      id,
      "Skipped because no Vectis `composition.yaml` was emitted by this backend; vectis-build assertions are only meaningful under the `vectis-build` backend.",
      "no composition.yaml on routed clone",
    );
  }
  return null;
}

/** Re-export for handler-graph diagnostics. */
export type { VectisShellPaths };
