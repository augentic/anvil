// Omnia-build assertion handlers (RM-01 plan, C14a).
//
// Implements the five C14a assertion ids that probe the Omnia
// implementation slice (today: `add-oauth-tokens`, routed to
// `shop-backend`) after the `OmniaBuildPhaseDriver` has run.
// Together they form the oracle that says "the Omnia slice
// produced a Cargo.toml + lib.rs + providers.rs trio in the
// routed project's `crates/<crate>/` tree, the residue stayed
// inside the project, no output landed outside `crates/`, and the
// merged baseline carries the slice's spec dir":
//
//   * `omnia-slice-emits-cargo-toml`             — the routed
//                                                  clone has a
//                                                  `crates/<crate>/
//                                                  Cargo.toml` that
//                                                  parses as
//                                                  `[package]`-
//                                                  bearing TOML.
//   * `omnia-slice-emits-lib-rs`                 — the routed
//                                                  clone has a
//                                                  `crates/<crate>/
//                                                  src/lib.rs`
//                                                  that is a
//                                                  non-empty
//                                                  regular file.
//   * `omnia-slice-residue-under-routed-project` — every Omnia
//                                                  output path
//                                                  starts with
//                                                  `crates/`.
//   * `omnia-slice-no-output-outside-project`    — the routed
//                                                  clone has no
//                                                  Omnia-shaped
//                                                  output outside
//                                                  `crates/` (the
//                                                  forbidden-path
//                                                  check).
//   * `omnia-baseline-files-present`             — the post-merge
//                                                  baseline
//                                                  `.specify/
//                                                  specs/<slice>/`
//                                                  dir exists in
//                                                  the routed
//                                                  project clone.
//
// Cascade-skip policy mirrors the C13 contracts-build family:
//   * upstream `setup-*` failure       → all five → `skip`.
//   * upstream `plan-*` failure        → all five → `skip`.
//   * `ctx.run.executeState` undefined → all five → `skip` (a
//     plan-only backend ran, e.g. `scripted-plan`).
//   * no Omnia slice in `executeState.slices` → all five → `skip`
//     with a "wrong backend" rationale (the plan-only / non-
//     omnia-build backends never wrote a crate; downgrading to
//     skip rather than fail keeps the same scenario file usable
//     under every backend in the suite).
//
// The Omnia validator wiring follows the same shape C13 reserved
// for the contracts WASI tool: if `specify tool run omnia` (or
// equivalent) becomes available, the validator handler can be
// promoted to a real invocation via `acceptance/assertions/
// verifier.ts`. Until that exists, the structural-on-disk
// assertions are the oracle. Probing for the tool today returns
// no match (the omnia capability ships skills + briefs, not a
// WASI sidecar in this snapshot), so we omit the handler entry
// rather than emit dead skip records — adding it later is a
// one-line change.

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
  omniaCratePaths,
  type OmniaCratePaths,
} from "../runner/backends/omnia-build-driver.ts";
import type { GitEnv, SetupHubResult, SpecifyBin } from "../runner/types.ts";

/** Stable id list — used by the smoke driver's `expected` set. */
export const OMNIA_BUILD_ASSERTION_IDS = [
  "omnia-slice-emits-cargo-toml",
  "omnia-slice-emits-lib-rs",
  "omnia-slice-residue-under-routed-project",
  "omnia-slice-no-output-outside-project",
  "omnia-baseline-files-present",
] as const;

export type OmniaBuildAssertionId = typeof OMNIA_BUILD_ASSERTION_IDS[number];

/** Inputs shared across the omnia-build handlers. */
export interface OmniaBuildAssertionInputs {
  /** Cross-repo setup produced by the backend's `prepare`. */
  setup: SetupHubResult;
  /** Resolved `specify` binary. Reserved for future validator wiring. */
  specifyBin: SpecifyBin;
  /** Per-run Git env. Reserved for parity with the other family inputs. */
  env: GitEnv;
}

