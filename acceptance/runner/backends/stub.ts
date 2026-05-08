// Deterministic stub backend (RM-01 plan, C08).
//
// Purpose: drive a scenario's slice lifecycle through the real `specify`
// CLI without paying live-agent generation cost. The stub is the
// deterministic *workflow* backend — distinct from C05's `fixture`
// backend, which materialises one scenario's expected-artifact set into
// the workspace for runner-plumbing smoke tests but does NOT call the
// CLI.
//
// What the stub does:
//
//   1. `prepare(ctx)` resolves `specify` (skipping with a clear notice
//      when neither `SPECIFY_BIN` nor a `specify` on PATH resolve), then
//      shells out to `specify init <name> <capability-uri>` so the
//      workspace is real `.specify/` state — never hand-edited.
//   2. `invoke(ctx)` runs `specify slice create <name>` once, then
//      walks `frontmatter.stubbed-stages` in order, driving each stage
//      through the `specify` CLI:
//        - `define`: write tiny stub `proposal.md` / `specs/main.md` /
//          `tasks.md` bodies (clearly marked `STUB:`) inside the slice
//          directory, then `specify slice transition <slice> defined`.
//        - `build`: `specify slice transition <slice> building`,
//          materialise every `expected-artifacts:` path from the
//          `stub-fixtures.build` directory (relative to the repo root),
//          then `specify slice transition <slice> complete`.
//        - `merge`: `specify slice merge run <slice>`. The CLI itself
//          performs spec-merge + archive — the stub does not hand-edit
//          `.specify/archive/`.
//   3. The list of stubbed actions is returned in
//      `BackendResult.evidence.extras.stubbed`. The runner writes each
//      record to `stub-actions.jsonl` next to `assertions.json` and
//      surfaces the count in `summary.md`.
//
// CLI-authoritative invariant: every lifecycle transition goes through
// `specify`. The stub may write `proposal.md` / `specs/*.md` /
// `tasks.md` bodies (they are agent-authored artifacts), and may copy
// scenario-declared fixture files into the workspace (they are inputs,
// not lifecycle metadata). It must never write to `.metadata.yaml`,
// `archive/`, or the slice directory tree directly.
//
// Failure semantics: every CLI non-zero exit becomes a `cli-substrate`
// fault domain with the failing command surface in the evidence.
// Missing fixtures, unresolvable capability URIs, or empty
// `stubbed-stages` become `runner-setup`.

import { copy, ensureDir, exists } from "jsr:@std/fs@1";
import { dirname, fromFileUrl, isAbsolute, join, resolve } from "jsr:@std/path@1";

import { appendLog } from "../evidence.ts";
import { findSpecifyBin, runSpecify, SpecifyCommandError } from "../specify-cli.ts";
import type { SpecifyBin } from "../specify-cli.ts";
import type { GitEnv } from "../git.ts";
import {
  StubPhaseDriver,
  type DriveSliceOpts,
  type DriveSliceResult,
} from "./phase-driver.ts";
import type {
  AssertionRecord,
  Backend,
  BackendResult,
  RunContext,
} from "../types.ts";

const REPO_ROOT = resolve(
  dirname(fromFileUrl(import.meta.url)),
  "..",
  "..",
  "..",
);

/** Per-stub-action record persisted into evidence. */
export interface StubAction {
  /** Lifecycle phase the action belongs to. */
  phase: "setup" | "define" | "build" | "merge";
  /** Slice the action targeted. */
  slice: string;
  /** Short verb the runner can render in summaries. */
  action: string;
  /** Optional `specify` argv that produced the action (excluding the bin). */
  command?: string[];
  /** Workspace-relative paths the action wrote, when known. */
  artifacts: string[];
  /** ISO timestamp the action ran. */
  ts: string;
  /** Optional CLI exit code captured for substrate-level diagnostics. */
  exitCode?: number;
}

/**
 * @deprecated re-exported from `phase-driver.ts`. Use `DriveSliceOpts`
 * from `./phase-driver.ts`. Kept here for backwards compatibility
 * with C10/C11 import sites.
 */
