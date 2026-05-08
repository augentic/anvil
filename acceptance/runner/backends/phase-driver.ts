// PhaseDriver interface and the deterministic StubPhaseDriver
// implementation (RM-01 plan, C12).
//
// Background: the C10 / C11 scripted backends drive the RM-01
// happy-path loop by calling `StubBackend.driveSlice(...)` once per
// plan entry. C12 lifts that per-slice phase outcome producer behind
// a small interface so an `AgentPhaseDriver` (real `/spec:define`
// invocation, operator-manual or Cursor-SDK driven) can plug into the
// same loop without subclassing the stub.
//
// The interface is intentionally narrow: the driver only sees the
// per-slice inputs the loop driver already collected (resolved
// `specify` binary, hub dir, slice name, routed project, residue
// path). It returns the audit records the runner persists into
// evidence. CLI authority stays with `specify` — the driver shells
// out to lifecycle verbs and writes only the artifact bodies the
// agent (or the stub) would otherwise produce.
//
// Two implementations ship today:
//   * `StubPhaseDriver`  — moved verbatim from C10's
//     `StubBackend.driveSlice`. Writes "STUB:" bodies and the per-slice
//     baseline / residue commits the Layer-0 substrate test pins.
//     `StubBackend.driveSlice` becomes a thin wrapper over this driver
//     so the C08 single-slice path keeps working unchanged.
//   * `AgentPhaseDriver` — landed in `agent-phase-driver.ts`. Drives
//     real `/spec:define <slice>` per slice via an operator-manual
//     resume path or pre-collected `--operator-results` JSON.
//
// Both implementations share `driveSliceWithBodies` below: the shared
// lifecycle code (CLI transitions, baseline + residue commit pair,
// archive bookkeeping) lives here so the only thing a driver supplies
// is a `DefineBodyFactory` — the per-slice artifact bodies.

import { ensureDir } from "jsr:@std/fs@1";
import { dirname, join } from "jsr:@std/path@1";

import { runGit } from "../git.ts";
import { runSpecify } from "../specify-cli.ts";
import type { GitEnv } from "../git.ts";
import type { SpecifyBin } from "../specify-cli.ts";

import type { StubAction } from "./stub.ts";

/**
 * Per-slice driver shared by the scripted-execute, scripted-finalize,
 * and agent backends. The runner-side loop driver picks the next
 * eligible plan entry, prepares the routed clone, and delegates the
 * per-entry phase outcomes to one `PhaseDriver.driveSlice` call.
 *
 * Implementations:
 *   * `StubPhaseDriver`   — deterministic stub bodies + commit pair.
 *   * `AgentPhaseDriver`  — real `/spec:define` outputs, operator-driven.
 *
 * Both return the same `DriveSliceResult` shape so the loop driver
 * does not branch on backend type.
 */
export interface PhaseDriver {
  /** Stable identifier (e.g. `stub`, `agent`). Surfaced in evidence. */
  readonly name: string;

  /**
   * Drive a single slice through define → build → merge phases.
   * Returns the artifact actions taken so the runner can record
   * evidence.
   */
  driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult>;
}

/**
 * Inputs every phase driver consumes. Same shape across stub /
 * agent so the loop driver does not have to care which backend ran.
 */
export interface DriveSliceOpts {
  /** Resolved `specify` binary (e.g. from `findSpecifyBin`). */
  bin: SpecifyBin;
  /** Per-run Git env produced by `setupHub`. */
  env: GitEnv;
  /** Hub directory; cwd for every `specify change plan` invocation. */
  hubDir: string;
  /** Plan entry being driven. */
  sliceName: string;
  /** Routed project name, or `null` for the projectless contract slice. */
  project: string | null;
  /**
   * Workspace clone path (`<hubDir>/.specify/workspace/<project>`).
   * Required when `project != null`.
   */
  workspaceProjectDir?: string;
  /** Umbrella change name (used for the `specify/<change>` branch). */
  changeName: string;
  /**
   * Workspace-relative residue path inside the routed project clone.
   * Required when `project != null`.
   */
  residuePath?: string;
  /** Optional residue file body; defaults to a `STUB:` marker. */
  residueContent?: string;
  /**
   * Optional capability hint (`contracts`, `omnia`, `vectis`, …) the
   * `slice-has-design-when-required` assertion handler reads. Drivers
   * MAY use it to decide whether to author a `design.md` body. The
   * `StubPhaseDriver` always writes `design.md` defensively because
   * the assertion handler is the authoritative gate, but the
   * `AgentPhaseDriver` must respect the per-capability brief.
   */
  capabilityName?: string;
}

