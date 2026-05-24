// Refine-skill replay. For each tests/fixtures/skills/refine/<case>/:
//   - schema-validate every Evidence document under `inputs/evidence/` against
//     `evidence.schema.json`.
//   - parse `expected/spec.md` with the W1.3 provenance parser; assert every
//     requirement block has `ID:` / `Sources:` / a closed `Status:` value
//     (`agreed | divergence-resolved | conflict-pending | conflict | unknown`).
//   - confirm `expected/{proposal,design,tasks}.md` are non-empty markdown
//     with at least one heading.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";
import { walk } from "jsr:@std/fs@1/walk";

import { walkSkillFixtures } from "../lib/fixtures.ts";
import { validateOrThrow } from "../lib/validators.ts";
import { parseSpec } from "../lib/spec_provenance.ts";

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.lstat(path);
    return true;
  } catch {
    return false;
  }
}

Deno.test("skills/refine fixtures: tree present", async () => {
  const fixtures = await walkSkillFixtures("refine");
  if (fixtures.length === 0) {
    throw new Error(
      "expected at least one refine fixture under tests/fixtures/skills/refine/",
    );
  }
});

Deno.test("skills/refine: every Evidence input schema-validates", async () => {
  const fixtures = await walkSkillFixtures("refine");
  let evidenceFiles = 0;
  for (const fx of fixtures) {
    const dir = join(fx.dir, "inputs", "evidence");
    if (!(await exists(dir))) continue;
    for await (
      const entry of walk(dir, { exts: [".yaml"], includeDirs: false })
    ) {
      evidenceFiles++;
      const data = parseYaml(await Deno.readTextFile(entry.path));
      await validateOrThrow("evidence.schema.json", data, entry.path);
    }
  }
  if (evidenceFiles === 0) {
    throw new Error("no Evidence fixtures discovered under refine/*/inputs/evidence/");
  }
});

Deno.test("skills/refine: every expected/spec.md parses with closed Status enum", async () => {
  const fixtures = await walkSkillFixtures("refine");
  let casesValidated = 0;
  for (const fx of fixtures) {
    const specPath = join(fx.dir, "expected", "spec.md");
    if (!(await exists(specPath))) continue;
    casesValidated++;
    const content = await Deno.readTextFile(specPath);
    const { requirements, errors } = parseSpec(content);
    if (errors.length > 0) {
      throw new Error(`${specPath}: ${errors.join("; ")}`);
    }
    if (requirements.length === 0) {
      throw new Error(`${specPath}: no requirement blocks parsed`);
    }
  }
  if (casesValidated === 0) {
    throw new Error("no refine fixture exposed an expected/spec.md");
  }
});

Deno.test("skills/refine: every expected/{proposal,design,tasks}.md is non-empty", async () => {
  const fixtures = await walkSkillFixtures("refine");
  for (const fx of fixtures) {
    for (const name of ["proposal.md", "design.md", "tasks.md"]) {
      const path = join(fx.dir, "expected", name);
      if (!(await exists(path))) continue;
      const content = await Deno.readTextFile(path);
      if (content.trim().length === 0) {
        throw new Error(`${path}: file is empty`);
      }
      if (!/^#/m.test(content)) {
        throw new Error(`${path}: no markdown headings present`);
      }
    }
  }
});