export type { DriveSliceOpts, DriveSliceResult } from "./phase-driver.ts";

/** Shape stored on `BackendResult.evidence.extras.stubbed`. */
export interface StubEvidence {
  scenario: string;
  slice: string;
  stubbedStages: ReadonlyArray<"define" | "build" | "merge">;
  /** True when the stub skipped (no `specify` resolvable). */
  skipped?: boolean;
  /** Skip rationale (only set when `skipped` is true). */
  reason?: string;
  actions: StubAction[];
}

export class StubBackend implements Backend {
  readonly name = "stub" as const;

  private bin: SpecifyBin | null = null;
  private slice: string = "";
  private actions: StubAction[] = [];
  private skipReason: string | null = null;
  private capabilityUri: string | null = null;
  private activePhase: StubAction["phase"] = "setup";

  async prepare(ctx: RunContext): Promise<void> {
    const { scenario, paths } = ctx;
    this.slice = stubSliceName(scenario.frontmatter.id);

    const bin = await findSpecifyBin();
    if (!bin) {
      this.skipReason =
        "no `specify` binary resolved (set SPECIFY_BIN or install `specify` on PATH); " +
        "the stub backend cannot drive lifecycle transitions without it";
      await appendLog(
        paths.stdoutLog,
        `[stub] ${this.skipReason}\n`,
      );
      return;
    }
    this.bin = bin;

    const cap = resolveCapabilityUri(scenario);
    if (!cap) {
      throw new Error(
        `stub backend cannot resolve a capability URI for scenario '${scenario.frontmatter.id}'. ` +
          `Declare 'capability: <name>@v<n>' and place the scenario under capabilities/<owner>/tests/, ` +
          `or set 'stub-capability-uri:' in the scenario frontmatter.`,
      );
    }
    this.capabilityUri = cap;

    // Initialise `.specify/` through the CLI — never hand-edit.
    const env = await stubGitEnv(ctx);
    try {
      const run = await runSpecify({
        bin,
        cwd: paths.workspace,
        args: ["init", "--name", this.slice, cap],
        env,
      });
      this.recordAction({
        phase: "setup",
        slice: this.slice,
        action: "specify-init",
        command: run.args,
        artifacts: [".specify/project.yaml"],
        exitCode: run.exitCode,
      });
    } catch (e) {
      // Surface as a runner-setup error: prepare() failures are
      // wrapped by main.ts into the runner-setup fault domain. We
      // re-throw so the caller can record the runner-setup record.
      throw e;
    }
  }