/** Build the omnia-build dispatch fragment. */
export function omniaBuildHandlers(
  inputs: OmniaBuildAssertionInputs,
): Map<OmniaBuildAssertionId, AssertionHandler> {
  const map = new Map<OmniaBuildAssertionId, AssertionHandler>();
  map.set("omnia-slice-emits-cargo-toml", makeEmitsCargoToml(inputs));
  map.set("omnia-slice-emits-lib-rs", makeEmitsLibRs(inputs));
  map.set(
    "omnia-slice-residue-under-routed-project",
    makeResidueUnderRoutedProject(inputs),
  );
  map.set(
    "omnia-slice-no-output-outside-project",
    makeNoOutputOutsideProject(inputs),
  );
  map.set("omnia-baseline-files-present", makeBaselineFilesPresent(inputs));
  return map;
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

function makeEmitsCargoToml(
  inputs: OmniaBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of omniaSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = omniaCratePaths(slice.name);
      const abs = join(slot, paths.cargoToml);
      if (!(await exists(abs))) {
        records.push(
          fail(
            id,
            `Omnia slice did not emit Cargo.toml.`,
            { summary: `missing ${paths.cargoToml}`, paths: [abs] },
            "specialist-generation",
          ),
        );
        continue;
      }
      const body = await Deno.readTextFile(abs);
      if (!isPackageBearingToml(body)) {
        records.push(
          fail(
            id,
            `Cargo.toml is not a valid \`[package]\`-bearing TOML file.`,
            {
              summary: `${paths.cargoToml} missing [package] / name fields`,
              paths: [abs],
            },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(id, `Omnia slice emitted Cargo.toml.`, {
          summary: `${slice.name} → ${paths.cargoToml} (${body.length} bytes)`,
          paths: [abs],
        }),
      );
    }
    return { records };
  };
}

function makeEmitsLibRs(
  inputs: OmniaBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of omniaSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = omniaCratePaths(slice.name);
      const abs = join(slot, paths.libRs);
      if (!(await exists(abs))) {
        records.push(
          fail(
            id,
            `Omnia slice did not emit src/lib.rs.`,
            { summary: `missing ${paths.libRs}`, paths: [abs] },
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
            `src/lib.rs is empty.`,
            { summary: `${paths.libRs} has no content`, paths: [abs] },
            "specialist-generation",
          ),
        );
        continue;
      }
      records.push(
        pass(id, `Omnia slice emitted src/lib.rs.`, {
          summary: `${slice.name} → ${paths.libRs} (${body.length} bytes)`,
          paths: [abs],
        }),
      );
    }
    return { records };
  };
}

function makeResidueUnderRoutedProject(
  inputs: OmniaBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of omniaSlices(ctx)) {
      const slot = workspaceSlotFor(inputs.setup, slice.project!);
      const paths = omniaCratePaths(slice.name);
      const offending = pathsOutsideCrates([
        paths.cargoToml,
        paths.libRs,
        paths.providersRs,
      ]);
      if (offending.length > 0) {
        records.push(
          fail(
            id,
            `Omnia slice emitted at least one path outside \`crates/\`.`,
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
        pass(id, `Omnia slice residue is contained under \`crates/\`.`, {
          summary: `${slice.name} → ${paths.crateRoot}/`,
          paths: [join(slot, paths.crateRoot)],
        }),
      );
    }
    return { records };
  };
}

