// Cross-repo PLAN-LEVEL smoke driver (RM-01 plan, C09).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend scripted-plan` with two policy decisions the runner core
// stays out of:
//
//   1. Skip with exit 0 when `specify` is not on PATH (and
//      `SPECIFY_BIN` is unset). The C16 policy promotes this target
//      into CI; until then the make target must not destabilise PR
//      runs on hosts that don't carry the dev tool.
//   2. Skip with exit 0 when the resolved `specify` predates the
//      RFC-9 surface (`init --hub` / `change plan` / `workspace
//      sync`). Same reason.
//
// Boundary documentation:
//   The scripted-plan backend produces a deterministic stand-in for
//   `/change:plan`. It proves the assertion plumbing (hub setup →
//   plan-shape rules → role-based assertions) works end-to-end against
//   a fixed CLI sequence. It does NOT prove `/change:plan` itself does
//   the right thing on the same brief — that requires the reserved
//   `agent` backend (RM-15-ish or earlier). Operators wanting to drive
//   the same scenario through real `/change:plan` should:
//     a. invoke `/change:plan oauth-login` manually in a fresh hub,
//     b. capture the resulting plan into an `--operator-results` file,
//     c. re-run via `--backend manual --operator-results <path>` once
//        the operator-results path teaches the manual backend to
//        forward setup state.
//   That flow is documented in scenario.md §Invocation; until the
//   real-agent backend ships, the scripted-plan path is the only
//   automated end-to-end proof the framework keeps green.

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-plan-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    console.log(
      "       This is by design — C16 owns the install policy; C09 must not destabilise CI when the dev tool is missing.",
    );
    return 0;
  }

  console.log(
    `[c09] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[skip] acceptance-cross-repo-plan-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` / \`registry\` / \`workspace\` (pre-RFC-9 release).`,
    );
    return 0;
  }

  if (!(await supportsChangePlan(bin.path))) {
    console.log(
      `[skip] acceptance-cross-repo-plan-smoke: ${bin.path} does not expose ` +
        `\`specify change plan\` (pre-RFC-13 release).`,
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
      "scripted-plan",
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