  async invoke(ctx: RunContext): Promise<BackendResult> {
    const { scenario, paths } = ctx;
    const fm = scenario.frontmatter;

    if (this.skipReason || !this.bin) {
      const evidence: StubEvidence = {
        scenario: fm.id,
        slice: this.slice,
        stubbedStages: (fm["stubbed-stages"] ?? []) as StubEvidence["stubbedStages"],
        skipped: true,
        reason: this.skipReason ?? "stub bin missing",
        actions: this.actions,
      };
      const note =
        `Stub backend skipped: ${evidence.reason}. ` +
        `This is by design — the stub-smoke target must not destabilise CI when the dev tool is missing.`;
      // `skipAssertions: true` tells the runner-owned assertions stage
      // to bypass file-existence helpers — the workspace is empty
      // because we never drove the CLI, so files-exist would fail
      // misleadingly.
      return {
        verdict: "pending-operator",
        faultDomain: null,
        notes: note,
        assertions: [],
        evidence: {
          extras: { stubbed: evidence, skipAssertions: true },
        },
      };
    }

    const stubbed = (fm["stubbed-stages"] ?? []) as ReadonlyArray<
      "define" | "build" | "merge"
    >;
    if (stubbed.length === 0) {
      return this.failResult(
        scenario.frontmatter.id,
        `scenario '${fm.id}' declares 'backend: stub' but no 'stubbed-stages:'. ` +
          `Add 'stubbed-stages: [...]' to the frontmatter.`,
        "runner-setup",
      );
    }

    // Validate each declared build/merge fixture up front so the
    // failure surface is one record rather than a partially-driven
    // workspace. `define` does not require a fixture (the stub writes
    // tiny built-in bodies).
    const fixtures = fm["stub-fixtures"] ?? {};
    if (
      stubbed.includes("build") &&
      (fm["expected-artifacts"]?.length ?? 0) > 0 &&
      !fixtures.build
    ) {
      return this.failResult(
        fm.id,
        `scenario '${fm.id}' stubs 'build' and declares ${
          fm["expected-artifacts"]!.length
        } expected-artifacts but no 'stub-fixtures.build:' directory. ` +
          `Add 'stub-fixtures: { build: <repo-relative-dir> }' so the stub backend has artifacts to materialise.`,
        "runner-setup",
      );
    }
    for (const stage of ["define", "build", "merge"] as const) {
      const dir = fixtures[stage];
      if (!dir) continue;
      const abs = resolveRepoPath(dir);
      if (!abs) {
        return this.failResult(
          fm.id,
          `'stub-fixtures.${stage}' must be a repo-relative path, not absolute: ${dir}`,
          "runner-setup",
        );
      }
      if (!(await exists(abs))) {
        return this.failResult(
          fm.id,
          `'stub-fixtures.${stage}' points at '${dir}' but the directory does not exist under the repo root.`,
          "runner-setup",
        );
      }
    }

    const env = await stubGitEnv(ctx);

    // Always create the slice. Subsequent stubbed stages drive
    // transitions, so the slice must exist no matter which subset of
    // stages is stubbed.
    try {
      const run = await runSpecify({
        bin: this.bin,
        cwd: paths.workspace,
        args: ["slice", "create", this.slice],
        env,
      });
      this.recordAction({
        phase: "setup",
        slice: this.slice,
        action: "specify-slice-create",
        command: run.args,
        artifacts: [`.specify/slices/${this.slice}/.metadata.yaml`],
        exitCode: run.exitCode,
      });
    } catch (e) {
      return this.cliFailure(e, fm.id, "setup");
    }

    try {
      for (const stage of stubbed) {
        this.activePhase = stage;
        if (stage === "define") {
          await this.runDefineStage(ctx, env, fixtures.define);
        } else if (stage === "build") {
          await this.runBuildStage(ctx, env, fixtures.build);
        } else if (stage === "merge") {
          await this.runMergeStage(ctx, env);
        }
      }
    } catch (e) {
      return this.cliFailure(e, fm.id, this.activePhase);
    }

    const evidence: StubEvidence = {
      scenario: fm.id,
      slice: this.slice,
      stubbedStages: stubbed,
      actions: this.actions,
    };

    return {
      verdict: "passed",
      faultDomain: null,
      notes:
        `Stub backend drove ${stubbed.length} stage(s) ` +
        `(${stubbed.join(", ")}) for slice '${this.slice}' across ` +
        `${this.actions.length} CLI action(s).`,
      assertions: [],
      evidence: {
        materialisedPaths: collectArtifactPaths(this.actions),
        extras: { stubbed: evidence },
      },
    };
  }

  async teardown(_ctx: RunContext): Promise<void> {
    // No external resources; the runner owns workspace cleanup so the
    // retention policy stays centralised.
  }

  // --- C10 multi-repo loop driver ---------------------------------
  //
  // `driveSlice` is the C10 entrypoint: it drives ONE plan-entry
  // through the deterministic phase outcomes the loop-driver expects,
  // mirroring `specify-cli/tests/cross_repo.rs::replay_contract_slice`
  // / `replay_project_slice`.
  //
  // **C12 amendment.** The body lives in `StubPhaseDriver` (under
  // `phase-driver.ts`); this method is a thin backwards-compat
  // wrapper for any caller that still constructs a `StubBackend`
  // directly. New backends should depend on the `PhaseDriver`
  // interface instead.
  async driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult> {
    const driver = new StubPhaseDriver();
    const result = await driver.driveSlice(opts);
    // Mirror the action log onto this StubBackend instance so any
    // legacy caller that inspected `this.actions` after `driveSlice`
    // still sees the records (the field is internal — the tests do
    // not currently rely on it, but the duplication is cheap).
    for (const a of result.actions) {
      this.actions.push({
        phase: a.phase,
        slice: a.slice,
        action: a.action,
        command: a.command,
        artifacts: a.artifacts,
        ts: a.ts,
        exitCode: a.exitCode,
      });
    }
    return result;
  }