/** Outcome of a single `driveSlice` call. */
export interface DriveSliceResult {
  /** Per-action audit records appended to the backend's action log. */
  actions: StubAction[];
  /** Branch the routed clone was prepared on (when applicable). */
  preparedBranch: string | null;
}

/**
 * Per-slice define-stage artifact bodies. The shared `driveSliceWithBodies`
 * helper writes these into `.specify/specs/<slice>/` and (where
 * applicable) `.specify/archive/<slice>/` so the C12 define-* asserts
 * can observe them.
 *
 * The `null` design body opts the slice out of writing `design.md`
 * altogether — used by drivers that respect the per-capability
 * `pipeline.define[*].id` policy (contracts has no `design` brief).
 */
export interface DefineBodies {
  proposal: string;
  spec: string;
  tasks: string;
  /** `null` skips writing `design.md` entirely. */
  design: string | null;
  /** Optional residue body; falls back to `opts.residueContent` then to a `STUB:` marker. */
  residue?: string;
}

/**
 * Body factory the driver calls once per slice. Receives the slice
 * opts so the factory can vary bodies by slice / project / capability.
 */
export type DefineBodyFactory = (opts: DriveSliceOpts) => DefineBodies;

/**
 * Capability → requires-design map. Mirrors `capabilities/<name>/
 * capability.yaml` `pipeline.define[*].id` lists:
 *   * contracts → no `design` brief
 *   * omnia / vectis → `design` brief present
 * The C12 `slice-has-design-when-required` handler reads the same
 * map, so adding a capability here also opts the suite into the
 * design assertion for that capability.
 */
export const CAPABILITY_REQUIRES_DESIGN: Record<string, boolean> = {
  contracts: false,
  omnia: true,
  vectis: true,
};

/**
 * Look up whether a capability requires `design.md`. Unknown
 * capabilities default to `true` so the driver writes design.md
 * defensively (the assertion handler is the authoritative gate).
 */
export function capabilityRequiresDesign(name: string | undefined): boolean {
  if (!name) return true;
  const lookup = CAPABILITY_REQUIRES_DESIGN[name];
  return lookup === undefined ? true : lookup;
}

/**
 * Deterministic stub driver. Writes minimal but well-formed
 * define-stage artifacts (`proposal.md`, `spec.md`, `tasks.md`, and
 * `design.md` when the slice's capability requires it), then runs the
 * baseline / residue commit pair the Layer-0 substrate test
 * (`specify-cli/tests/cross_repo.rs::replay_project_slice`) pins.
 *
 * The implementation is the per-slice loop driver code that lived on
 * `StubBackend.driveSlice` through C10 and C11. C12 lifted it here so
 * `ScriptedExecuteBackend` and `ScriptedFinalizeBackend` accept any
 * `PhaseDriver` implementation without subclassing the stub.
 *
 * For implementation slices we deliberately seed the proposal and
 * design bodies with a `references baseline contracts/` marker so the
 * `implementation-slice-reads-baseline-contract` assertion passes
 * against stub-quality artifacts. Real `/spec:define` output should
 * carry the same reference (the load-bearing contract-first invariant)
 * so the assertion stays meaningful when the agent backend takes over.
 */
export class StubPhaseDriver implements PhaseDriver {
  readonly name = "stub" as const;

  driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult> {
    return driveSliceWithBodies(opts, stubBodyFactory);
  }
}

