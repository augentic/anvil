// Cross-repo RECORDED-LEVEL smoke driver (RM-01 plan, C15).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend recorded --recorded-trace acceptance/recorded/rm01-cross-repo/baseline.jsonl
// --allow-backend-mismatch` with the same skip-policy decisions
// C09/C10/C11 make for their smokes:
//
//   1. Skip with exit 0 when `specify` is not on PATH (and
//      `SPECIFY_BIN` is unset). Same policy as C07/C09/C10/C11.
//   2. Skip with exit 0 when the resolved `specify` predates the
//      RFC-9 surface (the trace argv set targets `init --hub`,
//      `change plan {create, add, status, next}`).
//   3. Skip with exit 0 when the checked-in baseline trace is
//      absent. The recorded backend prepares cleanly in that case
//      and the smoke driver translates the `pending-operator`
//      verdict into an exit-0 skip — useful for fresh checkouts
//      that haven't regenerated the baseline yet.
//
// Backend-mismatch flag rationale (same as C10/C11/C13/C14a):
//   The scenario file declares `backend: scripted-plan` (the
//   weakest-coverage default). The recorded backend is a strict
//   *narrow-coverage* alternative — it replays a frozen subset of
//   the scripted-execute backend's CLI actions. The mismatch flag
//   tells the runner this is intentional. The plan-only and
//   execute-only smokes continue to run with their own backends.
//
// Boundary documentation:
//   The recorded backend is *cheap regression coverage* — a
//   `cli-substrate` pin that flags exit-code drift in the recorded
//   argv set against the live binary. It does NOT prove
//   `/change:plan` / `/change:execute loop` itself does the right
//   thing; that requires the reserved `agent` backend. It does
//   NOT prove the scripted backends' loop logic is correct; it
//   only proves the `specify` argvs the scripted backends emit
//   today still exit cleanly tomorrow. See
//   `backends/README.md` §"Recorded Backend (C15)" for the rest
//   of the framing.

import { exists } from "jsr:@std/fs@1";
import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

const DEFAULT_TRACE_PATH = join(
  REPO_ROOT,
  "acceptance",
  "recorded",
  "rm01-cross-repo",
  "baseline.jsonl",
);

async function main(): Promise<number> {
  const tracePath = Deno.env.get("RECORDED_TRACE") ?? DEFAULT_TRACE_PATH;

  if (!(await exists(tracePath))) {
    console.log(
      `[c15 skip] acceptance-cross-repo-recorded-smoke: trace file not found: ${tracePath}.`,
    );
    console.log(
      `       Regenerate by running \`make acceptance-cross-repo-execute-smoke\` with --preserve,`,
    );
    console.log(
      `       then concatenate the run dir's scripted-plan-actions.jsonl + scripted-execute-loop.jsonl,`,
    );
    console.log(
      `       prefix a \`recorded-trace-header\` line, and copy to ${DEFAULT_TRACE_PATH}.`,
    );
    console.log(
      `       See acceptance/runner/backends/README.md §Regenerating A Recorded Trace.`,
    );
    return 0;
  }

  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-recorded-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    console.log(
      "       This is by design — C16 owns the install policy; C15 must not destabilise CI when the dev tool is missing.",
    );
    return 0;
  }

  console.log(
    `[c15] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );
  console.log(`[c15] trace:   ${tracePath}`);

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[c15 skip] acceptance-cross-repo-recorded-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` (pre-RFC-9 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlan(bin.path))) {
    console.log(
      `[c15 skip] acceptance-cross-repo-recorded-smoke: ${bin.path} does not expose ` +
        `\`specify change plan\` (pre-RFC-13 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlanNext(bin.path))) {
    console.log(
      `[c15 skip] acceptance-cross-repo-recorded-smoke: ${bin.path} does not expose ` +
        `\`specify change plan next\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }

  const mainTs = join(REPO_ROOT, "acceptance", "runner", "main.ts");
  const cmd = new Deno.Command(Deno.execPath(), {
    args: [
      "run",
      "--allow-read",
      "--allow-write",
      "--allow-env",
      "--allow-run",
      mainTs,
      "--suite",
      "rm01-cross-repo",
      "--backend",
      "recorded",
      "--recorded-trace",
      tracePath,
      "--allow-backend-mismatch",
    ],
    stdout: "inherit",
    stderr: "inherit",
    env: {
      ...readableEnv(),
      SPECIFY_BIN: bin.path,
    },
  });
  const { code } = await cmd.output();
  return code;
}

async function supportsHubSurface(bin: string): Promise<boolean> {
  return await helpHas(bin, ["init", "--help"], "--hub");
}

async function supportsChangePlan(bin: string): Promise<boolean> {
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["change", "plan", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    const text = new TextDecoder().decode(stdout);
    return text.includes("create") && text.includes("validate");
  } catch {
    return false;
  }
}

async function supportsChangePlanNext(bin: string): Promise<boolean> {
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["change", "plan", "next", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    const text = new TextDecoder().decode(stdout);
    return text.includes("Return the next eligible") ||
      text.includes("eligible plan entry");
  } catch {
    return false;
  }
}

async function helpHas(
  bin: string,
  args: string[],
  needle: string,
): Promise<boolean> {
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args,
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    return new TextDecoder().decode(stdout).includes(needle);
  } catch {
    return false;
  }
}

function readableEnv(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(Deno.env.toObject())) {
    out[k] = v;
  }
  return out;
}

if (import.meta.main) {
  Deno.exit(await main());
}