  // --- Stage drivers -----------------------------------------------

  private async runDefineStage(
    ctx: RunContext,
    env: GitEnv,
    fixtureDir: string | undefined,
  ): Promise<void> {
    const { paths } = ctx;
    const sliceDir = join(paths.workspace, ".specify", "slices", this.slice);

    const written: string[] = [];
    if (fixtureDir) {
      const copied = await copyFixtureInto(fixtureDir, sliceDir);
      written.push(...copied.map((p) => relToWorkspace(paths.workspace, p)));
    } else {
      const wrote = await writeStubDefineBodies(sliceDir, this.slice);
      written.push(...wrote.map((p) => relToWorkspace(paths.workspace, p)));
    }

    this.recordAction({
      phase: "define",
      slice: this.slice,
      action: fixtureDir ? "define-fixture-copy" : "define-stub-bodies",
      artifacts: written,
    });

    const run = await runSpecify({
      bin: this.bin!,
      cwd: paths.workspace,
      args: ["slice", "transition", this.slice, "defined"],
      env,
    });
    this.recordAction({
      phase: "define",
      slice: this.slice,
      action: "specify-slice-transition",
      command: run.args,
      artifacts: [],
      exitCode: run.exitCode,
    });
  }

  private async runBuildStage(
    ctx: RunContext,
    env: GitEnv,
    fixtureDir: string | undefined,
  ): Promise<void> {
    const { paths } = ctx;

    const t1 = await runSpecify({
      bin: this.bin!,
      cwd: paths.workspace,
      args: ["slice", "transition", this.slice, "building"],
      env,
    });
    this.recordAction({
      phase: "build",
      slice: this.slice,
      action: "specify-slice-transition",
      command: t1.args,
      artifacts: [],
      exitCode: t1.exitCode,
    });

    if (fixtureDir) {
      const copied = await copyFixtureInto(fixtureDir, paths.workspace);
      this.recordAction({
        phase: "build",
        slice: this.slice,
        action: "build-fixture-copy",
        artifacts: copied.map((p) => relToWorkspace(paths.workspace, p)),
      });
    }

    const t2 = await runSpecify({
      bin: this.bin!,
      cwd: paths.workspace,
      args: ["slice", "transition", this.slice, "complete"],
      env,
    });
    this.recordAction({
      phase: "build",
      slice: this.slice,
      action: "specify-slice-transition",
      command: t2.args,
      artifacts: [],
      exitCode: t2.exitCode,
    });
  }

  private async runMergeStage(ctx: RunContext, env: GitEnv): Promise<void> {
    const { paths } = ctx;
    const run = await runSpecify({
      bin: this.bin!,
      cwd: paths.workspace,
      args: ["slice", "merge", "run", this.slice],
      env,
    });
    this.recordAction({
      phase: "merge",
      slice: this.slice,
      action: "specify-slice-merge-run",
      command: run.args,
      artifacts: [],
      exitCode: run.exitCode,
    });
  }

  // --- Helpers -----------------------------------------------------

  private recordAction(
    a: Omit<StubAction, "ts"> & { ts?: string },
  ): void {
    this.actions.push({ ts: new Date().toISOString(), ...a });
  }

