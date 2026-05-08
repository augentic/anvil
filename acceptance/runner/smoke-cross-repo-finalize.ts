// Cross-repo FINALIZE-LEVEL smoke driver (RM-01 plan, C11).
//
// Wraps `acceptance/runner/main.ts --suite rm01-cross-repo
// --backend scripted-finalize --allow-backend-mismatch` with the
// same skip-policy decisions C09/C10 make for their smokes:
//
//   1. Skip with exit 0 when `specify` is not on PATH (and
//      `SPECIFY_BIN` is unset). Same policy as C07/C09/C10 smokes.
//   2. Skip with exit 0 when the resolved `specify` predates the
//      RFC-9 / RFC-13 surface (`init --hub`, `change plan {next,
//      transition}`, `workspace {prepare-branch, push}`, `change
//      finalize`). C11 needs a binary that supports the full landing
//      path.
//
// Backend-mismatch flag rationale (same as C10):
//   The scenario file declares `backend: scripted-plan` (the
//   weakest-coverage default). Both `scripted-execute` and
//   `scripted-finalize` are strict supersets of that path; the
//   mismatch flag tells the runner this is intentional. The plan-
//   only and execute-only smokes continue to run with the declared
//   backend.
//
// Boundary documentation:
//   The scripted-finalize backend produces a deterministic stand-in
//   for the post-execute landing path (`workspace push` → operator
//   merge → `change finalize`). It proves the assertion plumbing
//   (setup → plan-shape → role-based + execute-* + push-* +
//   finalize-* rules) end-to-end against a fixed CLI sequence. It
//   does NOT prove the real `/change:plan orchestrate` skill (or any
//   other agent surface) does the right thing on the same brief —
//   that requires the reserved `agent` backend (deferred to a future
//   change). See `backends/README.md` §"Scripted-Finalize Vs
//   Real-Agent Boundary" for the rest of the framing.

import { dirname, fromFileUrl, join, resolve } from "jsr:@std/path@1";

import { findSpecifyBin } from "./specify-cli.ts";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..", "..");

async function main(): Promise<number> {
  const bin = await findSpecifyBin();
  if (!bin) {
    console.log(
      "[skip] acceptance-cross-repo-finalize-smoke: no `specify` binary found on PATH (and SPECIFY_BIN unset).",
    );
    console.log(
      "       Build/install `specify-cli` (or set SPECIFY_BIN=/path/to/specify) and re-run.",
    );
    console.log(
      "       This is by design — C16 owns the install policy; C11 must not destabilise CI when the dev tool is missing.",
    );
    return 0;
  }

  console.log(
    `[c11] specify: ${bin.path} (${bin.version ?? "unknown version"})`,
  );

  if (!(await supportsHubSurface(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`init --hub\` / \`registry\` / \`workspace\` (pre-RFC-9 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlan(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`specify change plan\` (pre-RFC-13 release).`,
    );
    return 0;
  }
  if (!(await supportsChangePlanNext(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`specify change plan next\` / \`transition\` (needs RFC-9-aware binary; the pre-0.2 release does not).`,
    );
    return 0;
  }
  if (!(await supportsPrepareBranch(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`specify workspace prepare-branch\` (needs RFC-9-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsWorkspacePush(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`specify workspace push\` (needs RFC-3b-aware binary).`,
    );
    return 0;
  }
  if (!(await supportsChangeFinalize(bin.path))) {
    console.log(
      `[c11 skip] acceptance-cross-repo-finalize-smoke: ${bin.path} does not expose ` +
        `\`specify change finalize\` (needs RFC-9 §4C-aware binary).`,
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
      "scripted-finalize",
      // The scenario file declares `backend: scripted-plan`; the
      // scripted-finalize backend is a strict superset of that path.
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