/**
 * Shared per-slice lifecycle. Both `StubPhaseDriver` and
 * `AgentPhaseDriver` route through this helper so the per-slice CLI
 * sequence, commit shape, and evidence shape stay byte-for-byte
 * identical across drivers; the only thing that varies is the
 * artifact bodies the `factory` returns.
 *
 * Routed slice (project != null):
 *   1. `specify --format json workspace prepare-branch <project> --change <change>`
 *   2. `specify change plan transition <slice> in-progress`
 *   3. Write `<slot>/.specify/specs/<slice>/{proposal.md, spec.md,
 *      tasks.md, design.md?}` and `<slot>/.specify/archive/<slice>/
 *      {proposal.md, tasks.md, design.md?}` from `factory(opts)`.
 *   4. git add .specify/specs .specify/archive
 *   5. git commit -m "specify: merge <slice>"          (baseline)
 *   6. Write residue file (capability-specific path + body).
 *   7. git add <residue-path>
 *   8. git commit -m "specify: residue <slice>"        (residue)
 *   9. `specify change plan transition <slice> done`
 *
 * Contract slice (project == null):
 *   1. `specify change plan transition <slice> in-progress`
 *   2. Write `<hub>/.specify/specs/<slice>/{proposal.md, spec.md,
 *      tasks.md, design.md?}` and `<hub>/.specify/archive/<slice>/
 *      {proposal.md, tasks.md, design.md?}`.
 *   3. `specify change plan transition <slice> done`
 *
 * CLI authority: every transition goes through `specify change plan
 * transition`. Only spec/archive bodies and the residue file are
 * written directly — they are agent-authored content, not lifecycle
 * metadata.
 */
