// Walk the four scenario discovery roots from `acceptance/README.md`
// §Scenario Discovery and return the discovered scenarios.

import { walk } from "jsr:@std/fs@1/walk";
import { join, relative } from "jsr:@std/path@1";

import { tryParseScenarioFile } from "./scenario.ts";
import type { Scenario, ScenarioSource } from "./types.ts";

/**
 * Walk the discovery roots under `repoRoot` and return every opt-in
 * scenario. A markdown file is treated as a scenario only when it has
 * YAML frontmatter with at least `id` and `kind` (see
 * `acceptance/README.md` §Scenario Discovery).
 */
export async function discoverScenarios(repoRoot: string): Promise<Scenario[]> {
  const found: Scenario[] = [];

  // 1. Shared outside-in suites: acceptance/suites/<suite>/scenario.md
  await collectSuiteScenarios(repoRoot, found);

  // 2 + 3. Owner-local capability scenarios under capabilities/<cap>/tests/
  await collectCapabilityScenarios(repoRoot, found);

  // 4. Skill-owned fixtures promoted to scenario shape.
  await collectSkillFixtureScenarios(repoRoot, found);

  // Stable order: by scenario id for predictable --list output.
  found.sort((a, b) => a.frontmatter.id.localeCompare(b.frontmatter.id));
  return found;
}

async function collectSuiteScenarios(repoRoot: string, out: Scenario[]) {
  const suitesDir = join(repoRoot, "acceptance", "suites");
  if (!(await exists(suitesDir))) return;

  for await (const entry of Deno.readDir(suitesDir)) {
    if (!entry.isDirectory) continue;
    const scenarioPath = join(suitesDir, entry.name, "scenario.md");
    if (!(await exists(scenarioPath))) continue;
    const source: ScenarioSource = { kind: "suite", suite: entry.name };
    await tryAdd(scenarioPath, repoRoot, source, out);
  }
}

async function collectCapabilityScenarios(repoRoot: string, out: Scenario[]) {
  const capabilitiesDir = join(repoRoot, "capabilities");
  if (!(await exists(capabilitiesDir))) return;

  for await (const cap of Deno.readDir(capabilitiesDir)) {
    if (!cap.isDirectory) continue;
    const testsDir = join(capabilitiesDir, cap.name, "tests");
    if (!(await exists(testsDir))) continue;

    // Flat owner-local: capabilities/<cap>/tests/<scenario>.md
    for await (const entry of Deno.readDir(testsDir)) {
      if (entry.isFile && entry.name.endsWith(".md") && entry.name !== "README.md") {
        const path = join(testsDir, entry.name);
        const source: ScenarioSource = { kind: "capability-flat", capability: cap.name };
        await tryAdd(path, repoRoot, source, out);
      }

      // Directory form: capabilities/<cap>/tests/<scenario>/scenario.md
      if (entry.isDirectory) {
        const path = join(testsDir, entry.name, "scenario.md");
        if (await exists(path)) {
          const source: ScenarioSource = {
            kind: "capability-dir",
            capability: cap.name,
            scenarioDir: entry.name,
          };
          await tryAdd(path, repoRoot, source, out);
        }
      }
    }
  }
}

async function collectSkillFixtureScenarios(repoRoot: string, out: Scenario[]) {
  const pluginsDir = join(repoRoot, "plugins");
  if (!(await exists(pluginsDir))) return;

  // Promoted skill fixtures sit at:
  //   plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md
  for await (const entry of walk(pluginsDir, {
    match: [/\/fixtures\/[^/]+\/scenario\.md$/],
    includeDirs: false,
    followSymlinks: false,
  })) {
    const rel = relative(pluginsDir, entry.path);
    const parts = rel.split("/");
    // Expected: <plugin>/skills/<skill>/fixtures/<scenario>/scenario.md
    if (parts.length !== 6 || parts[1] !== "skills" || parts[3] !== "fixtures") continue;
    const source: ScenarioSource = {
      kind: "skill-fixture",
      plugin: parts[0],
      skill: parts[2],
      scenarioDir: parts[4],
    };
    await tryAdd(entry.path, repoRoot, source, out);
  }
}

async function tryAdd(
  path: string,
  repoRoot: string,
  source: ScenarioSource,
  out: Scenario[],
) {
  const rel = relative(repoRoot, path);
  const scenario = await tryParseScenarioFile(path, rel, source);
  if (scenario) out.push(scenario);
}

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}

/** Bucket label used to nest run directories under the temp root. */
export function bucketFor(scenario: Scenario): string {
  switch (scenario.source.kind) {
    case "suite":
      return `suites/${scenario.source.suite}`;
    case "capability-flat":
    case "capability-dir":
      return `capability/${scenario.source.capability}`;
    case "skill-fixture":
      return `plugins/${scenario.source.plugin}/${scenario.source.skill}`;
  }
}
