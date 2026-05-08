// Evidence collectors for cross-repo acceptance runs (RM-01 plan, C07).
//
// Walks the hub + each project repo at scenario teardown, captures Git
// logs and fake `gh` PR state, and copies the registry / workspace
// snapshots into the run dir. The destination paths match the canonical
// inventory in
// `acceptance/suites/rm01-cross-repo/expected/evidence-inventory.md`:
//
//   <runDir>/
//     registry.yaml                 # snapshot of <hub>/registry.yaml
//     workspace.md                  # snapshot when present (sync-peers wrote one)
//     workspace-status.json         # JSON dump (when caller passes a status)
//     git/hub.log                   # git log --decorate --oneline --all -n 200
//     git/<project>.log             # one per project source repo
//     fake-gh/prs.json              # parsed dump of <stateDir>/*.pr
//
// The collector never throws on a missing source file; missing inputs
// produce a one-line marker in the destination so a failed run still
// preserves enough breadcrumbs to attribute the failure.

import { dirname, join } from "jsr:@std/path@1";

import { captureGitLog } from "./git.ts";
import type { GitEnv } from "./git.ts";
import { readAllPrStates } from "./fake-gh.ts";
import type { PrState } from "./fake-gh.ts";

export interface CollectEvidenceOptions {
  /** Run dir under the temp evidence root. */
  runDir: string;
  /** Hub project root. */
  hubDir: string;
  /**
   * Map of project name → repo dir to capture a `git log` from. Pass
   * either the working source repos or the hub's workspace clones; the
   * collector does not care which.
   */
  projectDirs: Record<string, string>;
  /** Fake `gh` state dir; the collector parses every `.pr` file here. */
  fakeGhStateDir: string;
  /** Per-run Git env (for the captured `git log` invocations). */
  env: GitEnv;
  /**
   * Optional pre-fetched workspace status JSON. When supplied the
   * collector writes `workspace-status.json` verbatim; otherwise the
   * file is omitted (callers that want it should pass the JSON in).
   */
  workspaceStatusJson?: unknown;
  /**
   * Optional pre-fetched plan.yaml contents. When supplied the
   * collector writes `plan.yaml.before-finalize` (matching the
   * inventory). C07 itself does not read plan state, but the collector
   * accepts the field so C09/C10/C11 can reuse the same entrypoint.
   */
  planYamlBeforeFinalize?: string;
}

export interface CollectEvidenceResult {
  /** Files actually written. Useful for the smoke target's summary. */
  written: string[];
  /** PR states captured for inspection. */
  prStates: PrState[];
}

/**
 * Walk the hub + projects + fake-`gh` and write the per-suite evidence
 * inventory. Best-effort: missing inputs leave a marker file so the
 * failure attribution stays clear.
 */
export async function collectEvidence(
  opts: CollectEvidenceOptions,
): Promise<CollectEvidenceResult> {
  const written: string[] = [];

  await ensureDir(opts.runDir);
  await ensureDir(join(opts.runDir, "git"));
  await ensureDir(join(opts.runDir, "fake-gh"));

  const registrySrc = join(opts.hubDir, "registry.yaml");
  const registryDst = join(opts.runDir, "registry.yaml");
  await copyOrMarker(registrySrc, registryDst);
  written.push(registryDst);

  const workspaceSrc = join(opts.hubDir, "workspace.md");
  const workspaceDst = join(opts.runDir, "workspace.md");
  if (await exists(workspaceSrc)) {
    await Deno.copyFile(workspaceSrc, workspaceDst);
    written.push(workspaceDst);
  }

  if (opts.workspaceStatusJson !== undefined) {
    const statusDst = join(opts.runDir, "workspace-status.json");
    await Deno.writeTextFile(
      statusDst,
      JSON.stringify(opts.workspaceStatusJson, null, 2) + "\n",
    );
    written.push(statusDst);
  }

  if (opts.planYamlBeforeFinalize !== undefined) {
    const planDst = join(opts.runDir, "plan.yaml.before-finalize");
    await Deno.writeTextFile(planDst, opts.planYamlBeforeFinalize);
    written.push(planDst);
  }

  const hubLogDst = join(opts.runDir, "git", "hub.log");
  await Deno.writeTextFile(hubLogDst, await captureGitLog(opts.hubDir, opts.env));
  written.push(hubLogDst);

  for (const [name, dir] of Object.entries(opts.projectDirs)) {
    const dst = join(opts.runDir, "git", `${name}.log`);
    await Deno.writeTextFile(dst, await captureGitLog(dir, opts.env));
    written.push(dst);
  }

  const prStates = await readAllPrStates(opts.fakeGhStateDir);
  const prsDst = join(opts.runDir, "fake-gh", "prs.json");
  await Deno.writeTextFile(
    prsDst,
    JSON.stringify(
      {
        "state-dir": opts.fakeGhStateDir,
        prs: prStates.map((s) => ({
          "repo-key": s.repoKey,
          number: s.number,
          state: s.state,
          merged: s.merged,
          branch: s.branch,
          url: s.url,
        })),
      },
      null,
      2,
    ) + "\n",
  );
  written.push(prsDst);

  return { written, prStates };
}

async function ensureDir(p: string): Promise<void> {
  await Deno.mkdir(p, { recursive: true });
}

async function copyOrMarker(src: string, dst: string): Promise<void> {
  await ensureDir(dirname(dst));
  if (await exists(src)) {
    await Deno.copyFile(src, dst);
  } else {
    await Deno.writeTextFile(
      dst,
      `# evidence-collector: ${src} did not exist at capture time\n`,
    );
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
