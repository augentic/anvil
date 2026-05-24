// Fixture walkers for the acceptance harness. Each walker yields typed
// case descriptors that the per-category test files in
// `tests/cross_repo/*.ts` consume.

import { fromFileUrl, join, resolve } from "jsr:@std/path@1";

export const REPO_ROOT = resolve(
  fromFileUrl(new URL("../..", import.meta.url)),
);

export const FIXTURES_DIR = join(REPO_ROOT, "tests/fixtures");

export interface SourceFixture {
  name: string;
  dir: string;
}

export interface TargetFixture {
  name: string;
  caseName: string | null;
  dir: string;
}

export interface SkillFixture {
  skill: string;
  caseName: string;
  dir: string;
}

async function listDirs(root: string): Promise<string[]> {
  const out: string[] = [];
  try {
    for await (const entry of Deno.readDir(root)) {
      if (entry.isDirectory) out.push(entry.name);
    }
  } catch {
    // Optional root.
  }
  return out.sort();
}

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.lstat(path);
    return true;
  } catch {
    return false;
  }
}

export async function walkSourceFixtures(): Promise<SourceFixture[]> {
  const root = join(FIXTURES_DIR, "sources");
  const out: SourceFixture[] = [];
  for (const name of await listDirs(root)) {
    out.push({ name, dir: join(root, name) });
  }
  return out;
}

export async function walkTargetFixtures(): Promise<TargetFixture[]> {
  const root = join(FIXTURES_DIR, "targets");
  const out: TargetFixture[] = [];
  for (const name of await listDirs(root)) {
    const targetDir = join(root, name);
    // Two layouts:
    //   adapters/targets/<name>/{input,expected}/        — single case
    //   adapters/targets/<name>/<case>/{input,expected}/ — multi case
    if (await exists(join(targetDir, "input"))) {
      out.push({ name, caseName: null, dir: targetDir });
      continue;
    }
    for (const caseName of await listDirs(targetDir)) {
      const caseDir = join(targetDir, caseName);
      if (await exists(join(caseDir, "input"))) {
        out.push({ name, caseName, dir: caseDir });
      }
    }
  }
  return out;
}

export async function walkSkillFixtures(skill: string): Promise<SkillFixture[]> {
  const root = join(FIXTURES_DIR, "skills", skill);
  const out: SkillFixture[] = [];
  for (const caseName of await listDirs(root)) {
    out.push({ skill, caseName, dir: join(root, caseName) });
  }
  return out;
}
