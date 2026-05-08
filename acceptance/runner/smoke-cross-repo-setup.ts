// Cross-repo SETUP smoke driver (RM-01 plan, C07).
//
// Runs ONLY the C07 setup primitives end-to-end:
//   1. resolve `specify` (skip with exit 0 if not on PATH),
//   2. build a temp hub + two registered projects via `setupHub`,
//   3. exercise `specify workspace sync` and capture the JSON status,
//   4. run the four `setup-*` assertion handlers,
//   5. collect evidence (Git logs, fake `gh` PR state, registry copy)
//      into the run dir,
//   6. print a one-screen summary and exit non-zero on any failed
//      setup-* verdict.
//
// This is the C07-only smoke. C16 owns the broader
// `make acceptance-cross-repo` target that drives the whole RM-01
// suite; until then the entrypoint here is wired through
// `make acceptance-cross-repo-setup-smoke`.
//
// Skips gracefully (exit 0, "skipped" verdict) when:
//   * no `specify` binary is on `PATH` (and `SPECIFY_BIN` is unset),
//   * the resolved `specify` does not support `init --hub` /
//     `registry add` / `workspace sync` (older releases).

import { join } from "jsr:@std/path@1";

import { collectEvidence } from "./evidence-collectors.ts";
import { findSpecifyBin } from "./specify-cli.ts";
import { setupHub } from "./hub.ts";
import {
  getWorkspaceStatus,
  runWorkspaceSync,
} from "./workspace-sync.ts";
import {
  SETUP_ASSERTION_IDS,
  runSetupAssertions,
} from "../assertions/setup.ts";
import type { AssertionContext, AssertionRecord } from "../assertions/types.ts";

const HUB_NAME = "shop-platform";
const RUN_ROOT_PREFIX = "specify-acceptance-runs";
const RUN_BUCKET = "suites/rm01-cross-repo";
const RUN_ID_LABEL = "c07-setup-smoke";

