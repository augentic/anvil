// Hub setup primitive for the RM-01 test.
//
// `setupHub` is the **single primitive** that lands hub state, fixture
// source repos, bare remotes, fake `gh`, and the registry rows together.
// What it does, in order:
//
//   1. Lay out the per-run directory tree under `tempDir`:
//        <tempDir>/
//          <hubName>/        # hub repo (cwd for every specify call)
//          sources/<name>/   # working source repos
//          remotes/<name>.git/ # local bare remotes
//          bin/              # fake gh + fake-ssh (PATH-prepended)
//          gh-state/         # PR-state files
//          gitconfig         # empty isolated GIT_CONFIG_GLOBAL
//   2. Install fake `gh` + fake-SSH (`installFakeGh`).
//   3. Build a `GitEnv` so every subsequent subprocess sees the same
//      env (deterministic identity, isolated git config, fake SSH,
//      fake bin dir, GH_STATE_DIR).
//   4. Scaffold each fixture project + bare remote (`setupFixtureProject`).
//   5. Run `specify init --hub` in the hub dir.
//   6. Run `specify registry add` for each project (capability +
//      description from the project descriptor).
//   7. Return everything later helpers / assertions need.
//
// Assertions are layered on top by `tests/cross_repo.ts`.
// `setupHub` itself is intentionally side-effect-only on disk + CLI; it
// does NOT decide pass/fail.

import { join } from "jsr:@std/path@1";

import { installFakeGh } from "./fake-gh.ts";
import type { GitEnv } from "./git.ts";
import { defaultGithubUrl, setupFixtureProject } from "./projects.ts";
import type { FixtureProjectResult, SourceLayout } from "./projects.ts";
import { runSpecify } from "./specify-cli.ts";
import type { SpecifyBin } from "./specify-cli.ts";

/** A single project to register under the hub. */
export interface HubProjectDescriptor {
  /** Bare repo name (kebab-case). Must be unique per hub. */
  name: string;
  /** Capability id passed to `specify registry add --schema`. */
  capability: string;
  /**
   * Description passed to `specify registry add --description`.
   * Required (RFC-9 §2A `description-missing-multi-repo` invariant).
   */
  description: string;
  /** Optional source-layout override seeded into the working repo. */
  sourceLayout?: SourceLayout;
  /** Optional org slug override; defaults to `shop`. */
  orgSlug?: string;
}

export interface SetupHubOptions {
  /** Run-owned temp root. The helper creates children inside it. */
  tempDir: string;
  /** Hub project name (e.g. `shop-platform`). */
  hubName: string;
  /** Resolved `specify` binary metadata. */
  specifyBin: SpecifyBin;
  /** Project descriptors to scaffold + register. */
  projects: HubProjectDescriptor[];
  /**
   * Per-repo PR-number policy passed to `installFakeGh`. Defaults to
   * `DEFAULT_PR_NUMBERS` from `fake-gh.ts` when omitted.
   */
  prNumbers?: Record<string, number>;
  /**
   * Optional log-capture sinks. When set, every `git` and `specify`
   * call streams stdout/stderr here.
   */
  capture?: { stdoutLog?: string; stderrLog?: string };
}

export interface SetupHubResult {
  hubDir: string;
  registryPath: string;
  /** Map of `name → working source repo dir`. */
  projectDirs: Record<string, string>;
  /** Map of `name → bare remote dir`. */
  remoteDirs: Record<string, string>;
  /** Per-project metadata returned by `setupFixtureProject`. */
  projects: FixtureProjectResult[];
  fakeGhStateDir: string;
  fakeBinDir: string;
  /**
   * The fully-built `GitEnv` later helpers should reuse. Avoids each
   * caller re-deriving the same env block.
   */
  env: GitEnv;
  /** PR-number map actually written into the fake-`gh` script. */
  prNumbers: Record<string, number>;
}

/**
 * Build a hub + registered projects + fake-`gh` substrate. Side-effect
 * on disk + the supplied `specify` binary.
 */
export async function setupHub(opts: SetupHubOptions): Promise<SetupHubResult> {
  const layout = await materialiseLayout(opts.tempDir, opts.hubName);

  const ghInstall = await installFakeGh({
    binDir: layout.binDir,
    stateDir: layout.fakeGhStateDir,
    prNumbers: opts.prNumbers,
  });

  const env: GitEnv = {
    gitConfigGlobal: layout.gitConfigPath,
    remotesDir: layout.remotesDir,
    fakeSshScript: ghInstall.fakeSshPath,
    fakeBinDir: layout.binDir,
    fakeGhStateDir: ghInstall.stateDir,
    stdoutLog: opts.capture?.stdoutLog,
    stderrLog: opts.capture?.stderrLog,
  };

  const projects: FixtureProjectResult[] = [];
  const projectDirs: Record<string, string> = {};
  const remoteDirs: Record<string, string> = {};
  for (const p of opts.projects) {
    const sourceDir = join(layout.sourcesDir, p.name);
    const remoteDir = join(layout.remotesDir, `${p.name}.git`);
    const result = await setupFixtureProject({
      name: p.name,
      capability: p.capability,
      sourceDir,
      remoteDir,
      env,
      sourceLayout: p.sourceLayout,
    });
    // Override the URL when the descriptor specified a non-default org.
    if (p.orgSlug) {
      (result as { githubUrl: string }).githubUrl = defaultGithubUrl(
        p.name,
        p.orgSlug,
      );
    }
    projects.push(result);
    projectDirs[p.name] = sourceDir;
    remoteDirs[p.name] = remoteDir;
  }

  await runSpecify({
    bin: opts.specifyBin,
    cwd: layout.hubDir,
    args: ["init", "--name", opts.hubName, "--hub"],
    env,
  });

  for (let i = 0; i < opts.projects.length; i++) {
    const desc = opts.projects[i];
    const url = projects[i].githubUrl;
    await runSpecify({
      bin: opts.specifyBin,
      cwd: layout.hubDir,
      args: [
        "registry",
        "add",
        desc.name,
        "--url",
        url,
        "--schema",
        desc.capability,
        "--description",
        desc.description,
      ],
      env,
    });
  }

  return {
    hubDir: layout.hubDir,
    registryPath: join(layout.hubDir, "registry.yaml"),
    projectDirs,
    remoteDirs,
    projects,
    fakeGhStateDir: ghInstall.stateDir,
    fakeBinDir: layout.binDir,
    env,
    prNumbers: ghInstall.prNumbers,
  };
}

interface HubLayout {
  hubDir: string;
  sourcesDir: string;
  remotesDir: string;
  binDir: string;
  fakeGhStateDir: string;
  gitConfigPath: string;
}

async function materialiseLayout(
  tempDir: string,
  hubName: string,
): Promise<HubLayout> {
  const hubDir = join(tempDir, hubName);
  const sourcesDir = join(tempDir, "sources");
  const remotesDir = join(tempDir, "remotes");
  const binDir = join(tempDir, "bin");
  const fakeGhStateDir = join(tempDir, "gh-state");
  const gitConfigPath = join(tempDir, "gitconfig");

  for (const dir of [hubDir, sourcesDir, remotesDir, binDir, fakeGhStateDir]) {
    await Deno.mkdir(dir, { recursive: true });
  }
  // Empty isolated GIT_CONFIG_GLOBAL so the operator's `~/.gitconfig`
  // (especially `commit.gpgsign = true`) does not leak into the run.
  await Deno.writeTextFile(gitConfigPath, "");

  return {
    hubDir,
    sourcesDir,
    remotesDir,
    binDir,
    fakeGhStateDir,
    gitConfigPath,
  };
}
