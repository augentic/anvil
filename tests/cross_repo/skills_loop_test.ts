// Execute / build / merge / finalize skill-replay. These fixtures
// are walkthroughs (`expected-trace.md`, `expected-stop-hint.md`,
// `expected.md`, `transcript.md`) describing the visible side effects of
// each skill body. Byte-replay against an LLM-driven skill body is out of
// scope for the harness, so the assertions here are structural:
//
//   - every `input/plan.yaml` is well-formed YAML with a `slices[]` array.
//   - every `input/slice-metadata.yaml`, when present, is a YAML mapping.
//   - every expected transcript / trace / stop-hint markdown file is
//     non-empty and lead-headed.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";

import { walkSkillFixtures } from "../lib/fixtures.ts";

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.lstat(path);
    return true;
  } catch {
    return false;
  }
}

async function assertYamlMapping(path: string): Promise<unknown> {
  const content = await Deno.readTextFile(path);
  const data = parseYaml(content);
  if (data === null || typeof data !== "object" || Array.isArray(data)) {
    throw new Error(`${path}: expected a YAML mapping`);
  }
  return data;
}

async function assertNonEmptyMarkdown(path: string): Promise<void> {
  const content = await Deno.readTextFile(path);
  if (content.trim().length === 0) throw new Error(`${path}: empty`);
  if (!/^#/m.test(content)) {
    throw new Error(`${path}: no markdown heading`);
  }
}

async function checkLoopFixture(skill: string): Promise<void> {
  const fixtures = await walkSkillFixtures(skill);
  if (fixtures.length === 0) {
    throw new Error(
      `expected at least one ${skill} fixture under tests/fixtures/skills/${skill}/`,
    );
  }
  for (const fx of fixtures) {
    const planPath = join(fx.dir, "input", "plan.yaml");
    if (await exists(planPath)) {
      const data = await assertYamlMapping(planPath) as Record<string, unknown>;
      if (!Array.isArray(data["slices"])) {
        throw new Error(`${planPath}: missing slices[] array`);
      }
    }

    const metaPath = join(fx.dir, "input", "slice-metadata.yaml");
    if (await exists(metaPath)) {
      await assertYamlMapping(metaPath);
    }

    for (
      const name of [
        "expected-trace.md",
        "expected-stop-hint.md",
        "expected.md",
        "transcript.md",
      ]
    ) {
      const path = join(fx.dir, name);
      if (await exists(path)) await assertNonEmptyMarkdown(path);
    }
  }
}

Deno.test("skills/execute: fixture-shape", async () => {
  await checkLoopFixture("execute");
});

Deno.test("skills/build: fixture-shape", async () => {
  await checkLoopFixture("build");
});

Deno.test("skills/merge: fixture-shape", async () => {
  await checkLoopFixture("merge");
});

Deno.test("skills/finalize: fixture-shape", async () => {
  await checkLoopFixture("finalize");
});
