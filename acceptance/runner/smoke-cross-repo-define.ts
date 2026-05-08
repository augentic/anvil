// Cross-repo DEFINE-LEVEL smoke driver (RM-01 plan, C12).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend agent --allow-backend-mismatch
// --operator-results acceptance/suites/rm01-cross-repo/operator-results.example.json`
// with the same skip-policy decisions C09/C10/C11 make for their
// smokes:
//
//   1. Skip with exit 0 when `specify` is not on PATH (and
//      `SPECIFY_BIN` is unset). Same policy as C07/C09/C10/C11.
//   2. Skip with exit 0 when the resolved `specify` predates the
//      RFC-9 / RFC-13 surface the agent backend's inner
//      ScriptedFinalizeBackend depends on.
//   3. Skip with exit 0 when neither `--operator-results` nor
//      `--cursor-sdk` is supplied; the backend reports
//      `pending-operator` and the smoke driver translates that to
//      a `[c12 skip]` print.
//
// Boundary documentation:
//   The agent backend produces real define-stage bodies inside the
//   same composition pattern `scripted-finalize` uses. It proves the
//   define-* assertion plumbing end-to-end against operator-supplied
//   artifact bodies. It does NOT prove the real `/spec:define` skill
//   is correct on the same brief — that requires running the live
//   agent (option A: Cursor SDK driver, deferred). See
//   `backends/README.md` §Agent Backend for the full framing.
//
// Invocation:
//   make acceptance-cross-repo-define-smoke              # uses example operator-results
//   OPERATOR_RESULTS=/path/to/results.json \
//     make acceptance-cross-repo-define-smoke            # custom operator-results file

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

const DEFAULT_OPERATOR_RESULTS = join(
  REPO_ROOT,
  "acceptance",
  "suites",
  "rm01-cross-repo",
  "operator-results.example.json",
);

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-define-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    return 0;
  }

  console.log(
    `[c12] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` / \`registry\` / \`workspace\` (pre-RFC-9 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlan(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`specify change plan\` (pre-RFC-13 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlanNext(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`specify change plan next\` / \`transition\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsPrepareBranch(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`specify workspace prepare-branch\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsWorkspacePush(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`specify workspace push\` (needs RFC-3b-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsChangeFinalize(bin.path))) {
    console.log(
      `[c12 skip] acceptance-cross-repo-define-smoke: ${bin.path} does not expose ` +
        `\`specify change finalize\` (needs RFC-9 §4C-aware binary).`,
    );
    return 0;
  }

  const operatorResults = Deno.env.get("OPERATOR_RESULTS") ??
    DEFAULT_OPERATOR_RESULTS;

  if (!operatorResults) {
    console.log(
      `[c12 skip] AgentBackend requires either Cursor SDK (--cursor-sdk; ` +
        `deferred to a future amendment) or operator results ` +
        `(--operator-results <path>); skipping.`,
    );
    return 0;
  }

  try {
    await Deno.stat(operatorResults);
  } catch {
    console.log(
      `[c12 skip] AgentBackend operator-results file not found: ${operatorResults}. ` +
        `Set OPERATOR_RESULTS to a readable JSON path or remove the env var to use the example.`,
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
      "agent",
      "--operator-results",
      operatorResults,
      // The scenario file declares `backend: scripted-plan`; the
      // agent backend is a strict superset of that path with real
      // define-stage bodies plugged in.
      "--allow-backend-mismatch",
    ],
    stdout: "inherit",
    stderr: "inherit",
    env: {
      ...readableEnv(),
      // Ensure the runner picks up the same binary the smoke driver
      // resolved. The runner uses `findSpecifyBin` which honours
      // `SPECIFY_BIN`.
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

async function supportsPrepareBranch(bin: string): Promise<boolean> {
  return await helpHas(bin, ["workspace", "prepare-branch", "--help"], "--change");
}

async function supportsWorkspacePush(bin: string): Promise<boolean> {
  return await helpHas(bin, ["workspace", "push", "--help"], "--format");
}

async function supportsChangeFinalize(bin: string): Promise<boolean> {
  return await helpHas(bin, ["change", "finalize", "--help"], "--format");
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