interface SmokeResult {
  /** `0` for pass, `1` for assertion failure, `2` for runner setup error. */
  exitCode: number;
  message: string;
}

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-setup-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    console.log(
      "       This is by design — C16 owns the install policy; C07 must not destabilise CI when the dev tool is missing.",
    );
    return 0;
  }

  console.log(`[c07] specify: ${bin.path} (${bin.version ?? "unknown version"})`);

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[skip] acceptance-cross-repo-setup-smoke: ${bin.path} does not expose \`init --hub\` / \`registry\` / \`workspace\` (pre-RFC-9 release).`,
    );
    return 0;
  }

  const tempDir = await Deno.makeTempDir({ prefix: "specify-c07-smoke-" });
  const runDir = await makeRunDir();
  const stdoutLog = join(runDir, "stdout.log");
  const stderrLog = join(runDir, "stderr.log");
  await Deno.writeTextFile(stdoutLog, "");
  await Deno.writeTextFile(stderrLog, "");

  console.log(`[c07] temp dir:    ${tempDir}`);
  console.log(`[c07] run dir:     ${runDir}`);

  let result: SmokeResult;
  let setupHubResult: Awaited<ReturnType<typeof setupHub>> | null = null;
  let workspaceStatusJson: unknown = undefined;

  try {
    setupHubResult = await setupHub({
      tempDir,
      hubName: HUB_NAME,
      specifyBin: bin,
      capture: { stdoutLog, stderrLog },
      projects: [
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
      ],
    });

    // workspace sync is part of the setup substrate even though no
    // setup-* assertion gates on its output. We still run it so the
    // `git/<project>.log` evidence captures the workspace clones.
    try {
      await runWorkspaceSync({
        bin,
        hubDir: setupHubResult.hubDir,
        env: setupHubResult.env,
      });
      const { status } = await getWorkspaceStatus({
        bin,
        hubDir: setupHubResult.hubDir,
        env: setupHubResult.env,
      });
      workspaceStatusJson = status;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      await Deno.writeTextFile(
        stderrLog,
        `\n[c07-smoke] workspace sync/status non-fatal warning: ${msg}\n`,
        { append: true },
      );
    }

    const records = await runSetupAssertions(
      {
        hubDir: setupHubResult.hubDir,
        specifyBin: bin,
        env: setupHubResult.env,
      },
      buildAssertionContext({ runDir, hubDir: setupHubResult.hubDir, stdoutLog, stderrLog }),
    );

    await Deno.writeTextFile(
      join(runDir, "assertions.json"),
      JSON.stringify({ assertions: records }, null, 2) + "\n",
    );

    result = renderResult(records);
  } catch (e) {
    const msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    await Deno.writeTextFile(
      stderrLog,
      `\n[c07-smoke] setup error before assertions could run:\n${msg}\n`,
      { append: true },
    );
    result = { exitCode: 2, message: `runner setup error: ${msg}` };
  } finally {
    if (setupHubResult) {
      try {
        await collectEvidence({
          runDir,
          hubDir: setupHubResult.hubDir,
          projectDirs: setupHubResult.projectDirs,
          fakeGhStateDir: setupHubResult.fakeGhStateDir,
          env: setupHubResult.env,
          workspaceStatusJson,
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        await Deno.writeTextFile(
          stderrLog,
          `\n[c07-smoke] evidence collection error: ${msg}\n`,
          { append: true },
        );
      }
    }
  }

  await printSummary(runDir, result);

  // Retention: keep the run dir on failure (C07 guardrail), drop the
  // temp scaffold either way to leave the operator's filesystem tidy.
  if (result.exitCode === 0) {
    await safeRemove(runDir);
  }
  await safeRemove(tempDir);

  return result.exitCode;
}

async function supportsHubSurface(bin: string): Promise<boolean> {
  // `specify init --help` text mentions `--hub` only on the new
  // surface (RFC-9 §1D + RFC-13 §3.5). Cheap probe; avoids guessing
  // CLI versions.
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["init", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    const text = new TextDecoder().decode(stdout);
    return text.includes("--hub");
  } catch {
    return false;
  }
}

async function makeRunDir(): Promise<string> {
  const tmp = Deno.env.get("TMPDIR") ?? "/tmp";
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const rand = Math.random().toString(36).slice(2, 8);
  const runDir = join(
    tmp,
    RUN_ROOT_PREFIX,
    RUN_BUCKET,
    RUN_ID_LABEL,
    `${ts}-${rand}`,
  );
  await Deno.mkdir(runDir, { recursive: true });
  return runDir;
}

function buildAssertionContext(input: {
  runDir: string;
  hubDir: string;
  stdoutLog: string;
  stderrLog: string;
}): AssertionContext {
  // The C07 smoke drives setup outside the C04 runner skeleton, so we
  // synthesise just enough RunContext for the assertion handlers to
  // consume. C09 will rewire this through the real runner.
  return {
    workspace: input.hubDir,
    prior: [],
    run: {
      // Cast through unknown — only the fields the setup handlers use
      // are populated. C09's wiring will replace this stub with the
      // real `RunContext` from `acceptance/runner/main.ts`.
      paths: {
        runDir: input.runDir,
        workspace: input.hubDir,
        stdoutLog: input.stdoutLog,
        stderrLog: input.stderrLog,
        transcriptMd: join(input.runDir, "transcript.md"),
        toolCallsJsonl: join(input.runDir, "tool-calls.jsonl"),
        summaryMd: join(input.runDir, "summary.md"),
        scenarioMd: join(input.runDir, "scenario.md"),
        assertionsJson: join(input.runDir, "assertions.json"),
        finalTreeTxt: join(input.runDir, "final-tree.txt"),
      },
      options: { preserve: false },
      startedAt: new Date().toISOString(),
      // The smoke runs without a discovered scenario file. The
      // assertion handlers do not read this field.
      scenario: null as unknown as never,
    } as unknown as AssertionContext["run"],
  };
}

function renderResult(records: AssertionRecord[]): SmokeResult {
  const byId = new Map<string, AssertionRecord>();
  for (const r of records) byId.set(r.id, r);

  const missing = SETUP_ASSERTION_IDS.filter((id) => !byId.has(id));
  if (missing.length > 0) {
    return {
      exitCode: 2,
      message: `setup handlers did not emit records for: ${missing.join(", ")}`,
    };
  }
  const failed = records.filter((r) => r.verdict === "fail");
  if (failed.length > 0) {
    return {
      exitCode: 1,
      message: `failed: ${failed.map((r) => r.id).join(", ")}`,
    };
  }
  return { exitCode: 0, message: "all four setup-* assertions passed" };
}

async function printSummary(runDir: string, result: SmokeResult): Promise<void> {
  const assertionsPath = join(runDir, "assertions.json");
  console.log("");
  console.log("=== C07 cross-repo SETUP smoke summary ===");
  console.log(`Run directory: ${runDir}`);
  if (await exists(assertionsPath)) {
    const body = await Deno.readTextFile(assertionsPath);
    try {
      const parsed = JSON.parse(body) as { assertions: AssertionRecord[] };
      for (const r of parsed.assertions) {
        const fault = r["fault-domain"] ? ` [${r["fault-domain"]}]` : "";
        console.log(`  - ${r.id}: ${r.verdict}${fault}`);
        console.log(`      evidence: ${r.evidence}`);
      }
    } catch {
      console.log("(failed to render assertions.json — see file directly)");
    }
  } else {
    console.log("(assertions.json missing — see stderr.log)");
  }
  console.log("");
  console.log(`Result: ${result.message}`);
}

async function safeRemove(path: string): Promise<void> {
  try {
    await Deno.remove(path, { recursive: true });
  } catch (e) {
    if (!(e instanceof Deno.errors.NotFound)) {
      console.warn(`(warning) failed to remove ${path}: ${e}`);
    }
  }
}

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}

if (import.meta.main) {
  Deno.exit(await main());
}