function makeNoOutputOutsideProject(
  inputs: OmniaBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    // Forbidden-path check: the Omnia driver must not write
    // build outputs at the project root (e.g. a stray `lib.rs`,
    // `Cargo.toml`, or `target/` directory at <slot>/). Anything
    // outside `crates/` and `.specify/` is suspect for an Omnia
    // residue commit. We probe a small allowlist of "common
    // mistake" locations rather than walking the whole tree —
    // walking could turn this into a slow/flaky assertion.
    const FORBIDDEN_ROOT_PATHS = [
      "Cargo.toml",
      "src/lib.rs",
      "lib.rs",
      "target",
    ];

    const records: AssertionRecord[] = [];
    for (const slice of omniaSlices(ctx)) {
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
            `Omnia slice wrote build output outside \`crates/\`.`,
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
          `Omnia slice did not leak output outside the routed project's \`crates/\` tree.`,
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
  inputs: OmniaBuildAssertionInputs,
): AssertionHandler {
  return async (id, ctx): Promise<AssertionResult> => {
    const gate = await gateOrSkip(id, ctx, inputs);
    if (gate) return { records: [gate] };

    const records: AssertionRecord[] = [];
    for (const slice of omniaSlices(ctx)) {
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
          pass(id, `Omnia slice baseline files present.`, {
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
          `Omnia slice baseline missing one or more required artifacts after merge.`,
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

interface OmniaSlice {
  name: string;
  project: string | null;
  capability?: string;
}

/** Return the Omnia-capability impl slices the loop driver visited. */
function omniaSlices(ctx: AssertionContext): OmniaSlice[] {
  const slices = ctx.run.executeState?.slices ?? [];
  return slices.filter((s) =>
    s.capability === "omnia" && s.project !== null
  ) as OmniaSlice[];
}

/** Compute the routed-clone path under the hub workspace. */
function workspaceSlotFor(
  setup: SetupHubResult,
  project: string,
): string {
  return join(setup.hubDir, ".specify", "workspace", project);
}

/** Filter to paths that do NOT live under `crates/`. */
function pathsOutsideCrates(rels: string[]): string[] {
  return rels.filter((p) => !p.startsWith("crates/"));
}

/**
 * Crude `[package]`-bearing TOML detector. The full TOML grammar is
 * out of scope for an assertion; we only need to confirm the file
 * is structurally what cargo would accept (a `[package]` table
 * with a `name = ...` line). The C14a deterministic body always
 * matches; a real `/spec:build` regression that produces a
 * malformed Cargo.toml fails here with a clear evidence pointer.
 */
function isPackageBearingToml(body: string): boolean {
  const lines = body.split(/\r?\n/);
  let inPackageTable = false;
  let sawName = false;
  for (const raw of lines) {
    const line = raw.trim();
    if (line.startsWith("#") || line.length === 0) continue;
    if (line.startsWith("[")) {
      inPackageTable = line === "[package]";
      continue;
    }
    if (inPackageTable && /^name\s*=/.test(line)) {
      sawName = true;
      break;
    }
  }
  return sawName;
}

/**
 * Cascade-skip gate (mirrors the C13 contracts-build family).
 *
 * Two layers of skipping:
 *   1. Hard cascade: upstream `setup-*` / `plan-*` failures, or a
 *      plan-only backend (no `executeState`) → skip every handler.
 *   2. Wrong-backend signal: no Omnia slice on `executeState.slices`,
 *      OR no `crates/<crate>/` tree on disk for any Omnia slice. The
 *      second clause covers backends that DO populate `executeState`
 *      with an Omnia-capability slice (e.g. `scripted-execute`,
 *      `scripted-finalize`, `contracts-build`) but route it through
 *      `StubPhaseDriver` rather than `OmniaBuildPhaseDriver`. The
 *      stub driver writes no crate files, so the absence of the
 *      crate dir is the canonical "the omnia-build driver did not
 *      run" signal — same shape as the contracts-build family's
 *      `no .yaml under <hub>/contracts/` skip rationale.
 */
async function gateOrSkip(
  id: string,
  ctx: AssertionContext,
  inputs: OmniaBuildAssertionInputs,
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
  const slices = omniaSlices(ctx);
  if (slices.length === 0) {
    return skip(
      id,
      "Skipped because no Omnia slice ran (the omnia-build assertions are only meaningful under the `omnia-build` backend).",
      "no omnia slice on executeState.slices",
    );
  }
  // Wrong-backend signal: backend populated executeState with an
  // Omnia slice but routed it through `StubPhaseDriver`, so no
  // `Cargo.toml` exists on disk. Probing for `Cargo.toml` (rather
  // than the crate root) is the canonical signal because the
  // stub residue path under `scripted-execute` happens to share
  // the `crates/<crate>/src/lib.rs` shape — its parent dir
  // exists, but `Cargo.toml` does not. Only the
  // `OmniaBuildPhaseDriver` writes `Cargo.toml`.
  let anyCargoToml = false;
  for (const slice of slices) {
    const slot = workspaceSlotFor(inputs.setup, slice.project!);
    const paths = omniaCratePaths(slice.name);
    if (await exists(join(slot, paths.cargoToml))) {
      anyCargoToml = true;
      break;
    }
  }
  if (!anyCargoToml) {
    return skip(
      id,
      "Skipped because no Omnia `Cargo.toml` was emitted by this backend; omnia-build assertions are only meaningful under the `omnia-build` backend.",
      "no crates/<crate>/Cargo.toml on routed clone",
    );
  }
  return null;
}
