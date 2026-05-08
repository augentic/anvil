// Wrapper around the `specify` CLI for the acceptance runner (RM-01
// plan, C07).
//
// Every Specify lifecycle mutation goes through `specify`. This file is
// the single place subprocess flags / env propagation lives so helpers
// in `hub.ts`, `projects.ts`, `workspace-sync.ts`, and the RM-01 test
// all behave identically.

import { subprocessEnv } from "./git.ts";
import type { GitEnv } from "./git.ts";
import { appendLog } from "./evidence.ts";

/** Resolved path to the `specify` binary the helpers should invoke. */
export interface SpecifyBin {
  /** Absolute path to the `specify` binary. */
  path: string;
  /** Reported `specify --version` (best-effort; `null` when unknown). */
  version: string | null;
}

/** Result of a single `specify` invocation. */
export interface SpecifyRun {
  args: string[];
  cwd: string;
  exitCode: number;
  stdout: string;
  stderr: string;
}

/** Thrown when `runSpecify` sees a non-zero exit. */
export class SpecifyCommandError extends Error {
  constructor(public readonly run: SpecifyRun, bin: string) {
    super(
      `${bin} ${
        run.args.join(" ")
      } failed in ${run.cwd} (exit ${run.exitCode})\n` +
        `--- stdout (last 50 lines) ---\n${tail(run.stdout, 50)}\n` +
        `--- stderr (last 50 lines) ---\n${tail(run.stderr, 50)}`,
    );
    this.name = "SpecifyCommandError";
  }
}

/**
 * Locate a usable `specify` binary. Resolution order:
 *   1. `SPECIFY_BIN` env var (explicit override; useful for the
 *      acceptance smoke target),
 *   2. plain `specify` on `PATH`.
 *
 * Best-effort `--version` capture is included so the smoke target can
 * print "skipped: specify not found" with enough context for the
 * operator to fix their environment.
 */
export async function findSpecifyBin(): Promise<SpecifyBin | null> {
  const candidates = [Deno.env.get("SPECIFY_BIN"), "specify"].filter(
    (c): c is string => typeof c === "string" && c.length > 0,
  );

  for (const candidate of candidates) {
    try {
      const { code, stdout } = await new Deno.Command(candidate, {
        args: ["--version"],
        stdout: "piped",
        stderr: "null",
      }).output();
      if (code !== 0) continue;
      return {
        path: candidate,
        version: new TextDecoder().decode(stdout).trim() || null,
      };
    } catch {
      // ignore — try next candidate
    }
  }
  return null;
}

/**
 * Run `specify` with the runner's deterministic env and capture
 * stdout/stderr. Throws `SpecifyCommandError` on non-zero exit.
 */
export async function runSpecify(opts: {
  bin: SpecifyBin;
  cwd: string;
  args: string[];
  env: GitEnv;
}): Promise<SpecifyRun> {
  const cmd = new Deno.Command(opts.bin.path, {
    cwd: opts.cwd,
    args: opts.args,
    env: subprocessEnv(opts.env),
    stdout: "piped",
    stderr: "piped",
  });
  const { code, stdout, stderr } = await cmd.output();
  const run: SpecifyRun = {
    args: opts.args,
    cwd: opts.cwd,
    exitCode: code,
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
  };
  await captureSpecify(opts.bin.path, run, opts.env);
  if (code !== 0) throw new SpecifyCommandError(run, opts.bin.path);
  return run;
}

/** Convenience: run `specify ... --format json` and parse the stdout. */
export async function runSpecifyJson<T = unknown>(opts: {
  bin: SpecifyBin;
  cwd: string;
  args: string[];
  env: GitEnv;
}): Promise<{ run: SpecifyRun; json: T }> {
  const args = injectFormatJson(opts.args);
  const run = await runSpecify({ ...opts, args });
  let json: T;
  try {
    json = JSON.parse(run.stdout) as T;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new SpecifyCommandError(
      {
        ...run,
        stderr: run.stderr +
          `\n[runner] failed to parse stdout as JSON: ${msg}\n`,
      },
      opts.bin.path,
    );
  }
  return { run, json };
}

function injectFormatJson(args: string[]): string[] {
  // The CLI accepts `--format json` either as a global option (before
  // the subcommand) or attached to a subcommand. The cross-repo Rust
  // test sticks it before the subcommand for `change`, `workspace`, and
  // `registry` verbs that bubble it up to the global parser. Match that
  // convention so our calls go through the same code paths.
  if (args.includes("--format")) return args;
  return ["--format", "json", ...args];
}

async function captureSpecify(
  bin: string,
  run: SpecifyRun,
  env: GitEnv,
): Promise<void> {
  const header = `\n$ ${bin} ${
    run.args.join(" ")
  }  (cwd=${run.cwd}, exit=${run.exitCode})\n`;
  if (env.stdoutLog) await appendLog(env.stdoutLog, header + run.stdout);
  if (env.stderrLog) await appendLog(env.stderrLog, header + run.stderr);
}

function tail(text: string, lines: number): string {
  const all = text.split("\n");
  if (all.length <= lines) return text.replace(/\n$/, "");
  return all.slice(-lines).join("\n");
}
