// File-existence helpers for the assertion stage.
//
// These are the simplest helpers in the suite: every scenario with an
// `expected-artifacts:` list eventually flows through `assertFileExists`
// or `assertFileAbsent`. They live here, not under `acceptance/runner/`,
// so backends can stay free of assertion code.

import { isAbsolute, join, normalize, relative } from "jsr:@std/path@1";

import { fail as failRecord, pass as passRecord } from "./types.ts";
import type { AssertionRecord } from "./types.ts";

/** Result of a single file-existence probe. */
export interface FileProbe {
  /** Path the operator asked us to check, as declared in the scenario. */
  declaredPath: string;
  /** Resolved absolute path on disk. */
  absPath: string;
  /** True when the path resolved to a regular file. */
  exists: boolean;
  /** Byte size when `exists` is true. */
  size?: number;
}

/**
 * Confirm a scenario-declared expected-artifact path exists inside the
 * workspace. The path must be relative to the workspace root and must
 * not escape it via `..`. Returns a structured record either way so the
 * runner can merge it into `assertions.json`.
 */
export async function assertFileExists(
  id: string,
  workspace: string,
  declaredPath: string,
): Promise<AssertionRecord> {
  const probe = await probeWorkspacePath(workspace, declaredPath);
  if (!probe) {
    return failRecord(
      id,
      `Expected artifact path is unsafe (must be a relative path under the workspace).`,
      { summary: `unsafe path: ${declaredPath}` },
      "runner-setup",
    );
  }
  if (probe.exists) {
    return passRecord(
      id,
      `Expected artifact present: ${declaredPath} (${probe.size ?? 0} bytes)`,
      { summary: probe.absPath, paths: [declaredPath] },
    );
  }
  return failRecord(
    id,
    `Expected artifact missing: ${declaredPath}`,
    { summary: `missing path: ${probe.absPath}`, paths: [declaredPath] },
    "capability-brief",
  );
}

/**
 * Confirm a path the scenario forbids does NOT exist inside the
 * workspace. The path must be relative to the workspace root.
 */
export async function assertFileAbsent(
  id: string,
  workspace: string,
  declaredPath: string,
): Promise<AssertionRecord> {
  const probe = await probeWorkspacePath(workspace, declaredPath);
  if (!probe) {
    return failRecord(
      id,
      `Forbidden path is unsafe (must be a relative path under the workspace).`,
      { summary: `unsafe path: ${declaredPath}` },
      "runner-setup",
    );
  }
  if (!probe.exists) {
    return passRecord(
      id,
      `Forbidden path absent: ${declaredPath}`,
      { summary: probe.absPath, paths: [declaredPath] },
    );
  }
  return failRecord(
    id,
    `Forbidden path present: ${declaredPath} (${probe.size ?? 0} bytes)`,
    { summary: `present at ${probe.absPath}`, paths: [declaredPath] },
    "capability-brief",
  );
}

/**
 * Confirm that no file under `workspace` matches one of the supplied
 * relative-path globs. Used for `negative-expectations` like
 * "no contracts/**\/*.yaml". Globs are matched as posix-style strings;
 * the helper only supports `*` and `**` for the contracts smoke runner
 * scope. More expressive matching can land when a suite needs it.
 */
export async function assertNoMatchingPath(
  id: string,
  workspace: string,
  globs: string[],
): Promise<AssertionRecord> {
  const matched: string[] = [];
  const compiled = globs.map((g) => globToRegExp(g));

  try {
    for await (
      const entry of (await import("jsr:@std/fs@1/walk")).walk(workspace, {
        includeDirs: false,
        followSymlinks: false,
      })
    ) {
      const rel = relative(workspace, entry.path);
      if (compiled.some((re) => re.test(rel))) matched.push(rel);
    }
  } catch (e) {
    if (!(e instanceof Deno.errors.NotFound)) throw e;
  }

  if (matched.length === 0) {
    return passRecord(
      id,
      `No paths matched forbidden globs: ${globs.join(", ")}`,
      { summary: `0 matches under ${workspace}` },
    );
  }
  return failRecord(
    id,
    `${matched.length} path(s) matched forbidden globs: ${globs.join(", ")}`,
    { summary: `forbidden paths present`, paths: matched.slice(0, 16) },
    "capability-brief",
  );
}

/** Resolve a declared relative path under the workspace; reject escapes. */
async function probeWorkspacePath(
  workspace: string,
  declaredPath: string,
): Promise<FileProbe | null> {
  if (isAbsolute(declaredPath)) return null;
  const normWorkspace = normalize(workspace);
  const abs = normalize(join(normWorkspace, declaredPath));
  const relFromWorkspace = relative(normWorkspace, abs);
  if (relFromWorkspace.startsWith("..") || isAbsolute(relFromWorkspace)) {
    return null;
  }
  try {
    const stat = await Deno.stat(abs);
    if (!stat.isFile) {
      return { declaredPath, absPath: abs, exists: false };
    }
    return { declaredPath, absPath: abs, exists: true, size: stat.size };
  } catch (e) {
    if (e instanceof Deno.errors.NotFound) {
      return { declaredPath, absPath: abs, exists: false };
    }
    throw e;
  }
}

/** Translate a `*` / `**` glob into a regex anchored at start and end. */
function globToRegExp(glob: string): RegExp {
  let re = "^";
  let i = 0;
  while (i < glob.length) {
    const c = glob[i];
    if (c === "*") {
      if (glob[i + 1] === "*") {
        re += ".*";
        i += 2;
        if (glob[i] === "/") i += 1;
      } else {
        re += "[^/]*";
        i += 1;
      }
    } else if (/[.+^${}()|[\]\\]/.test(c)) {
      re += "\\" + c;
      i += 1;
    } else {
      re += c;
      i += 1;
    }
  }
  re += "$";
  return new RegExp(re);
}
