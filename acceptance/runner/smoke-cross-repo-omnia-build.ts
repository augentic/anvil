// Cross-repo OMNIA-BUILD smoke driver (RM-01 plan, C14a).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend omnia-build --allow-backend-mismatch` with the same
// environment-skip policy the C09–C13 smokes use, plus the
// contracts WASI tool probe (the omnia-build backend keeps the
// contract slice on `ContractsBuildPhaseDriver` so the contract
// validator handler still runs, and skipping the suite when the
// validator is missing keeps the operator-facing message friendly).
//
// The C14a backend is execute-only by design (boundary documented
// in `backends/omnia-build.ts`): it drives the contract slice with
// `ContractsBuildPhaseDriver` (real OpenAPI / JSON Schema emission),
// the backend slice with `OmniaBuildPhaseDriver` (real Rust crate
// skeleton emission), and the mobile slice with `StubPhaseDriver`
// (C14b reserves real Vectis builds). Push / finalize coverage
// stays on `scripted-finalize` / `agent`.

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-omnia-build-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    return 0;
  }

  console.log(
    `[c14a] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[c14a skip] acceptance-cross-repo-omnia-build-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` (pre-RFC-9 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlanNext(bin.path))) {
    console.log(
      `[c14a skip] acceptance-cross-repo-omnia-build-smoke: ${bin.path} does not expose ` +
        `\`specify change plan next\` / \`transition\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsPrepareBranch(bin.path))) {
    console.log(
      `[c14a skip] acceptance-cross-repo-omnia-build-smoke: ${bin.path} does not expose ` +
        `\`specify workspace prepare-branch\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsContractTool(bin.path))) {
    console.log(
      `[c14a skip] acceptance-cross-repo-omnia-build-smoke: ${bin.path} does not expose ` +
        `\`specify tool run contract\` (pre-RFC-15 release; the omnia-build ` +
        `backend reuses C13's contracts-build driver for the contract slice ` +
        `so a runnable validator is required).`,
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
      "omnia-build",
      // Scenario file declares `backend: scripted-plan`. The
      // omnia-build backend is a strict superset (plan creation +
      // execute loop + per-slice contracts/omnia phase drivers);
      // the mismatch flag tells the runner this is intentional.
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
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["init", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    return new TextDecoder().decode(stdout).includes("--hub");
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
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["workspace", "prepare-branch", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    return new TextDecoder().decode(stdout).includes("--change");
  } catch {
    return false;
  }
}

async function supportsContractTool(bin: string): Promise<boolean> {
  try {
    const { code, stdout, stderr } = await new Deno.Command(bin, {
      args: ["tool", "run", "--help"],
      stdout: "piped",
      stderr: "piped",
    }).output();
    if (code !== 0) return false;
    const text = new TextDecoder().decode(stdout) +
      new TextDecoder().decode(stderr);
    return text.includes("Run a registered tool") ||
      text.includes("tool name") ||
      text.includes("<TOOL>");
  } catch {
    return false;
  }
}

/** Inherit the operator's env so PATH / HOME / TMPDIR propagate. */
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
