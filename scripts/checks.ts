// Documentation consistency checks for the Augentic Plugins repository.
// Run via: make checks
// Exit code 0 = all checks pass; non-zero = one or more failures.

import { walk } from "jsr:@std/fs@1/walk";
import { parse as parseYaml } from "jsr:@std/yaml@1";
import { relative, join, dirname, resolve, fromFileUrl } from "jsr:@std/path@1";
import Ajv2020 from "npm:ajv@8/dist/2020.js";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");
const SCHEMA_DIR = join(REPO_ROOT, "schemas");
const RED = "\x1b[0;31m";
const NC = "\x1b[0m";

let errors = 0;

function fail(msg: string): void {
  console.log(`${RED}FAIL${NC}: ${msg}`);
  errors++;
}

async function isUnderSymlink(filepath: string): Promise<boolean> {
  const rel = relative(REPO_ROOT, filepath);
  const parts = rel.split("/");
  let current = REPO_ROOT;
  for (const part of parts.slice(0, -1)) {
    current = join(current, part);
    const info = await Deno.lstat(current);
    if (info.isSymlink) return true;
  }
  const info = await Deno.lstat(filepath);
  return info.isSymlink;
}

// ──────────────────────────────────────────────────────────────
// 1. Markdown link targets exist (relative links only)
// ──────────────────────────────────────────────────────────────

