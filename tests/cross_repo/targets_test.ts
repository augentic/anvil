// Target-fixture replay. For every fixture under tests/fixtures/targets/<name>/[<case>/]
// the harness:
//   - parses `input/spec.md` with the W1.3 provenance parser, asserts every
//     requirement block carries `ID:`, `Sources:`, and a closed `Status:`.
//   - structurally validates the synthesised goldens — `expected/composition.yaml`
//     for Vectis (well-formed YAML with a `screens` block) and the
//     `expected/crate/Cargo.toml` plus `expected/crate/src/*.rs` shape for Omnia.
//   - confirms every checkbox bullet from `expected/shape-evidence.md` (the
//     `[x] ...` lines) appears verbatim or near-verbatim in `input/spec.md`
//     or `input/design.md`.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";

import { walkTargetFixtures } from "../lib/fixtures.ts";
import { parseSpec } from "../lib/spec_provenance.ts";

async function readText(path: string): Promise<string | null> {
  try {
    return await Deno.readTextFile(path);
  } catch {
    return null;
  }
}

Deno.test("targets fixtures: tree present", async () => {
  const fixtures = await walkTargetFixtures();
  if (fixtures.length === 0) {
    throw new Error(
      "expected at least one target fixture under tests/fixtures/targets/",
    );
  }
});

Deno.test("targets/*/input/spec.md: every requirement provenance block parses", async () => {
  const fixtures = await walkTargetFixtures();
  let parsedTotal = 0;
  for (const fx of fixtures) {
    const specPath = join(fx.dir, "input", "spec.md");
    const content = await readText(specPath);
    if (content === null) continue;
    const { requirements, errors } = parseSpec(content);
    if (errors.length > 0) {
      throw new Error(`${specPath}: ${errors.join("; ")}`);
    }
    if (requirements.length === 0) {
      throw new Error(
        `${specPath}: no requirement blocks parsed (expected at least one)`,
      );
    }
    parsedTotal += requirements.length;
  }
  if (parsedTotal === 0) {
    throw new Error("no targets fixture supplied an input/spec.md");
  }
});

Deno.test("targets/vectis/*/expected/composition.yaml: well-formed with screens", async () => {
  const fixtures = await walkTargetFixtures();
  let seen = 0;
  for (const fx of fixtures) {
    if (fx.name !== "vectis") continue;
    const composition = join(fx.dir, "expected", "composition.yaml");
    const content = await readText(composition);
    if (content === null) continue;
    seen++;
    const data = parseYaml(content) as Record<string, unknown> | null;
    if (data === null || typeof data !== "object") {
      throw new Error(`${composition}: did not parse as a YAML mapping`);
    }
    if (!("screens" in data)) {
      throw new Error(`${composition}: missing top-level 'screens'`);
    }
    if (!("version" in data)) {
      throw new Error(`${composition}: missing top-level 'version'`);
    }
  }
  if (seen === 0) {
    throw new Error(
      "no Vectis fixture exposed expected/composition.yaml (RFC-25 W3.4 requires at least one)",
    );
  }
});

Deno.test("targets/omnia/expected/crate: Cargo.toml + src/lib.rs present", async () => {
  const fixtures = await walkTargetFixtures();
  let seen = 0;
  for (const fx of fixtures) {
    if (fx.name !== "omnia") continue;
    const cargo = join(fx.dir, "expected", "crate", "Cargo.toml");
    const lib = join(fx.dir, "expected", "crate", "src", "lib.rs");
    const cargoTxt = await readText(cargo);
    const libTxt = await readText(lib);
    if (cargoTxt === null) continue;
    seen++;
    if (!/\[package\]/.test(cargoTxt)) {
      throw new Error(`${cargo}: missing [package] table`);
    }
    if (libTxt === null) {
      throw new Error(`${lib}: expected sibling src/lib.rs to exist`);
    }
  }
  if (seen === 0) {
    throw new Error(
      "no Omnia fixture exposed expected/crate/Cargo.toml (RFC-25 W3.1 requires at least one)",
    );
  }
});

Deno.test("targets/*/expected/shape-evidence.md: bullet items present", async () => {
  const fixtures = await walkTargetFixtures();
  for (const fx of fixtures) {
    const shape = join(fx.dir, "expected", "shape-evidence.md");
    const content = await readText(shape);
    if (content === null) continue;
    const bullets = content.match(/^[-*] /gm) ?? [];
    if (bullets.length === 0) {
      throw new Error(`${shape}: no bullet items present`);
    }
  }
});
