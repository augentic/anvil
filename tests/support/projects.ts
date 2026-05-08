// Fixture-project setup helper for the RM-01 test.
//
// `setupFixtureProject` scaffolds a single source repo + bare remote
// pair under the test's temp root. The shape mirrors the
// `FixtureProject::new` constructor in `specify-cli/tests/cross_repo.rs`:
//
//   sources/<name>/                # working source repo (where we
//                                  # write the seed layout and commit)
//     .specify/project.yaml        # `name:` + `capability:`
//     README.md                    # so the seed commit has content
//     <sourceLayout files...>      # optional caller-supplied tree
//   remotes/<name>.git             # bare clone of the working repo
//
// The hub helper (`setupHub`) drives this for each registered project
// and then registers the project through `specify registry add` using
// the GitHub-shaped URL `git@github.com:<orgSlug>/<name>.git` so the
// fake-SSH script under `fake-gh.ts` can rewrite the operation onto
// the local bare remote.

import { join } from "jsr:@std/path@1";

import { gitCloneBare, gitCommit, gitInit } from "./git.ts";
import type { GitEnv } from "./git.ts";

/**
 * Optional source layout the helper materialises before the seed
 * commit. Paths are repo-relative; the helper creates parent
 * directories as needed.
 */
export type SourceLayout = ReadonlyArray<{
  path: string;
  contents: string;
}>;

export interface SetupFixtureProjectOptions {
  /** Bare repo name. Must match the registry entry name. */
  name: string;
  /** Capability id (e.g. `omnia@v1`). Written into `project.yaml`. */
  capability: string;
  /**
   * Working source-repo dir (the helper creates it). Typically
   * `<tempRoot>/sources/<name>`.
   */
  sourceDir: string;
  /**
   * Bare remote dir (the helper creates it). Typically
   * `<tempRoot>/remotes/<name>.git`.
   */
  remoteDir: string;
  /** Per-run Git env (carries fake SSH + bare-remote root). */
  env: GitEnv;
  /** Optional extra files to seed before the initial commit. */
  sourceLayout?: SourceLayout;
}

export interface FixtureProjectResult {
  name: string;
  capability: string;
  sourceDir: string;
  remoteDir: string;
  /** GitHub-shaped URL for `specify registry add --url`. */
  githubUrl: string;
}

/**
 * Compute the GitHub-shaped URL for a project using the fixture's
 * default org slug `shop`. Matches `FixtureProject::github_url` in
 * `cross_repo.rs`. Suites that need a different org should pass the
 * URL through `setupHub`'s project descriptor instead of relying on
 * this helper.
 */
export function defaultGithubUrl(name: string, orgSlug = "shop"): string {
  return `git@github.com:${orgSlug}/${name}.git`;
}

/**
 * Scaffold a single fixture project (working source repo + bare remote)
 * and return enough metadata for `setupHub` to register it.
 *
 * The function is intentionally side-effect-only on disk; it does NOT
 * call `specify registry add` itself (that lives in `hub.ts` so the
 * single `setup-hub` primitive owns the "consistency" invariants).
 */
export async function setupFixtureProject(
  opts: SetupFixtureProjectOptions,
): Promise<FixtureProjectResult> {
  const specifyDir = join(opts.sourceDir, ".specify");
  await Deno.mkdir(specifyDir, { recursive: true });
  await gitInit(opts.sourceDir, opts.env);

  await Deno.writeTextFile(
    join(opts.sourceDir, "README.md"),
    `# ${opts.name}\n\nSeed source repo for the RM-01 test.\n`,
  );
  await Deno.writeTextFile(
    join(specifyDir, "project.yaml"),
    `name: ${opts.name}\ncapability: ${opts.capability}\n`,
  );

  for (const file of opts.sourceLayout ?? []) {
    const target = join(opts.sourceDir, file.path);
    await Deno.mkdir(parentOf(target), { recursive: true });
    await Deno.writeTextFile(target, file.contents);
  }

  await gitCommit(opts.sourceDir, "seed project", opts.env);

  await gitCloneBare(opts.sourceDir, opts.remoteDir, opts.env);

  return {
    name: opts.name,
    capability: opts.capability,
    sourceDir: opts.sourceDir,
    remoteDir: opts.remoteDir,
    githubUrl: defaultGithubUrl(opts.name),
  };
}

function parentOf(p: string): string {
  const idx = p.lastIndexOf("/");
  return idx > 0 ? p.slice(0, idx) : ".";
}
