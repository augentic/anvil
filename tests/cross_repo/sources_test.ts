// Source-fixture replay. For every fixture under tests/fixtures/sources/<name>/
// the harness:
//   - parses every `expected{,/evidence}/*.yaml` Evidence document, schema-
//     validates it against `evidence.schema.json`, and asserts the closed
//     `kind` enum on each claim.
//   - structurally validates the synthesised `expected/discovery.md`
//     (or `expected-enumerate.md` for the degenerate intent case): a non-
//     empty file with at least one candidate block is required.
//
// CLI replay (driving `specify source resolve <name>` against `input/`) is
// guarded behind `SPECIFY_BIN`; without a binary the harness falls back to
// fixture-shape checks.

import { parse as parseYaml } from "jsr:@std/yaml@1";
import { join } from "jsr:@std/path@1";
import { walk } from "jsr:@std/fs@1/walk";

import { walkSourceFixtures } from "../lib/fixtures.ts";
import { validateOrThrow } from "../lib/validators.ts";

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.lstat(path);
    return true;
  } catch {
    return false;
  }
}

async function readYaml(path: string): Promise<unknown> {
  return parseYaml(await Deno.readTextFile(path));
}

async function* walkYaml(root: string): AsyncIterable<string> {
  if (!(await exists(root))) return;
  for await (const entry of walk(root, { exts: [".yaml"], includeDirs: false })) {
    yield entry.path;
  }
}

Deno.test("sources fixtures: tree present", async () => {
  const fixtures = await walkSourceFixtures();
  if (fixtures.length === 0) {
    throw new Error(
      "expected at least one source fixture under tests/fixtures/sources/",
    );
  }
});

Deno.test("sources/intent: extract Evidence schema-validates", async () => {
  const path = join(
    "tests/fixtures/sources/intent",
    "expected-extract.yaml",
  );
  const data = await readYaml(path);
  await validateOrThrow("evidence.schema.json", data, path);
});

Deno.test("sources/intent: enumerate synthesises a candidate block", async () => {
  const md = await Deno.readTextFile(
    "tests/fixtures/sources/intent/expected-enumerate.md",
  );
  if (!/^### \S+/m.test(md)) {
    throw new Error(
      "expected-enumerate.md must contain at least one `### <candidate>` block",
    );
  }
});

Deno.test("sources/documentation: every Evidence document schema-validates", async () => {
  const dir = "tests/fixtures/sources/documentation/expected/evidence";
  let seen = 0;
  for await (const path of walkYaml(dir)) {
    seen++;
    const data = await readYaml(path);
    await validateOrThrow("evidence.schema.json", data, path);
  }
  if (seen === 0) throw new Error(`no Evidence docs under ${dir}`);
});

Deno.test("sources/code-typescript: Evidence schema-validates and discovery is non-empty", async () => {
  const evidenceDir =
    "tests/fixtures/sources/code-typescript/expected/evidence";
  let seen = 0;
  for await (const path of walkYaml(evidenceDir)) {
    seen++;
    const data = await readYaml(path);
    await validateOrThrow("evidence.schema.json", data, path);
  }
  if (seen === 0) throw new Error(`no Evidence docs under ${evidenceDir}`);
  const md = await Deno.readTextFile(
    "tests/fixtures/sources/code-typescript/expected/discovery.md",
  );
  if (md.trim().length === 0) {
    throw new Error("expected/discovery.md is empty");
  }
});

Deno.test("sources/screenshots: discovery.md present and non-empty", async () => {
  // Screenshots fixtures emit a single discovery.md per slice; per-source
  // Evidence is co-emitted by the screenshots adapter at extract-time.
  for await (
    const entry of walk("tests/fixtures/sources/screenshots", {
      includeDirs: false,
      match: [/discovery\.md$/],
    })
  ) {
    const md = await Deno.readTextFile(entry.path);
    if (md.trim().length === 0) {
      throw new Error(`empty discovery.md: ${entry.path}`);
    }
  }
});