export async function driveSliceWithBodies(
  opts: DriveSliceOpts,
  factory: DefineBodyFactory,
): Promise<DriveSliceResult> {
  const recorded: StubAction[] = [];
  const record = (a: Omit<StubAction, "ts">) => {
    recorded.push({ ts: new Date().toISOString(), ...a });
  };

  const bodies = factory(opts);

  if (opts.project !== null) {
    if (!opts.workspaceProjectDir) {
      throw new Error(
        `driveSliceWithBodies: routed slice '${opts.sliceName}' (project=${opts.project}) ` +
          `requires workspaceProjectDir.`,
      );
    }
    if (!opts.residuePath) {
      throw new Error(
        `driveSliceWithBodies: routed slice '${opts.sliceName}' requires residuePath ` +
          `(non-empty residue is the load-bearing assertion in C10).`,
      );
    }

    // 1. Prepare the workspace clone on `specify/<change>`.
    const prep = await runSpecify({
      bin: opts.bin,
      cwd: opts.hubDir,
      args: [
        "--format",
        "json",
        "workspace",
        "prepare-branch",
        opts.project,
        "--change",
        opts.changeName,
      ],
      env: opts.env,
    });
    record({
      phase: "setup",
      slice: opts.sliceName,
      action: "specify-workspace-prepare-branch",
      command: prep.args,
      artifacts: [],
      exitCode: prep.exitCode,
    });

    // 2. Plan-entry transition: pending → in-progress.
    const t1 = await runSpecify({
      bin: opts.bin,
      cwd: opts.hubDir,
      args: ["change", "plan", "transition", opts.sliceName, "in-progress"],
      env: opts.env,
    });
    record({
      phase: "define",
      slice: opts.sliceName,
      action: "specify-change-plan-transition",
      command: t1.args,
      artifacts: [],
      exitCode: t1.exitCode,
    });

    // 3. Write define-stage + baseline-merge artifacts inside the
    //    workspace clone. We always materialise proposal + spec +
    //    tasks; design.md goes in too when the bodies factory returns
    //    a non-null design body.
    const slot = opts.workspaceProjectDir;
    const specDir = join(slot, ".specify", "specs", opts.sliceName);
    const archiveDir = join(slot, ".specify", "archive", opts.sliceName);
    const writtenArtifacts = await writeDefineArtifacts(
      specDir,
      archiveDir,
      opts.sliceName,
      bodies,
    );
    record({
      phase: "define",
      slice: opts.sliceName,
      action: "phase-driver-define-bodies",
      artifacts: writtenArtifacts,
    });

    // 4-5. Baseline merge commit.
    await runGit(slot, ["add", ".specify/specs", ".specify/archive"], opts.env);
    await runGit(
      slot,
      ["commit", "--no-gpg-sign", "-m", `specify: merge ${opts.sliceName}`],
      opts.env,
    );
    record({
      phase: "merge",
      slice: opts.sliceName,
      action: "git-commit-baseline-merge",
      command: ["git", "commit", "-m", `specify: merge ${opts.sliceName}`],
      artifacts: [
        `.specify/specs/${opts.sliceName}/`,
        `.specify/archive/${opts.sliceName}/`,
      ],
    });

    // 6-8. Residue commit (touches only paths outside `.specify/`).
    const residueAbs = join(slot, opts.residuePath);
    await ensureDir(dirname(residueAbs));
    const residueBody = bodies.residue ?? opts.residueContent ??
      `// STUB: residue for ${opts.sliceName}\n` +
        `// Generated by the C12 phase driver.\n`;
    await Deno.writeTextFile(residueAbs, residueBody);
    await runGit(slot, ["add", opts.residuePath], opts.env);
    await runGit(
      slot,
      ["commit", "--no-gpg-sign", "-m", `specify: residue ${opts.sliceName}`],
      opts.env,
    );
    record({
      phase: "merge",
      slice: opts.sliceName,
      action: "git-commit-residue",
      command: ["git", "commit", "-m", `specify: residue ${opts.sliceName}`],
      artifacts: [opts.residuePath],
    });

    // 9. Plan-entry transition: in-progress → done.
    const t2 = await runSpecify({
      bin: opts.bin,
      cwd: opts.hubDir,
      args: ["change", "plan", "transition", opts.sliceName, "done"],
      env: opts.env,
    });
    record({
      phase: "merge",
      slice: opts.sliceName,
      action: "specify-change-plan-transition",
      command: t2.args,
      artifacts: [],
      exitCode: t2.exitCode,
    });
    return { actions: recorded, preparedBranch: `specify/${opts.changeName}` };
  }

  // --- Contract (projectless) slice path -------------------------

  const t1 = await runSpecify({
    bin: opts.bin,
    cwd: opts.hubDir,
    args: ["change", "plan", "transition", opts.sliceName, "in-progress"],
    env: opts.env,
  });
  record({
    phase: "define",
    slice: opts.sliceName,
    action: "specify-change-plan-transition",
    command: t1.args,
    artifacts: [],
    exitCode: t1.exitCode,
  });

  const specDir = join(opts.hubDir, ".specify", "specs", opts.sliceName);
  const archiveDir = join(opts.hubDir, ".specify", "archive", opts.sliceName);
  const writtenArtifacts = await writeDefineArtifacts(
    specDir,
    archiveDir,
    opts.sliceName,
    bodies,
  );
  record({
    phase: "define",
    slice: opts.sliceName,
    action: "phase-driver-define-bodies",
    artifacts: writtenArtifacts,
  });

  const t2 = await runSpecify({
    bin: opts.bin,
    cwd: opts.hubDir,
    args: ["change", "plan", "transition", opts.sliceName, "done"],
    env: opts.env,
  });
  record({
    phase: "merge",
    slice: opts.sliceName,
    action: "specify-change-plan-transition",
    command: t2.args,
    artifacts: [],
    exitCode: t2.exitCode,
  });
  return { actions: recorded, preparedBranch: null };
}

/**
 * Write the per-slice define-stage artifact set:
 *
 *   `<specDir>/proposal.md`
 *   `<specDir>/spec.md`
 *   `<specDir>/tasks.md`
 *   `<specDir>/design.md`        (skipped when `bodies.design === null`)
 *   `<archiveDir>/proposal.md`
 *   `<archiveDir>/tasks.md`
 *   `<archiveDir>/design.md`     (skipped when `bodies.design === null`)
 *
 * Returns workspace-relative paths suitable for the action log
 * `artifacts:` field. The caller decides which directory tree those
 * paths live under (hub for the contract slice, routed clone for
 * implementation slices).
 *
 * Both the `specs/` and `archive/` copies are written so:
 *   * the C12 `slice-has-*` assertions can observe artifacts at either
 *     location (real `/spec:define` writes to `specs/`; the merge step
 *     mirrors them into `archive/`),
 *   * the C10 baseline-merge commit's `git add .specify/specs
 *     .specify/archive` step picks both up in the same commit.
 */