async function checkMarkdownLinks(): Promise<void> {
  const SKIP_DIRS = [/node_modules/, /\.git/, /temp/];
  const LINK_RE = /\[[^\]]*\]\(([^)]+)\)/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const COMMENT_RE = /<!--[\s\S]*?-->/g;

  for await (const entry of walk(REPO_ROOT, {
    exts: [".md"],
    includeDirs: false,
  })) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await isUnderSymlink(entry.path)) continue;

    const relFile = relative(REPO_ROOT, entry.path);
    const parent = dirname(entry.path);
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }

    const stripped = content.replace(FENCE_RE, "").replace(COMMENT_RE, "");

    for (const m of stripped.matchAll(LINK_RE)) {
      const target = m[1];
      if (/^(https?:\/\/|mailto:|#)/.test(target)) continue;
      const path = target.split("#")[0];
      if (!path) continue;
      if (path.startsWith("src/")) continue;
      const resolved = resolve(parent, path);
      try {
        await Deno.stat(resolved);
      } catch {
        fail(`Broken link in ${relFile}: ${target}`);
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 2. No "109-point" claims remain
// ──────────────────────────────────────────────────────────────

async function checkStaleClaims(): Promise<void> {
  const PATTERNS = [/109-point/, /109 items/, /109 Items/];

  for await (const entry of walk(REPO_ROOT, {
    exts: [".md"],
    includeDirs: false,
  })) {
    if (/node_modules|\.git/.test(entry.path)) continue;
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    if (PATTERNS.some((re) => re.test(content))) {
      fail(`Stale '109' claim in ${relative(REPO_ROOT, entry.path)}`);
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 3. Removed workflow surfaces do not reappear
// ──────────────────────────────────────────────────────────────

async function checkRemovedSurfaces(): Promise<void> {
  const STALE_TERMS = [
    "/rt:orchestrator",
    "/omnia:orchestrator",
    "cursor plugin marketplace",
    "claude plugin validate",
    "Provider Capabilities",
  ];
  const SKIP_PATHS = [/\.git/, /temp/, /improvements/, /scripts\/checks\.ts$/];

  for await (const entry of walk(REPO_ROOT, {
    includeDirs: false,
  })) {
    if (SKIP_PATHS.some((re) => re.test(entry.path))) continue;
    const name = entry.name;
    if (
      !name.endsWith(".md") &&
      !name.endsWith(".sh") &&
      !name.endsWith(".ts") &&
      name !== "Makefile"
    ) {
      continue;
    }

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    if (STALE_TERMS.some((term) => content.includes(term))) {
      fail(
        `Stale removed workflow surface in ${relative(REPO_ROOT, entry.path)}`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 4. Schema YAML files validate against JSON Schema
// ──────────────────────────────────────────────────────────────

interface SchemaYaml {
  name: string;
  blueprints: { id: string; requires: string[]; instruction?: string }[];
  build: { requires: string[]; instruction?: string };
}

interface ConfigYaml {
  rules?: Record<string, string>;
}

async function validateSchemaYaml(): Promise<void> {
  const ajv = new Ajv2020({ allErrors: true });

  const schemaSchema = JSON.parse(
    await Deno.readTextFile(join(SCHEMA_DIR, "schema.schema.json")),
  );
  const configSchema = JSON.parse(
    await Deno.readTextFile(join(SCHEMA_DIR, "config.schema.json")),
  );

  const validateSchema = ajv.compile(schemaSchema);
  const validateConfig = ajv.compile(configSchema);

  for await (const entry of walk(SCHEMA_DIR, {
    maxDepth: 2,
    includeDirs: false,
    match: [/schema\.yaml$/],
  })) {
    const rel = relative(REPO_ROOT, entry.path);
    const data = parseYaml(await Deno.readTextFile(entry.path));
    if (!validateSchema(data)) {
      for (const err of validateSchema.errors ?? []) {
        fail(`Schema validation failed: ${rel} — ${err.instancePath} ${err.message}`);
      }
    }
  }

  for await (const entry of walk(SCHEMA_DIR, {
    maxDepth: 2,
    includeDirs: false,
    match: [/config\.yaml$/],
  })) {
    const rel = relative(REPO_ROOT, entry.path);
    const data = parseYaml(await Deno.readTextFile(entry.path));
    if (!validateConfig(data)) {
      for (const err of validateConfig.errors ?? []) {
        fail(`Config validation failed: ${rel} — ${err.instancePath} ${err.message}`);
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 5. Schema referential integrity
//    (blueprint requires, instruction paths, config rule keys)
// ──────────────────────────────────────────────────────────────

async function checkSchemaIntegrity(): Promise<void> {
  for await (const entry of walk(SCHEMA_DIR, {
    maxDepth: 2,
    includeDirs: false,
    match: [/schema\.yaml$/],
  })) {
    const dirPath = dirname(entry.path);
    const name = dirPath.split("/").pop()!;
    const schema = parseYaml(
      await Deno.readTextFile(entry.path),
    ) as SchemaYaml;

    const blueprints = schema.blueprints ?? [];
    const ids = new Set(blueprints.map((bp) => bp.id));

    for (const bp of blueprints) {
      for (const req of bp.requires ?? []) {
        if (!ids.has(req)) {
          fail(
            `Schema integrity: ${name}/schema.yaml: blueprint '${bp.id}' requires undeclared '${req}'`,
          );
        }
      }
    }

    const build = schema.build ?? { requires: [] };
    for (const req of build.requires ?? []) {
      if (!ids.has(req)) {
        fail(
          `Schema integrity: ${name}/schema.yaml: build.requires references undeclared '${req}'`,
        );
      }
    }

    // Cycle detection via Kahn's algorithm
    const inDeg = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const id of ids) {
      inDeg.set(id, 0);
      adj.set(id, []);
    }
    for (const bp of blueprints) {
      for (const req of bp.requires ?? []) {
        if (ids.has(req)) {
          adj.get(req)!.push(bp.id);
          inDeg.set(bp.id, (inDeg.get(bp.id) ?? 0) + 1);
        }
      }
    }
    const queue = [...ids].filter((id) => inDeg.get(id) === 0);
    let visited = 0;
    while (queue.length > 0) {
      const n = queue.shift()!;
      visited++;
      for (const nb of adj.get(n) ?? []) {
        const deg = (inDeg.get(nb) ?? 1) - 1;
        inDeg.set(nb, deg);
        if (deg === 0) queue.push(nb);
      }
    }
    if (visited < ids.size) {
      fail(`Schema integrity: ${name}/schema.yaml: cycle in blueprint requires graph`);
    }

    for (const bp of blueprints) {
      const inst = bp.instruction ?? "";
      if (inst) {
        try {
          await Deno.stat(join(dirPath, inst));
        } catch {
          fail(
            `Schema integrity: ${name}/schema.yaml: blueprint '${bp.id}' instruction not found: ${inst}`,
          );
        }
      }
    }

    const buildInst = build.instruction ?? "";
    if (buildInst) {
      try {
        await Deno.stat(join(dirPath, buildInst));
      } catch {
        fail(
          `Schema integrity: ${name}/schema.yaml: build.instruction not found: ${buildInst}`,
        );
      }
    }

    const configPath = join(dirPath, "config.yaml");
    try {
      const config = parseYaml(
        await Deno.readTextFile(configPath),
      ) as ConfigYaml;
      for (const key of Object.keys(config.rules ?? {})) {
        if (!ids.has(key)) {
          fail(
            `Schema integrity: ${name}/config.yaml: rule key '${key}' does not match any blueprint id`,
          );
        }
      }
    } catch {
      // config.yaml doesn't exist — that's fine
    }
  }
}

// ──────────────────────────────────────────────────────────────
// Run all checks
// ──────────────────────────────────────────────────────────────

await checkMarkdownLinks();
await checkStaleClaims();
await checkRemovedSurfaces();
await validateSchemaYaml();
await checkSchemaIntegrity();

console.log();
if (errors > 0) {
  console.log(`${RED}${errors} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
