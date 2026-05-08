// Git helpers for the acceptance runner (RM-01 plan, C07).
//
// All Git operations the cross-repo setup primitives need go through the
// helpers in this file so:
//   1. every `git` invocation runs with deterministic identity
//      (`GIT_AUTHOR_NAME`, `GIT_COMMITTER_NAME`, etc.) and an isolated
//      `GIT_CONFIG_GLOBAL` — no operator state leaks into the run,
//   2. callers can opt into the fake-`gh` / fake-SSH boundary by passing
//      a `GitEnv` that already carries `FAKE_GITHUB_REMOTE_ROOT` and
//      `GIT_SSH_COMMAND`,
//   3. stdout AND stderr are captured into the shared `stdout.log` /
//      `stderr.log` per the C07 guardrail (every subprocess call surfaces
//      its last 50 lines on assertion failure),
//   4. helpers return the same `GitRun` shape so a higher-level helper
//      can wrap a non-zero exit into a `runner-setup` fault domain.
//
// The shape mirrors `specify-cli/tests/cross_repo.rs` (`run_git`,
// `git_output`, `GIT_TEST_ENV`) so a reader who knows the Layer 0
// substrate test recognises the Layer 4 setup translation.

import { join } from "jsr:@std/path@1";

import { appendLog } from "./evidence.ts";

/** Deterministic identity used for every Git operation in setup. */
export const GIT_TEST_ENV: Readonly<Record<string, string>> = Object.freeze({
  GIT_AUTHOR_NAME: "Specify Acceptance",
  GIT_AUTHOR_EMAIL: "specify-acceptance@example.invalid",
  GIT_COMMITTER_NAME: "Specify Acceptance",
  GIT_COMMITTER_EMAIL: "specify-acceptance@example.invalid",
});

/**
 * Per-run Git environment. Built once by `setupHub` (or any caller that
 * wants to drive Git directly) and threaded through every helper. The
 * shape is intentionally narrow so a reader can trace exactly which env
 * vars cross the subprocess boundary.
 */
export interface GitEnv {
  /** Isolated `GIT_CONFIG_GLOBAL` file (typically empty). */
  gitConfigGlobal: string;
  /** Bare-remote root that fake-SSH rewrites onto. */
  remotesDir: string;
  /** Fake-SSH script path (used as `GIT_SSH_COMMAND`). */
  fakeSshScript: string;
  /** Bin dir prepended to PATH so subprocesses see fake `gh`. */
  fakeBinDir: string;
  /** Fake `gh` PR-state directory (`gh-state/<repo>.pr`). */
  fakeGhStateDir: string;
  /** Optional capture sinks. When set, every subprocess streams here. */
  stdoutLog?: string;
  stderrLog?: string;
}

/** Result of a single git invocation. */
export interface GitRun {
  args: string[];
  cwd: string;
  exitCode: number;
  stdout: string;
  stderr: string;
}

/** Thrown when a `git` invocation exits non-zero in `runGit`. */
export class GitCommandError extends Error {
  constructor(public readonly run: GitRun) {
    super(
      `git ${run.args.join(" ")} failed in ${run.cwd} (exit ${run.exitCode})\n` +
        `--- stdout ---\n${tail(run.stdout, 50)}\n` +
        `--- stderr ---\n${tail(run.stderr, 50)}`,
    );
    this.name = "GitCommandError";
  }
}

/**
 * Run `git` in `cwd` with the deterministic identity and the supplied
 * `GitEnv`. Captures stdout/stderr; appends both to the configured log
 * sinks (when any). Throws `GitCommandError` on non-zero exit.
 */
export async function runGit(
  cwd: string,
  args: string[],
  env: GitEnv,
): Promise<GitRun> {
  const cmd = new Deno.Command("git", {
    cwd,
    args,
    env: subprocessEnv(env),
    stdout: "piped",
    stderr: "piped",
  });
  const { code, stdout, stderr } = await cmd.output();
  const run: GitRun = {
    args,
    cwd,
    exitCode: code,
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
  };
  await captureSubprocess("git", args, run, env);
  if (code !== 0) throw new GitCommandError(run);
  return run;
}

