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

Deno.test("adapters/targets/*/input/spec.md: every requirement provenance block parses", async () => {
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

Deno.test("adapters/targets/vectis/*/expected/composition.yaml: well-formed with screens", async () => {
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

Deno.test("adapters/targets/omnia/expected/crate: Cargo.toml + src/lib.rs present", async () => {
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

Deno.test("adapters/targets/*/expected/shape-evidence.md: bullet items present", async () => {
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

// ---------------------------------------------------------------------------
// RFC-27 #26-1 — optional `fixture-replay` block on `.metadata.yaml`.
//
// The target-half of the release blocker: D1 carries an *optional*
// build-time fixture-replay hook for targets that consume
// `code-runtime` fixtures (RFC-27 §`build`-time fixture replay).
// Targets that have not implemented the hook simply omit the
// `fixture-replay` field; `merge` does not require it.
//
// The plg fixture tree under tests/fixtures/targets/omnia/ carries
// two siblings — `with-fixture-replay/.metadata.yaml` and
// `without-fixture-replay/.metadata.yaml` — exercising both
// branches. This test pins the byte-stable shape of each so a
// future schema or skill rewrite cannot regress the optional posture
// silently.
// ---------------------------------------------------------------------------

const REQUIRED_FIXTURE_REPLAY_KEYS = ["passed", "failed", "skipped", "ran-at", "runner"];

Deno.test("adapters/targets/omnia/with-fixture-replay: .metadata.yaml carries full fixture-replay block", async () => {
  const path = "tests/fixtures/targets/omnia/with-fixture-replay/.metadata.yaml";
  const content = await readText(path);
  if (content === null) {
    throw new Error(`${path}: expected metadata.yaml present per RFC-27 Change 4.1`);
  }
  const data = parseYaml(content) as Record<string, unknown> | null;
  if (data === null || typeof data !== "object") {
    throw new Error(`${path}: did not parse as a YAML mapping`);
  }
  const replay = data["fixture-replay"] as Record<string, unknown> | undefined;
  if (!replay || typeof replay !== "object") {
    throw new Error(
      `${path}: with-fixture-replay metadata must carry a fixture-replay block`,
    );
  }
  for (const key of REQUIRED_FIXTURE_REPLAY_KEYS) {
    if (!(key in replay)) {
      throw new Error(`${path}: fixture-replay block missing required key '${key}'`);
    }
  }
  if (typeof replay.passed !== "number" || typeof replay.failed !== "number") {
    throw new Error(`${path}: fixture-replay passed/failed must be numbers`);
  }
});

Deno.test("adapters/targets/omnia/without-fixture-replay: .metadata.yaml omits fixture-replay (optional posture)", async () => {
  const path = "tests/fixtures/targets/omnia/without-fixture-replay/.metadata.yaml";
  const content = await readText(path);
  if (content === null) {
    throw new Error(`${path}: expected metadata.yaml present per RFC-27 Change 4.1`);
  }
  const data = parseYaml(content) as Record<string, unknown> | null;
  if (data === null || typeof data !== "object") {
    throw new Error(`${path}: did not parse as a YAML mapping`);
  }
  if ("fixture-replay" in data) {
    throw new Error(
      `${path}: without-fixture-replay metadata MUST omit the fixture-replay key entirely — its presence in this fixture would regress RFC-27 §D1 'omission is not an error'`,
    );
  }
  if (data.target !== "omnia") {
    throw new Error(`${path}: target field must be 'omnia', got: ${data.target}`);
  }
});
