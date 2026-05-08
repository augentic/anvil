// Wrappers around `specify workspace sync` and `specify workspace
// status --format json` for the acceptance runner (RM-01 plan, C07).
//
// These are thin shims that:
//   1. invoke the CLI with the runner's `GitEnv` so fake-SSH and fake
//      `gh` are wired in,
//   2. capture stdout/stderr to the run's log sinks,
//   3. parse the JSON status output into a typed shape later assertion
//      helpers (`workspace-clean-before-push`, `branch-prepared`, etc.)
//      can consume without re-parsing.
//
// The status shape is taken from the JSON `specify workspace status
// --format json` returns (see `specify-cli/tests/cross_repo.rs`'s
// `assert_workspace_ready_for_push`). Fields beyond what the C07 smoke
// needs are kept as `unknown` so future chunks can extend without a
// type churn round-trip.

import { runSpecify, runSpecifyJson } from "./specify-cli.ts";
import type { SpecifyBin, SpecifyRun } from "./specify-cli.ts";
import type { GitEnv } from "./git.ts";

/** Single slot in `workspace status --format json` output. */
export interface WorkspaceSlot {
  name: string;
  kind?: string;
  "current-branch"?: string;
  dirty?: boolean;
  "branch-matches-change"?: boolean;
  "project-config-present"?: boolean;
  /** Catch-all for fields the C07 smoke does not assert on. */
  [extra: string]: unknown;
}

export interface WorkspaceStatus {
  slots: WorkspaceSlot[];
  /** Catch-all for top-level fields the smoke does not consume yet. */
  [extra: string]: unknown;
}

/** Run `specify workspace sync` from the hub dir. */
export async function runWorkspaceSync(opts: {
  bin: SpecifyBin;
  hubDir: string;
  env: GitEnv;
}): Promise<SpecifyRun> {
  return await runSpecify({
    bin: opts.bin,
    cwd: opts.hubDir,
    args: ["workspace", "sync"],
    env: opts.env,
  });
}

/**
 * Run `specify workspace status --format json` and return the parsed
 * payload. The runner forwards a `--format json` global flag.
 */
export async function getWorkspaceStatus(opts: {
  bin: SpecifyBin;
  hubDir: string;
  env: GitEnv;
}): Promise<{ run: SpecifyRun; status: WorkspaceStatus }> {
  const { run, json } = await runSpecifyJson<WorkspaceStatus>({
    bin: opts.bin,
    cwd: opts.hubDir,
    args: ["workspace", "status"],
    env: opts.env,
  });
  return { run, status: json };
}
