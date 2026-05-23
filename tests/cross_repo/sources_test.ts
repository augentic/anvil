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

Deno.test("adapters/sources/intent: extract Evidence schema-validates", async () => {
  const path = join(
    "tests/fixtures/sources/intent",
    "expected-extract.yaml",
  );
  const data = await readYaml(path);
  await validateOrThrow("evidence.schema.json", data, path);
});

Deno.test("adapters/sources/intent: enumerate synthesises a candidate block", async () => {
  const md = await Deno.readTextFile(
    "tests/fixtures/sources/intent/expected-enumerate.md",
  );
  if (!/^### \S+/m.test(md)) {
    throw new Error(
      "expected-enumerate.md must contain at least one `### <candidate>` block",
    );
  }
});

Deno.test("adapters/sources/documentation: every Evidence document schema-validates", async () => {
  const dir = "tests/fixtures/sources/documentation/expected/evidence";
  let seen = 0;
  for await (const path of walkYaml(dir)) {
    seen++;
    const data = await readYaml(path);
    await validateOrThrow("evidence.schema.json", data, path);
  }
  if (seen === 0) throw new Error(`no Evidence docs under ${dir}`);
});

Deno.test("adapters/sources/code-typescript: Evidence schema-validates and discovery is non-empty", async () => {
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

Deno.test("adapters/sources/screenshots: discovery.md present and non-empty", async () => {
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

// ---------------------------------------------------------------------------
// RFC-27 #26-1 — `runtime-fixtures` source adapter end-to-end fixture walk.
//
// The 4.1 fixture under tests/fixtures/sources/runtime-fixtures/ is the
// golden-file half of the release blocker for D1. This test pins the
// deterministic shape of the fixture without re-running the
// `enumerate` / `extract` briefs (those require an LLM and are
// explicitly out of scope per the harness top comment). The
// assertions cover what RFC-27 §Acceptance scenarios #26-1 demands at
// the data-structure level:
//
//   1. The `runtime-fixtures` source adapter is discoverable in `plg`.
//   2. Every `expected/evidence.yaml` schema-validates against
//      `schemas/evidence.schema.json` (D1: `kind: example` joins the
//      closed enum).
//   3. Every `example`-kind claim carries a `fixture-digest:
//      sha256:<hex>` anchor and the source-adapter default
//      `authority: behaviour` posture.
//   4. The synthesised `expected/fusion.yaml` schema-validates
//      against `schemas/slice/fusion.schema.json` (D4 audit surface
//      for the runtime-sourced slice).
//   5. The candidate inventory in `expected/discovery.md` carries a
//      `### <slug>` block whose `sources:` line names `runtime`
//      (the bound source key for `runtime-fixtures` per the RFC §Binding
//      example).
// ---------------------------------------------------------------------------

Deno.test("adapters/sources/runtime-fixtures: adapter manifest is discoverable in plg tree", async () => {
  const manifestPath = "adapters/sources/runtime-fixtures/adapter.yaml";
  if (!(await exists(manifestPath))) {
    throw new Error(
      `expected ${manifestPath} to exist; RFC-27 Change 3.1 landed the runtime-fixtures source adapter`,
    );
  }
  const manifest = await readYaml(manifestPath) as Record<string, unknown>;
  if (manifest.name !== "runtime-fixtures") {
    throw new Error(`adapter name must be runtime-fixtures, got: ${manifest.name}`);
  }
  if (manifest.axis !== "source") {
    throw new Error(`adapter axis must be source, got: ${manifest.axis}`);
  }
  const ops = manifest.operations as string[] | undefined;
  if (!ops || !ops.includes("enumerate") || !ops.includes("extract")) {
    throw new Error(
      `operations must include enumerate + extract, got: ${JSON.stringify(ops)}`,
    );
  }
});

Deno.test("adapters/sources/runtime-fixtures: every Evidence document schema-validates with example claims", async () => {
  const root = "tests/fixtures/sources/runtime-fixtures";
  if (!(await exists(root))) {
    throw new Error(
      `expected ${root}/ to exist; RFC-27 Change 4.1 landed the golden fixture tree`,
    );
  }
  let seen = 0;
  for await (
    const entry of walk(root, {
      includeDirs: false,
      match: [/expected\/evidence\.yaml$/],
    })
  ) {
    seen++;
    const data = await readYaml(entry.path) as Record<string, unknown>;
    await validateOrThrow("evidence.schema.json", data, entry.path);
    if (data.authority !== "behaviour") {
      throw new Error(
        `${entry.path}: runtime-fixtures emits authority: behaviour by default, got: ${data.authority}`,
      );
    }
    if (data.adapter !== "runtime-fixtures") {
      throw new Error(
        `${entry.path}: adapter field must be runtime-fixtures, got: ${data.adapter}`,
      );
    }
    const claims = data.claims as Array<Record<string, unknown>> | undefined;
    if (!claims || claims.length === 0) {
      throw new Error(`${entry.path}: runtime-fixtures Evidence must carry at least one claim`);
    }
    let exampleClaims = 0;
    for (const claim of claims) {
      if (claim.kind === "example") {
        exampleClaims++;
        const digest = claim["fixture-digest"];
        if (typeof digest !== "string" || !digest.startsWith("sha256:")) {
          throw new Error(
            `${entry.path}: example claim ${claim["claim-id"]} must carry fixture-digest: sha256:<hex>, got: ${digest}`,
          );
        }
      }
    }
    if (exampleClaims === 0) {
      throw new Error(
        `${entry.path}: runtime-fixtures Evidence must carry at least one kind: example claim`,
      );
    }
  }
  if (seen === 0) {
    throw new Error(`no Evidence docs under ${root}/**/expected/evidence.yaml`);
  }
});

Deno.test("adapters/sources/runtime-fixtures: every fusion.yaml schema-validates against slice/fusion.schema.json", async () => {
  const root = "tests/fixtures/sources/runtime-fixtures";
  let seen = 0;
  for await (
    const entry of walk(root, {
      includeDirs: false,
      match: [/expected\/fusion\.yaml$/],
    })
  ) {
    seen++;
    const data = await readYaml(entry.path);
    await validateOrThrow("slice/fusion.schema.json", data, entry.path);
  }
  if (seen === 0) {
    throw new Error(`no fusion.yaml under ${root}/**/expected/fusion.yaml`);
  }
});

Deno.test("adapters/sources/runtime-fixtures: discovery.md names runtime as the bound source key", async () => {
  const root = "tests/fixtures/sources/runtime-fixtures";
  let seen = 0;
  for await (
    const entry of walk(root, {
      includeDirs: false,
      match: [/expected\/discovery\.md$/],
    })
  ) {
    seen++;
    const md = await Deno.readTextFile(entry.path);
    if (!/^### \S+/m.test(md)) {
      throw new Error(`${entry.path}: must contain at least one \`### <candidate>\` block`);
    }
    if (!/sources:\s*\[\s*runtime\s*\]/.test(md)) {
      throw new Error(
        `${entry.path}: candidate block must cite the bound \`runtime\` source key (per RFC-27 §Binding example)`,
      );
    }
  }
  if (seen === 0) {
    throw new Error(`no discovery.md under ${root}/**/expected/discovery.md`);
  }
});
