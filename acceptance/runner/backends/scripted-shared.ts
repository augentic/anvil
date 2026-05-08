// Shared helpers for the RM-01 cross-repo scripted backends (RM-01
// plan, C10).
//
// The C09 `ScriptedPlanBackend` and the C10 `ScriptedExecuteBackend`
// both:
//   1. resolve `specify` (skip-friendly when missing),
//   2. land the cross-repo hub via `setupHub` with the fixed RM-01
//      project descriptors (`shop-platform` hub, `shop-backend@omnia@v1`,
//      `shop-mobile@vectis@v1`),
//   3. copy the fixture brief into the hub at `docs/oauth-login.md`,
//   4. drive a fixed sequence of `specify change plan {create, add}`
//      calls so the role-based plan assertions exercise end to end,
//   5. run `specify workspace sync` + a `--format json workspace status`
//      probe so the C06 evidence inventory is populated.
//
// C10 then layers a deterministic loop driver on top of (5). Keeping
// (1)-(5) in one helper avoids drift between the two backends and lets
// `ScriptedPlanBackend` stay byte-for-byte equivalent to its C09 form.

import { copy, ensureDir, exists } from "jsr:@std/fs@1";
import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { appendLog } from "../evidence.ts";
import {
  findSpecifyBin,
  runSpecify,
  runSpecifyJson,
  SpecifyCommandError,
} from "../specify-cli.ts";
import type { SpecifyBin } from "../specify-cli.ts";
import { setupHub } from "../hub.ts";
import { getWorkspaceStatus, runWorkspaceSync } from "../workspace-sync.ts";
import type {
  RunContext,
  SetupHubResult,
} from "../types.ts";

const REPO_ROOT = resolve(
  dirname(fromFileUrl(import.meta.url)),
  "..",
  "..",
  "..",
);

// --- Fixed cross-repo descriptors for the RM-01 suite ---------------

/** Stable hub name for the RM-01 cross-repo suite. */
export const HUB_NAME = "shop-platform";

/**
 * Project descriptors driven into `setupHub` and the registry. Keep
 * descriptions identical to the C07 setup smoke and the layered
 * `registry.yaml.skeleton.md` so the four `setup-*` invariants pass
 * without re-deriving wording.
 */
export const PROJECT_DESCRIPTORS = [
  {
    name: "shop-backend",
    capability: "omnia@v1",
    description:
      "User registration, account management, OAuth provider integration, token storage, and the authoritative HTTP API.",
  },
  {
    name: "shop-mobile",
    capability: "vectis@v1",
    description:
      "iOS and Android mobile clients with login screens, OAuth redirect handling, and token refresh flows.",
  },
] as const;

/**
 * Hard-coded slice / change names matching the Layer 0 `cross_repo.rs`
 * test. Role-based assertions accept any name as long as roles,
 * dependencies, and routing hold; we pick the substrate-test names so a
 * maintainer reading both layers sees the same vocabulary.
 */
export const CHANGE_NAME = "oauth-login";
export const SLICE_CONTRACT = "oauth-login-contract";
export const SLICE_BACKEND = "add-oauth-tokens";
export const SLICE_MOBILE = "add-oauth-screens";

/**
 * Per-slice residue path policy for the RM-01 cross-repo suite
 * (single source — C12 amendment lifted this out of `scripted-execute.ts`
 * and `scripted-finalize.ts` so all backends agree).
 *
 * Mirrors the Layer 0 substrate test
 * (`specify-cli/tests/cross_repo.rs::main`):
 *
 *   add-oauth-tokens   → crates/oauth_tokens/src/lib.rs   (Omnia-shape)
 *   add-oauth-screens  → apps/mobile/login_screen.swift   (Vectis-shape)
 *
 * The `agent` backend (C12) consumes this same map so operator-results
 * JSON does not have to redeclare residue placement.
 */
