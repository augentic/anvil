// Cross-repo EXECUTE-LEVEL smoke driver (RM-01 plan, C10).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend scripted-execute --allow-backend-mismatch` with two policy
// decisions the runner core stays out of:
//
//   1. Skip with exit 0 when `specify` is not on PATH (and
//      `SPECIFY_BIN` is unset). Same policy as C07/C09 smokes.
//   2. Skip with exit 0 when the resolved `specify` predates the
//      RFC-9 / RFC-13 surface (`init --hub`, `change plan {next,
//      transition}`, `workspace prepare-branch`). C10 needs a binary
//      that supports the loop driver verbs.
//
// Backend-mismatch flag rationale:
//   The C09 scenario file declares `backend: scripted-plan`. C10
//   reuses the same scenario (no separate scenario file per the C10
//   amendment) and drives it through `scripted-execute` — a superset
//   that does plan creation AND the loop driver. The mismatch flag
//   tells the runner this is intentional. The plan-only smoke
//   continues to run with the declared backend.
//
// Boundary documentation:
//   The scripted-execute backend produces a deterministic stand-in
//   for `/change:execute loop`. It proves the assertion plumbing
//   (setup → plan-shape → role-based + execute-* rules) end-to-end
//   against a fixed CLI sequence. It does NOT prove `/change:execute
//   loop` itself does the right thing on the same brief — that
//   requires the reserved `agent` backend (deferred to a future
//   change). See `backends/README.md` §"Scripted-Execute Vs
//   Real-Agent Boundary" for the rest of the framing.

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-execute-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    console.log(
      "       This is by design — C16 owns the install policy; C10 must not destabilise CI when the dev tool is missing.",
    );
    return 0;
  }

  console.log(
    `[c10] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[c10 skip] acceptance-cross-repo-execute-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` / \`registry\` / \`workspace\` (pre-RFC-9 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlan(bin.path))) {
    console.log(
      `[c10 skip] acceptance-cross-repo-execute-smoke: ${bin.path} does not expose ` +
        `\`specify change plan\` (pre-RFC-13 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlanNext(bin.path))) {
    console.log(
      `[c10 skip] acceptance-cross-repo-execute-smoke: ${bin.path} does not expose ` +
        `\`specify change plan next\` / \`transition\` (needs RFC-9-aware binary; the pre-0.2 release does not).`,
    );
    return 0;
  }
  if (!(await supportsPrepareBranch(bin.path))) {
    console.log(
      `[c10 skip] acceptance-cross-repo-execute-smoke: ${bin.path} does not expose ` +
        `\`specify workspace prepare-branch\` (needs RFC-9-aware binary).`,
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
      "scripted-execute",
      // The scenario file declares `backend: scripted-plan`; the
      // scripted-execute backend is a strict superset of that path.
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
  try {
    const { code, stdout } = await new Deno.Command(bin, {
      args: ["workspace", "prepare-branch", "--help"],
      stdout: "piped",
      stderr: "null",
    }).output();
    if (code !== 0) return false;
    const text = new TextDecoder().decode(stdout);
    return text.includes("--change");
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