  private failResult(
    scenarioId: string,
    msg: string,
    faultDomain: BackendResult["faultDomain"],
  ): BackendResult {
    const evidence: StubEvidence = {
      scenario: scenarioId,
      slice: this.slice,
      stubbedStages: [],
      actions: this.actions,
    };
    return {
      verdict: "failed",
      faultDomain,
      notes: msg,
      assertions: [
        {
          id: "stub-precondition",
          description: "Stub backend precondition.",
          verdict: "fail",
          evidence: msg,
          "fault-domain": faultDomain ?? "runner-setup",
        },
      ],
      evidence: { extras: { stubbed: evidence } },
    };
  }

  private cliFailure(
    e: unknown,
    scenarioId: string,
    phase: StubAction["phase"],
  ): BackendResult {
    let msg: string;
    let cmd: string[] | undefined;
    let exitCode: number | undefined;
    if (e instanceof SpecifyCommandError) {
      msg = e.message;
      cmd = e.run.args;
      exitCode = e.run.exitCode;
      // Surface the failing CLI command in evidence so the
      // cli-substrate fault is attributable to a specific phase.
      this.recordAction({
        phase,
        slice: this.slice,
        action: "cli-failure",
        command: cmd,
        artifacts: [],
        exitCode,
      });
    } else {
      msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    }
    const evidence: StubEvidence = {
      scenario: scenarioId,
      slice: this.slice,
      stubbedStages: [],
      actions: this.actions,
    };
    return {
      verdict: "failed",
      faultDomain: "cli-substrate",
      notes:
        `Stub backend hit a non-zero \`specify\` exit during ${phase}. ` +
        `Last 50 lines of stderr captured in stderr.log. Full message: ${
          truncate(msg, 800)
        }`,
      assertions: [
        {
          id: "stub-cli-substrate",
          description: `\`specify ${
            (cmd ?? []).join(" ")
          }\` returned a non-zero exit during ${phase}.`,
          verdict: "fail",
          evidence: truncate(msg, 800),
          "fault-domain": "cli-substrate",
        },
      ],
      evidence: { extras: { stubbed: evidence } },
    };
  }
}

// --- Free helpers --------------------------------------------------

/**
 * Derive a slice name from the scenario id. Slice names are kebab-case
 * and the scenario id already conforms (per `.cursor/schemas/scenario.schema.json`).
 */
export function stubSliceName(scenarioId: string): string {
  return scenarioId;
}

/**
 * Resolve a capability URI for `specify init`. For capability-owned
 * scenarios (the common case) we point at the in-repo capability
 * directory via `file://` so the init step does not depend on the
 * operator's plugin installation. Hub scenarios should use the
 * frontmatter override path (out of scope for the C08 single-slice
 * stub; tracked for C10).
 */
export function resolveCapabilityUri(scenario: {
  frontmatter: { capability?: string; owner: string };
  source: { kind: string };
}): string | null {
  // owner is the directory name under capabilities/<owner>/tests/.
  if (
    scenario.source.kind === "capability-flat" ||
    scenario.source.kind === "capability-dir"
  ) {
    const dir = join(REPO_ROOT, "capabilities", scenario.frontmatter.owner);
    return `file://${dir}`;
  }
  return null;
}

/**
 * Build the GitEnv the stub backend hands to `runSpecify`. Single-slice
 * stub scenarios do not need fake-`gh` or local bare remotes — `specify
 * init` and `slice` verbs do not invoke git over SSH. We still produce a
 * deterministic, isolated env so a leaked operator `~/.gitconfig` does
 * not bleed in.
 */
export async function stubGitEnv(ctx: RunContext): Promise<GitEnv> {
  const envDir = join(ctx.paths.workspace, ".stub-env");
  const binDir = join(envDir, "bin");
  const stateDir = join(envDir, "gh-state");
  await ensureDir(binDir);
  await ensureDir(stateDir);
  const gitConfig = join(envDir, "gitconfig");
  if (!(await exists(gitConfig))) {
    await Deno.writeTextFile(gitConfig, "");
  }
  // Placeholder fake-ssh / fake-gh that exit non-zero if invoked. The
  // single-slice stub never invokes them; if a future multi-repo stub
  // path needs a real fake, swap to `installFakeGh` from `fake-gh.ts`.
  const fakeSsh = join(binDir, "fake-ssh");
  if (!(await exists(fakeSsh))) {
    await Deno.writeTextFile(
      fakeSsh,
      "#!/bin/sh\necho 'stub: fake-ssh invoked unexpectedly' >&2\nexit 99\n",
    );
    await Deno.chmod(fakeSsh, 0o755);
  }
  return {
    gitConfigGlobal: gitConfig,
    remotesDir: envDir,
    fakeSshScript: fakeSsh,
    fakeBinDir: binDir,
    fakeGhStateDir: stateDir,
    stdoutLog: ctx.paths.stdoutLog,
    stderrLog: ctx.paths.stderrLog,
  };
}