export const RESIDUE_PATHS: Record<string, string> = {
  [SLICE_BACKEND]: "crates/oauth_tokens/src/lib.rs",
  [SLICE_MOBILE]: "apps/mobile/login_screen.swift",
};

/** Default brief copy path, relative to the hub. */
export const HUB_BRIEF_PATH = "docs/oauth-login.md";

/** Where the C06 fixture brief lives in the repo. */
export const FIXTURE_BRIEF_REPO_PATH = join(
  REPO_ROOT,
  "acceptance",
  "suites",
  "rm01-cross-repo",
  "inputs",
  "docs",
  "oauth-login.md",
);

// --- Shared evidence shape ------------------------------------------

/**
 * Ordered record of a single `specify` invocation. Both backends
 * persist the list to evidence so a maintainer can correlate failures
 * with the exact CLI step that triggered them.
 */
export interface ScriptedAction {
  step: number;
  args: string[];
  cwd: string;
  exitCode: number;
}

// --- Setup phase shared helper --------------------------------------

/**
 * Outcome of `prepareScriptedHub`. The `setup` field is also written
 * back onto `ctx.setup` so the runner-owned assertions stage can read
 * hub state without a per-suite shim (C09 amendment §"Promote
 * `RunContext.setup?: SetupHubResult`").
 */
export interface ScriptedHubState {
  setup: SetupHubResult;
  bin: SpecifyBin;
  briefSourcePath: string | null;
  briefHubPath: string | null;
}

/**
 * Resolve `specify`, run `setupHub`, copy the fixture brief into the
 * hub, and stash the result on `ctx`. Throws when no `specify` binary
 * resolves; the caller is expected to translate that into a
 * `runner-setup` failure (the smoke driver wraps it as an exit-0 skip
 * when the binary is intentionally absent).
 */
export async function prepareScriptedHub(
  ctx: RunContext,
): Promise<ScriptedHubState> {
  const bin = await findSpecifyBin();
  if (!bin) {
    throw new Error(
      "scripted backend: no `specify` binary on PATH (and SPECIFY_BIN unset). " +
        "Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run. " +
        "The cross-repo smoke drivers wrap this with an exit-0 skip when the dev tool is absent.",
    );
  }

  await appendLog(
    ctx.paths.stdoutLog,
    `[scripted-shared] specify: ${bin.path} (${bin.version ?? "unknown version"})\n`,
  );

  const setup = await setupHub({
    tempDir: ctx.paths.workspace,
    hubName: HUB_NAME,
    specifyBin: bin,
    capture: { stdoutLog: ctx.paths.stdoutLog, stderrLog: ctx.paths.stderrLog },
    projects: PROJECT_DESCRIPTORS.map((p) => ({ ...p })),
  });

  let briefSourcePath: string | null = null;
  let briefHubPath: string | null = null;
  if (await exists(FIXTURE_BRIEF_REPO_PATH)) {
    const briefDst = join(setup.hubDir, HUB_BRIEF_PATH);
    await ensureDir(dirname(briefDst));
    await copy(FIXTURE_BRIEF_REPO_PATH, briefDst, { overwrite: true });
    briefSourcePath = FIXTURE_BRIEF_REPO_PATH;
    briefHubPath = briefDst;
  }

  // Promote cross-repo state onto the RunContext for the assertion stage.
  (ctx as { setup?: SetupHubResult }).setup = setup;
  (ctx as { specifyBin?: SpecifyBin }).specifyBin = bin;

  return { setup, bin, briefSourcePath, briefHubPath };
}

// --- Plan creation phase shared helper ------------------------------

/**
 * Drive the deterministic plan-creation sequence:
 *   change create
 *   change plan create
 *   change plan add <contract> --schema contracts@v1
 *   change plan add <backend>  --project shop-backend  --depends-on <contract>
 *   change plan add <mobile>   --project shop-mobile   --depends-on <contract>
 *
 * Returns the appended actions on success. Returns `null` on the first
 * non-zero exit and stashes the partial action list — callers should
 * surface this as a `cli-substrate` failure.
 */