/** `runGit` then return the trimmed stdout (parity with `git_output` in cross_repo.rs). */
export async function gitOutput(
  cwd: string,
  args: string[],
  env: GitEnv,
): Promise<string> {
  const run = await runGit(cwd, args, env);
  return run.stdout.trim();
}

/** Initialise an empty repo at `dir` on the `main` branch. */
export async function gitInit(dir: string, env: GitEnv): Promise<void> {
  await Deno.mkdir(dir, { recursive: true });
  await runGit(dir, ["init", "-b", "main"], env);
}

/** Stage paths and commit with the supplied message (no GPG signing). */
export async function gitCommit(
  dir: string,
  message: string,
  env: GitEnv,
  pathspecs: string[] = ["."],
): Promise<void> {
  await runGit(dir, ["add", ...pathspecs], env);
  await runGit(dir, ["commit", "--no-gpg-sign", "-m", message], env);
}

/** Clone `source` as a bare repository at `dest`. */
export async function gitCloneBare(
  source: string,
  dest: string,
  env: GitEnv,
): Promise<void> {
  await Deno.mkdir(dirOf(dest), { recursive: true });
  // Use a parent-dir cwd so a relative `dest` resolves predictably.
  await runGit(dirOf(dest), ["clone", "--bare", source, dest], env);
}

/**
 * Capture `git log --decorate --oneline --all -n <limit>` into a single
 * trimmed string. Used by the evidence collectors for `git/<repo>.log`.
 */
export async function captureGitLog(
  dir: string,
  env: GitEnv,
  limit = 200,
): Promise<string> {
  // Allow callers to pass a non-existent dir without crashing the runner;
  // a missing log shows up in evidence as a one-line marker instead.
  try {
    await Deno.stat(dir);
  } catch {
    return `# evidence-collector: ${dir} does not exist\n`;
  }
  try {
    const run = await runGit(
      dir,
      ["log", "--decorate", "--oneline", "--all", `-n`, String(limit)],
      env,
    );
    return run.stdout;
  } catch (e) {
    if (e instanceof GitCommandError) {
      // Most common cause: empty repo (no commits yet). Surface that
      // explicitly rather than failing evidence collection.
      return `# evidence-collector: git log failed for ${dir}\n# stderr:\n${e.run.stderr}\n`;
    }
    throw e;
  }
}

/** Build the env block passed to a git/gh subprocess. */
export function subprocessEnv(env: GitEnv): Record<string, string> {
  const base = {
    ...GIT_TEST_ENV,
    GIT_CONFIG_GLOBAL: env.gitConfigGlobal,
    GIT_SSH_COMMAND: env.fakeSshScript,
    FAKE_GITHUB_REMOTE_ROOT: env.remotesDir,
    GH_STATE_DIR: env.fakeGhStateDir,
    PATH: pathWithFront(env.fakeBinDir),
  } as Record<string, string>;
  // Carry through HOME so git can locate the (empty) global config file.
  const home = Deno.env.get("HOME");
  if (home) base.HOME = home;
  return base;
}

/** Prepend a directory to the inherited `PATH`. */
export function pathWithFront(front: string): string {
  const existing = Deno.env.get("PATH") ?? "";
  return existing ? `${front}:${existing}` : front;
}

async function captureSubprocess(
  bin: string,
  args: string[],
  run: GitRun,
  env: GitEnv,
): Promise<void> {
  const header = `\n$ ${bin} ${args.join(" ")}  (cwd=${run.cwd}, exit=${run.exitCode})\n`;
  if (env.stdoutLog) await appendLog(env.stdoutLog, header + run.stdout);
  if (env.stderrLog) await appendLog(env.stderrLog, header + run.stderr);
}

function tail(text: string, lines: number): string {
  const all = text.split("\n");
  if (all.length <= lines) return text.replace(/\n$/, "");
  return all.slice(-lines).join("\n");
}

function dirOf(p: string): string {
  return join(p, "..");
}
