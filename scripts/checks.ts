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
    if (await isUnderSymlink(entry.path)) continue;
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
// 3. Schema YAML files validate against JSON Schema
// ──────────────────────────────────────────────────────────────

interface SchemaYaml {
  name: string;
  version?: string;
  description?: string;
  terminology?: Record<string, string>;
  validation?: Record<string, boolean>;
  blueprints: { id: string; requires: string[]; instructions?: string }[];
  build: { requires: string[]; instructions?: string };
  defaults?: { context?: string; rules?: Record<string, string> };
}

async function validateSchemaYaml(): Promise<void> {
  const ajv = new Ajv2020({ allErrors: true });

  const schemaSchema = JSON.parse(
    await Deno.readTextFile(join(SCHEMA_DIR, "schema.schema.json")),
  );

  const validateSchema = ajv.compile(schemaSchema);

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
}

// ──────────────────────────────────────────────────────────────
// 4. Schema referential integrity
//    (blueprint requires, instructions paths, defaults.rules keys)
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
      const inst = bp.instructions ?? "";
      if (inst) {
        try {
          await Deno.stat(join(dirPath, inst));
        } catch {
          fail(
            `Schema integrity: ${name}/schema.yaml: blueprint '${bp.id}' instructions not found: ${inst}`,
          );
        }
      }
    }

    const buildInst = build.instructions ?? "";
    if (buildInst) {
      try {
        await Deno.stat(join(dirPath, buildInst));
      } catch {
        fail(
          `Schema integrity: ${name}/schema.yaml: build.instructions not found: ${buildInst}`,
        );
      }
    }

    for (const key of Object.keys(schema.defaults?.rules ?? {})) {
      if (!ids.has(key)) {
        fail(
          `Schema integrity: ${name}/schema.yaml: defaults.rules key '${key}' does not match any blueprint id`,
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 5. Symlink targets resolve
// ──────────────────────────────────────────────────────────────

async function checkSymlinks(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (const entry of walk(PLUGINS_DIR, {
    includeDirs: true,
    includeFiles: false,
  })) {
    let info: Deno.FileInfo;
    try {
      info = await Deno.lstat(entry.path);
    } catch {
      continue;
    }
    if (!info.isSymlink) continue;

    try {
      await Deno.stat(entry.path);
    } catch {
      fail(`Broken symlink: ${relative(REPO_ROOT, entry.path)}`);
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6. SKILL.md frontmatter validation
// ──────────────────────────────────────────────────────────────

const KNOWN_TOOLS = new Set([
  "Read", "Write", "StrReplace", "Shell", "Grep", "Glob",
  "ReadLints", "WebFetch", "WebSearch", "AskQuestion", "Task",
  "TodoWrite", "SemanticSearch", "EditNotebook", "GenerateImage",
]);

async function validateSkillFrontmatter(): Promise<void> {
  const skillSchema = JSON.parse(
    await Deno.readTextFile(join(SCHEMA_DIR, "skill.schema.json")),
  );
  const ajv = new Ajv2020({ allErrors: true });
  const validate = ajv.compile(skillSchema);

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) {
      fail(`Missing frontmatter: ${rel}`);
      continue;
    }

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Invalid YAML frontmatter: ${rel} — ${msg}`);
      continue;
    }

    if (!validate(fm)) {
      for (const err of validate.errors ?? []) {
        fail(
          `Skill frontmatter: ${rel} — ${err.instancePath} ${err.message}`,
        );
      }
    }

    const dirName = entry.path.split("/").at(-2);
    if (fm.name && fm.name !== dirName) {
      fail(
        `Skill name mismatch: ${rel} — name '${fm.name}' != dir '${dirName}'`,
      );
    }

    const tools = fm["allowed-tools"];
    if (typeof tools === "string") {
      for (const tool of tools.split(",").map((t) => t.trim())) {
        if (!tool) continue;
        if (!KNOWN_TOOLS.has(tool) && !tool.startsWith("mcp__")) {
          fail(`Unknown tool in allowed-tools: ${rel} — '${tool}'`);
        }
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 7. Skill reference link resolution
// ──────────────────────────────────────────────────────────────

async function checkSkillReferences(): Promise<void> {
  const REF_LINK_RE = /\[([^\]]*)\]\((references\/[^)]+|examples\/[^)]+)\)/g;
  const FENCE_RE = /```[\s\S]*?```/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const skillDir = dirname(entry.path);
    const content = await Deno.readTextFile(entry.path);

    const stripped = content.replace(FENCE_RE, "");

    for (const m of stripped.matchAll(REF_LINK_RE)) {
      const refPath = m[2].split("#")[0];
      if (!refPath) continue;
      const resolved = resolve(skillDir, refPath);
      try {
        await Deno.stat(resolved);
      } catch {
        fail(
          `Skill reference missing: ${rel} links to '${refPath}' but it doesn't exist`,
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 8. Skill variable consistency
// ──────────────────────────────────────────────────────────────

async function checkSkillVariables(): Promise<void> {
  const DEF_RE = /^\$([A-Z_][A-Z_0-9]*)\s*=/gm;
  const USE_RE = /\$([A-Z_][A-Z_0-9]*)/g;
  const ARGS_HEADING_RE = /^## (?:Derived )?Arguments/m;
  const CODE_BLOCK_RE = /```text\n([\s\S]*?)```/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;
  const BUILTIN = new Set(["ARGUMENTS", "HOME"]);

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const headingMatch = content.match(ARGS_HEADING_RE);
    if (!headingMatch || headingMatch.index === undefined) continue;
    const headingIdx = headingMatch.index;

    const afterHeading = content.slice(headingIdx + headingMatch[0].length);
    const nextH2 = afterHeading.match(/\n## /);
    const sectionEnd = nextH2
      ? headingIdx + headingMatch[0].length + nextH2.index!
      : content.length;
    const argsSection = content.slice(headingIdx, sectionEnd);

    const defined = new Set<string>();
    const usedInDefs = new Set<string>();

    for (const block of argsSection.matchAll(CODE_BLOCK_RE)) {
      for (const m of block[1].matchAll(DEF_RE)) {
        defined.add(m[1]);
      }
      for (const line of block[1].split("\n")) {
        const eqIdx = line.indexOf("=");
        if (eqIdx < 0) continue;
        const rhs = line.slice(eqIdx + 1);
        for (const m of rhs.matchAll(USE_RE)) {
          if (!BUILTIN.has(m[1])) usedInDefs.add(m[1]);
        }
      }
    }

    if (defined.size === 0) continue;

    const body = content.slice(sectionEnd);
    const bodyNoFences = body.replace(FENCE_RE, "");

    const usedInBody = new Set<string>();
    for (const m of bodyNoFences.matchAll(USE_RE)) {
      if (!BUILTIN.has(m[1])) usedInBody.add(m[1]);
    }

    const bodyStrict = bodyNoFences.replace(INLINE_CODE_RE, "");
    const usedInBodyStrict = new Set<string>();
    for (const m of bodyStrict.matchAll(USE_RE)) {
      if (!BUILTIN.has(m[1])) usedInBodyStrict.add(m[1]);
    }

    for (const v of defined) {
      if (!usedInBody.has(v) && !usedInDefs.has(v)) {
        fail(
          `Unused variable: ${rel} — $${v} defined but never referenced in body`,
        );
      }
    }
    for (const v of usedInBodyStrict) {
      if (!defined.has(v) && !BUILTIN.has(v)) {
        if (/^[A-Z][A-Z_]+$/.test(v)) {
          fail(
            `Undefined variable: ${rel} — $${v} used but not defined in Arguments`,
          );
        }
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 9. Skill directive validation
// ──────────────────────────────────────────────────────────────

async function checkSkillDirectives(): Promise<void> {
  const DIRECTIVE_RE = /<!-- skill: ([a-z][a-z0-9-]*):([a-z][a-z0-9-]*) -->/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  const registry = new Map<string, Set<string>>();
  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    const parts = relative(PLUGINS_DIR, entry.path).split("/");
    if (parts.length >= 4 && parts[1] === "skills") {
      const plugin = parts[0];
      const skill = parts[2];
      if (!registry.has(plugin)) registry.set(plugin, new Set());
      registry.get(plugin)!.add(skill);
    }
  }

  const SKIP_DIRS = [/node_modules/, /\.git/, /temp/, /roadmap/];

  for await (const entry of walk(REPO_ROOT, {
    exts: [".md"],
    includeDirs: false,
  })) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await isUnderSymlink(entry.path)) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    const rel = relative(REPO_ROOT, entry.path);

    const stripped = content.replace(FENCE_RE, "").replace(INLINE_CODE_RE, "");

    for (const m of stripped.matchAll(DIRECTIVE_RE)) {
      const [, plugin, skill] = m;
      if (!registry.has(plugin)) {
        fail(`Invalid skill directive: ${rel} — plugin '${plugin}' not found`);
      } else if (!registry.get(plugin)!.has(skill)) {
        fail(
          `Invalid skill directive: ${rel} — skill '${plugin}:${skill}' not found`,
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 10. Cross-plugin consistency with marketplace.json
// ──────────────────────────────────────────────────────────────

async function checkPluginConsistency(): Promise<void> {
  const manifestPath = join(REPO_ROOT, ".cursor-plugin", "marketplace.json");
  let manifest: {
    plugins: { name: string; source: string }[];
  };
  try {
    manifest = JSON.parse(await Deno.readTextFile(manifestPath));
  } catch {
    fail("Cannot read .cursor-plugin/marketplace.json");
    return;
  }

  const declaredSources = new Set(manifest.plugins.map((p) => p.source));

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  for await (const entry of walk(PLUGINS_DIR, {
    maxDepth: 3,
    match: [/plugin\.json$/],
    includeDirs: false,
  })) {
    const relParts = relative(PLUGINS_DIR, entry.path).split("/");
    if (
      relParts.length === 3 &&
      relParts[1] === ".cursor-plugin" &&
      relParts[2] === "plugin.json"
    ) {
      const pluginDir = relParts[0];
      if (!declaredSources.has(pluginDir)) {
        fail(
          `Plugin '${pluginDir}' has .cursor-plugin/plugin.json but is not in marketplace.json`,
        );
      }
    }
  }

  for (const p of manifest.plugins) {
    const skillsDir = join(PLUGINS_DIR, p.source, "skills");
    try {
      const stat = await Deno.stat(skillsDir);
      if (!stat.isDirectory) {
        fail(`Plugin '${p.name}' has no skills/ directory`);
      }
    } catch {
      fail(
        `Plugin '${p.name}' declared in marketplace.json but skills/ not found`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 11. docs/plugins.md lists every skill on disk (and vice versa)
// ──────────────────────────────────────────────────────────────

async function checkPluginsDocInventory(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const DOCS_PATH = join(REPO_ROOT, "docs", "plugins.md");

  const skillsOnDisk = new Set<string>();
  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    const parts = relative(PLUGINS_DIR, entry.path).split("/");
    if (parts.length >= 4 && parts[1] === "skills") {
      skillsOnDisk.add(`${parts[0]}/${parts[2]}`);
    }
  }

  let content: string;
  try {
    content = await Deno.readTextFile(DOCS_PATH);
  } catch {
    fail("Cannot read docs/plugins.md");
    return;
  }

  const documented = new Set<string>();
  const SECTION_RE = /^## .+\(`plugins\/([^/]+)\/`\)/;
  const SKILL_RE = /^- \*\*([a-z][a-z0-9-]*)\*\*/;

  let currentPlugin: string | null = null;
  for (const line of content.split("\n")) {
    const sectionMatch = line.match(SECTION_RE);
    if (sectionMatch) {
      currentPlugin = sectionMatch[1];
      continue;
    }
    if (/^## /.test(line)) {
      currentPlugin = null;
      continue;
    }
    if (currentPlugin) {
      const skillMatch = line.match(SKILL_RE);
      if (skillMatch) {
        documented.add(`${currentPlugin}/${skillMatch[1]}`);
      }
    }
  }

  for (const skill of skillsOnDisk) {
    if (!documented.has(skill)) {
      fail(`Skill '${skill}' exists on disk but is not listed in docs/plugins.md`);
    }
  }
  for (const skill of documented) {
    if (!skillsOnDisk.has(skill)) {
      fail(`Skill '${skill}' listed in docs/plugins.md but not found on disk`);
    }
  }
}

// ──────────────────────────────────────────────────────────────
// Run all checks
// ──────────────────────────────────────────────────────────────

await Promise.all([
  checkMarkdownLinks(),
  checkStaleClaims(),
  checkSymlinks(),
]);
await Promise.all([
  validateSchemaYaml(),
  checkSchemaIntegrity(),
]);
await Promise.all([
  validateSkillFrontmatter(),
  checkSkillReferences(),
  checkSkillVariables(),
  checkSkillDirectives(),
  checkPluginConsistency(),
  checkPluginsDocInventory(),
]);

console.log();
if (errors > 0) {
  console.log(`${RED}${errors} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