/**
 * Write minimal but legal stub bodies for a slice's define-stage
 * artifacts. Bodies include the explicit "STUB:" marker so a reader can
 * tell the artifact is fake. Returns absolute paths the helper wrote.
 */
export async function writeStubDefineBodies(
  sliceDir: string,
  sliceName: string,
): Promise<string[]> {
  const proposalPath = join(sliceDir, "proposal.md");
  const specsDir = join(sliceDir, "specs");
  const specPath = join(specsDir, "main.md");
  const tasksPath = join(sliceDir, "tasks.md");
  await ensureDir(specsDir);
  await Deno.writeTextFile(
    proposalPath,
    [
      `# STUB: define-stage proposal for ${sliceName}`,
      ``,
      `> Generated by the C08 deterministic stub backend.`,
      `> This file is fake — it exists only so the slice has the artifacts`,
      `> a real \`/spec:define\` would have produced.`,
      ``,
      `## Intent (STUB)`,
      ``,
      `Replay-only placeholder. Do not consume this body in production runs.`,
      ``,
    ].join("\n"),
  );
  await Deno.writeTextFile(
    specPath,
    [
      `# STUB: spec for ${sliceName}`,
      ``,
      `> Generated by the C08 deterministic stub backend.`,
      `> Replace with real \`/spec:define\` output before merging.`,
      ``,
    ].join("\n"),
  );
  await Deno.writeTextFile(
    tasksPath,
    [
      `# STUB: tasks for ${sliceName}`,
      ``,
      `- [ ] STUB: replaced by real \`/spec:define\` tasks`,
      ``,
    ].join("\n"),
  );
  return [proposalPath, specPath, tasksPath];
}

/**
 * Copy every file under `srcDir` into `destDir`, preserving relative
 * structure. Returns the list of destination paths actually written.
 */
export async function copyFixtureInto(
  srcRel: string,
  destDir: string,
): Promise<string[]> {
  const src = resolveRepoPath(srcRel);
  if (!src) {
    throw new Error(`stub-fixture path must be repo-relative: ${srcRel}`);
  }
  const written: string[] = [];
  // `copy(src, dst)` from std/fs preserves directory structure when src
  // is a directory. We walk to enumerate destination paths for evidence.
  const { walk } = await import("jsr:@std/fs@1/walk");
  for await (
    const entry of walk(src, {
      includeDirs: false,
      followSymlinks: false,
    })
  ) {
    const rel = entry.path.slice(src.length + 1);
    const dst = join(destDir, rel);
    await ensureDir(dirname(dst));
    await copy(entry.path, dst, { overwrite: true });
    written.push(dst);
  }
  return written;
}

function resolveRepoPath(relPath: string): string | null {
  if (isAbsolute(relPath)) return null;
  return resolve(REPO_ROOT, relPath);
}

function relToWorkspace(workspace: string, abs: string): string {
  if (!abs.startsWith(workspace)) return abs;
  return abs.slice(workspace.length + 1);
}

function collectArtifactPaths(actions: StubAction[]): string[] {
  const seen = new Set<string>();
  for (const a of actions) {
    for (const p of a.artifacts) seen.add(p);
  }
  return [...seen].sort();
}

function truncate(s: string, n: number): string {
  if (s.length <= n) return s;
  return s.slice(0, n) + "…(truncated)";
}