export async function runPlanCreationSequence(opts: {
  bin: SpecifyBin;
  setup: SetupHubResult;
  actions: ScriptedAction[];
}): Promise<{ ok: true } | { ok: false; failingArgs: string[]; exitCode: number }> {
  const { bin, setup, actions } = opts;

  const sequence: string[][] = [
    ["change", "create", CHANGE_NAME],
    ["change", "plan", "create", CHANGE_NAME],
    [
      "change",
      "plan",
      "add",
      SLICE_CONTRACT,
      "--schema",
      "contracts@v1",
      "--description",
      "Author the shared OAuth login HTTP contract.",
    ],
    [
      "change",
      "plan",
      "add",
      SLICE_BACKEND,
      "--project",
      "shop-backend",
      "--depends-on",
      SLICE_CONTRACT,
      "--description",
      "Implement OAuth provider token persistence and refresh endpoints.",
    ],
    [
      "change",
      "plan",
      "add",
      SLICE_MOBILE,
      "--project",
      "shop-mobile",
      "--depends-on",
      SLICE_CONTRACT,
      "--description",
      "Implement login UI and OAuth redirect handling.",
    ],
  ];

  for (const args of sequence) {
    try {
      const run = await runSpecify({
        bin,
        cwd: setup.hubDir,
        args,
        env: setup.env,
      });
      actions.push({
        step: actions.length + 1,
        args,
        cwd: setup.hubDir,
        exitCode: run.exitCode,
      });
    } catch (e) {
      if (e instanceof SpecifyCommandError) {
        actions.push({
          step: actions.length + 1,
          args,
          cwd: setup.hubDir,
          exitCode: e.run.exitCode,
        });
        return { ok: false, failingArgs: args, exitCode: e.run.exitCode };
      }
      throw e;
    }
  }
  return { ok: true };
}

/**
 * Best-effort: run `specify workspace sync` and capture the JSON status
 * probe. Errors are appended to stderr.log but do not fail the caller —
 * the role-based plan assertions don't gate on workspace status, but
 * the C06 evidence inventory expects the slot dirs to exist.
 *
 * Returns the parsed status JSON when the probe succeeded, otherwise
 * `undefined`.
 */
export async function syncAndProbeWorkspace(opts: {
  ctx: RunContext;
  bin: SpecifyBin;
  setup: SetupHubResult;
  actions: ScriptedAction[];
}): Promise<unknown | undefined> {
  const { ctx, bin, setup, actions } = opts;
  let workspaceStatusJson: unknown | undefined;
  try {
    await runWorkspaceSync({ bin, hubDir: setup.hubDir, env: setup.env });
    const { status } = await getWorkspaceStatus({
      bin,
      hubDir: setup.hubDir,
      env: setup.env,
    });
    workspaceStatusJson = status;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    await appendLog(
      ctx.paths.stderrLog,
      `[scripted-shared] workspace sync/status non-fatal warning: ${msg}\n`,
    );
  }

  // One JSON status probe so the next-eligible value lands in evidence.
  try {
    const { run } = await runSpecifyJson({
      bin,
      cwd: setup.hubDir,
      args: ["change", "plan", "status"],
      env: setup.env,
    });
    actions.push({
      step: actions.length + 1,
      args: ["--format", "json", "change", "plan", "status"],
      cwd: setup.hubDir,
      exitCode: run.exitCode,
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    await appendLog(
      ctx.paths.stderrLog,
      `[scripted-shared] change plan status probe failed: ${msg}\n`,
    );
  }

  return workspaceStatusJson;
}

// --- Misc -----------------------------------------------------------

/** Read a file, returning `undefined` if it does not exist. */
export async function readIfExists(path: string): Promise<string | undefined> {
  try {
    return await Deno.readTextFile(path);
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) return undefined;
    throw e;
  }
}