async function writeDefineArtifacts(
  specDir: string,
  archiveDir: string,
  sliceName: string,
  bodies: DefineBodies,
): Promise<string[]> {
  await ensureDir(specDir);
  await ensureDir(archiveDir);

  const written: string[] = [];

  await Deno.writeTextFile(join(specDir, "proposal.md"), bodies.proposal);
  written.push(`.specify/specs/${sliceName}/proposal.md`);

  await Deno.writeTextFile(join(specDir, "spec.md"), bodies.spec);
  written.push(`.specify/specs/${sliceName}/spec.md`);

  await Deno.writeTextFile(join(specDir, "tasks.md"), bodies.tasks);
  written.push(`.specify/specs/${sliceName}/tasks.md`);

  if (bodies.design !== null) {
    await Deno.writeTextFile(join(specDir, "design.md"), bodies.design);
    written.push(`.specify/specs/${sliceName}/design.md`);
  }

  await Deno.writeTextFile(join(archiveDir, "proposal.md"), bodies.proposal);
  written.push(`.specify/archive/${sliceName}/proposal.md`);

  await Deno.writeTextFile(join(archiveDir, "tasks.md"), bodies.tasks);
  written.push(`.specify/archive/${sliceName}/tasks.md`);

  if (bodies.design !== null) {
    await Deno.writeTextFile(join(archiveDir, "design.md"), bodies.design);
    written.push(`.specify/archive/${sliceName}/design.md`);
  }

  return written;
}

/**
 * Body factory the `StubPhaseDriver` uses. Writes deterministic
 * `STUB:` bodies that satisfy the C12 define-* asserts without ever
 * invoking a real agent:
 *   * `proposal.md` / `spec.md` / `tasks.md` always written;
 *   * `design.md` written when the slice's capability requires it
 *     (omnia / vectis); skipped for `contracts`.
 *   * implementation slices (`project != null`) include a baseline
 *     contract reference so `implementation-slice-reads-baseline-contract`
 *     passes against stub-quality artifacts. Real `/spec:define`
 *     output should carry the same reference (the load-bearing
 *     contract-first invariant).
 */
export function stubBodyFactory(opts: DriveSliceOpts): DefineBodies {
  const isImplementation = opts.project !== null;
  const requiresDesign = capabilityRequiresDesign(opts.capabilityName);

  const baselineReference = isImplementation
    ? `\nReferences baseline \`contracts/oauth-login.yaml\` (the merged contract slice). ` +
      `This implementation slice does NOT author new contract YAML inline; ` +
      `it depends-on the contract slice and consumes the baseline.`
    : `\nThis is the contract slice. The contract YAML lives under \`contracts/\` ` +
      `after the build phase merges to baseline.`;

  const proposal = [
    `# STUB: proposal for ${opts.sliceName}`,
    ``,
    `> Generated by the C12 \`StubPhaseDriver\`.`,
    `> Replace with real \`/spec:define\` output before merging.`,
    baselineReference,
    ``,
  ].join("\n");

  const spec = [
    `# STUB: spec for ${opts.sliceName}`,
    ``,
    `> Generated by the C12 \`StubPhaseDriver\`.`,
    `> Replace with real \`/spec:define\` output before merging.`,
    baselineReference,
    ``,
  ].join("\n");

  const tasks = [
    `# STUB: tasks for ${opts.sliceName}`,
    ``,
    `- [ ] STUB: replaced by real \`/spec:define\` tasks`,
    ``,
  ].join("\n");

  const design = requiresDesign
    ? [
      `# STUB: design for ${opts.sliceName}`,
      ``,
      `> Generated by the C12 \`StubPhaseDriver\`. The capability brief ` +
      `(\`${opts.capabilityName ?? "<unknown>"}\`) requires a design document; ` +
      `this body is a placeholder until \`/spec:define\` writes a real one.`,
      baselineReference,
      ``,
    ].join("\n")
    : null;

  return { proposal, spec, tasks, design };
}
