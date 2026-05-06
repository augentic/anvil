// Documentation consistency checks for the Augentic Plugins repository.
// Run via: make checks
// Exit code 0 = all checks pass; non-zero = one or more failures.

import { walk } from "jsr:@std/fs@1/walk";
import { parse as parseYaml } from "jsr:@std/yaml@1";
import { relative, join, dirname, resolve, fromFileUrl } from "jsr:@std/path@1";
import Ajv2020 from "npm:ajv@8/dist/2020.js";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");
const CAPABILITIES_DIR = join(REPO_ROOT, "capabilities");
const CURSOR_SCHEMA_DIR = join(REPO_ROOT, ".cursor", "schemas");
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
// 3. Capability manifests validate against capability.schema.json
// ──────────────────────────────────────────────────────────────

interface PipelineEntry {
  id: string;
  brief: string;
}

interface CapabilityYaml {
  name: string;
  version?: number;
  description?: string;
  pipeline: {
    plan?: PipelineEntry[];
    define: PipelineEntry[];
    build: PipelineEntry[];
    merge: PipelineEntry[];
  };
}

async function validateCapabilityYaml(): Promise<void> {
  const ajv = new Ajv2020({ allErrors: true });

  const capabilitySchema = JSON.parse(
    await Deno.readTextFile(join(CAPABILITIES_DIR, "capability.schema.json")),
  );

  const validate = ajv.compile(capabilitySchema);

  for await (const entry of walk(CAPABILITIES_DIR, {
    maxDepth: 2,
    includeDirs: false,
    match: [/capability\.yaml$/],
  })) {
    const rel = relative(REPO_ROOT, entry.path);
    const data = parseYaml(await Deno.readTextFile(entry.path));
    if (!validate(data)) {
      for (const err of validate.errors ?? []) {
        fail(`Capability validation failed: ${rel} — ${err.instancePath} ${err.message}`);
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 4. Capability manifest referential integrity
//    (pipeline brief paths, frontmatter needs references, id uniqueness)
// ──────────────────────────────────────────────────────────────

async function parseBriefFrontmatter(
  briefPath: string,
): Promise<Record<string, unknown> | null> {
  let content: string;
  try {
    content = await Deno.readTextFile(briefPath);
  } catch {
    return null;
  }
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) return null;
  try {
    return parseYaml(fmMatch[1]) as Record<string, unknown>;
  } catch {
    return null;
  }
}

async function checkCapabilityIntegrity(): Promise<void> {
  for await (const entry of walk(CAPABILITIES_DIR, {
    maxDepth: 2,
    includeDirs: false,
    match: [/capability\.yaml$/],
  })) {
    const dirPath = dirname(entry.path);
    const name = dirPath.split("/").pop()!;
    const manifest = parseYaml(
      await Deno.readTextFile(entry.path),
    ) as CapabilityYaml;

    const pipeline = manifest.pipeline;
    if (!pipeline) continue;

    // Include `plan` while it remains transitionally permitted by
    // capability.schema.json (see RFC-13 §Phase 1.5). Phase 3.11 drops
    // the property and this entry collapses back to the slice phases.
    const allEntries: PipelineEntry[] = [
      ...(pipeline.plan ?? []),
      ...(pipeline.define ?? []),
      ...(pipeline.build ?? []),
      ...(pipeline.merge ?? []),
    ];

    const ids = new Set<string>();
    for (const pe of allEntries) {
      if (ids.has(pe.id)) {
        fail(
          `Capability integrity: ${name}/capability.yaml: duplicate pipeline entry id '${pe.id}'`,
        );
      }
      ids.add(pe.id);
    }

    for (const pe of allEntries) {
      try {
        await Deno.stat(join(dirPath, pe.brief));
      } catch {
        fail(
          `Capability integrity: ${name}/capability.yaml: brief not found for '${pe.id}': ${pe.brief}`,
        );
        continue;
      }

      const fm = await parseBriefFrontmatter(join(dirPath, pe.brief));
      if (!fm) {
        fail(
          `Capability integrity: ${name}/capability.yaml: brief '${pe.id}' has no valid frontmatter: ${pe.brief}`,
        );
        continue;
      }

      if (fm.id !== pe.id) {
        fail(
          `Capability integrity: ${name}/capability.yaml: pipeline id '${pe.id}' does not match brief frontmatter id '${fm.id}'`,
        );
      }

      const needs = fm.needs as string[] | undefined;
      if (needs) {
        for (const dep of needs) {
          if (!ids.has(dep)) {
            fail(
              `Capability integrity: ${name}/capability.yaml: brief '${pe.id}' needs undeclared '${dep}'`,
            );
          }
        }
      }
    }

    // Cycle detection via Kahn's algorithm on needs graph
    const inDeg = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const id of ids) {
      inDeg.set(id, 0);
      adj.set(id, []);
    }
    for (const pe of allEntries) {
      const fm = await parseBriefFrontmatter(join(dirPath, pe.brief));
      const needs = (fm?.needs as string[] | undefined) ?? [];
      for (const dep of needs) {
        if (ids.has(dep)) {
          adj.get(dep)!.push(pe.id);
          inDeg.set(pe.id, (inDeg.get(pe.id) ?? 0) + 1);
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
      fail(
        `Capability integrity: ${name}/capability.yaml: cycle in brief needs graph`,
      );
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
    includeFiles: true,
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
    await Deno.readTextFile(join(CURSOR_SCHEMA_DIR, "skill.schema.json")),
  );
  const ajv = new Ajv2020({ allErrors: true });
  const validate = ajv.compile(skillSchema);

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const NAME_RE = /^[a-z][a-z0-9-]*$/;

  // Plugin-directory → required `name:` prefix. The default is `<dir>-`. The
  // `spec` directory is overridden to `specify-` because RFC-10 §A.1 keeps the
  // operator-facing product name (`specify`) in the discovery namespace even
  // though the plugin's directory and slash-command prefix are `spec`.
  const PREFIX_OVERRIDES: Record<string, string> = {
    spec: "specify",
  };

  // Track names for the global-uniqueness check (RFC-10 §A.1).
  const namesByValue = new Map<string, string[]>();

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

    // Plugin directory is the path component immediately under plugins/.
    const relParts = relative(PLUGINS_DIR, entry.path).split("/");
    const pluginDir = relParts[0];

    const name = fm.name;
    if (typeof name !== "string") {
      fail(`Missing or non-string skill name: ${rel}`);
    } else {
      if (!NAME_RE.test(name)) {
        fail(
          `Invalid skill name syntax: ${rel} — '${name}' must match /^[a-z][a-z0-9-]*$/`,
        );
      }
      const prefixBase = PREFIX_OVERRIDES[pluginDir] ?? pluginDir;
      const requiredPrefix = `${prefixBase}-`;
      if (!name.startsWith(requiredPrefix)) {
        fail(
          `Skill name missing plugin prefix: ${rel} — '${name}' must start with '${requiredPrefix}'`,
        );
      }
      const seen = namesByValue.get(name) ?? [];
      seen.push(rel);
      namesByValue.set(name, seen);
    }

    const tools = fm["allowed-tools"];
    if (typeof tools === "string") {
      for (const tool of tools.split(/\s+/).map((t) => t.trim())) {
        if (!tool) continue;
        if (!KNOWN_TOOLS.has(tool) && !tool.startsWith("mcp__")) {
          fail(`Unknown tool in allowed-tools: ${rel} — '${tool}'`);
        }
      }
    }
  }

  for (const [name, paths] of namesByValue) {
    if (paths.length > 1) {
      fail(
        `Duplicate skill name '${name}' across SKILL.md files: ${
          paths.join(", ")
        }`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6b. SKILL.md body line-count ceiling (RFC-10 §D)
// ──────────────────────────────────────────────────────────────

async function checkSkillBodyLineCount(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const MAX_BODY_LINES = 500;

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;

    const body = content.slice(fmMatch[0].length);
    const lines = body.split("\n");
    // Drop leading separator newline and trailing terminating newline so the
    // count matches what an editor displays after the closing `---`.
    if (lines.length > 0 && lines[0] === "") lines.shift();
    if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
    const lineCount = lines.length;

    if (lineCount > MAX_BODY_LINES) {
      fail(
        `Skill body too long: ${rel} — ${lineCount} body lines (limit ${MAX_BODY_LINES})`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6c. SKILL.md description length ceiling (RFC-10 §D)
// ──────────────────────────────────────────────────────────────

async function checkSkillDescriptionLength(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const MAX_DESCRIPTION_CHARS = 1024;

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch {
      continue;
    }

    const description = fm.description;
    if (typeof description !== "string") continue;

    if (description.length > MAX_DESCRIPTION_CHARS) {
      fail(
        `Skill description too long: ${rel} — ${description.length} chars (limit ${MAX_DESCRIPTION_CHARS})`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6d. SKILL.md argument-hint shape (RFC-10 §A.3, §D)
// ──────────────────────────────────────────────────────────────

async function checkSkillArgumentHint(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const FORBIDDEN: { token: string; reason: string }[] = [
    { token: "?", reason: "trailing optional marker" },
    { token: "--", reason: "flag dashes" },
    { token: "|", reason: "alternative-value pipe" },
  ];

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch {
      continue;
    }

    const hint = fm["argument-hint"];
    if (hint === undefined || hint === null) continue;
    if (typeof hint !== "string") {
      fail(`Invalid argument-hint type in ${rel}: must be a string`);
      continue;
    }

    for (const { token, reason } of FORBIDDEN) {
      if (hint.includes(token)) {
        fail(
          `Invalid argument-hint in ${rel}: '${hint}' contains forbidden ${reason} '${token}'`,
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6e. SKILL.md frontmatter must not declare `license` (RFC-10 §A.4, §D)
// ──────────────────────────────────────────────────────────────

async function checkSkillNoLicense(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (const entry of walk(PLUGINS_DIR, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch {
      continue;
    }

    if ("license" in fm) {
      fail(
        `Forbidden 'license' key in SKILL.md frontmatter: ${rel}`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6f. Retired slash commands must not appear in active prose (RFC-10 §D)
// ──────────────────────────────────────────────────────────────

async function checkRetiredSlashCommands(): Promise<void> {
  const RETIRED_SLASH_ALLOWLIST = new Set<string>([]);

  const RETIRED_PATTERNS = [
    "/plan:sow-writer",
    "/rt:git-cloner",
    "/contracts:writer",
    "/contracts:validator",
    "/contracts:importer",
    "/contracts:management",
    "/interfaces:openapi",
    "/interfaces:asyncapi",
    "/interfaces:json-schema",
  ];

  const SCAN_ROOTS = [
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "capabilities"),
    join(REPO_ROOT, ".cursor"),
  ];

  const scanFile = async (path: string): Promise<void> => {
    const rel = relative(REPO_ROOT, path);
    if (rel.startsWith("rfcs/archive/")) return;
    if (RETIRED_SLASH_ALLOWLIST.has(rel)) return;

    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      return;
    }

    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      for (const pattern of RETIRED_PATTERNS) {
        if (lines[i].includes(pattern)) {
          fail(
            `Retired slash command in ${rel}:${
              i + 1
            } -- '${pattern}' (line: ${lines[i].trim()})`,
          );
        }
      }
    }
  };

  for (const root of SCAN_ROOTS) {
    let exists = true;
    try {
      await Deno.stat(root);
    } catch {
      exists = false;
    }
    if (!exists) continue;

    for await (const entry of walk(root, {
      exts: [".md"],
      includeDirs: false,
    })) {
      if (await isUnderSymlink(entry.path)) continue;
      await scanFile(entry.path);
    }
  }

  // Standalone files at the repo root.
  for (const fname of ["AGENTS.md", "README.md"]) {
    const fpath = join(REPO_ROOT, fname);
    try {
      await Deno.stat(fpath);
    } catch {
      continue;
    }
    await scanFile(fpath);
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

  const SKIP_DIRS = [/node_modules/, /\.git/, /temp/, /rfcs/];

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
// 11. Retired CLI verbs do not appear in skills or docs
//     (see docs/explanation/migrating-cli-v1.md for the rename map)
// ──────────────────────────────────────────────────────────────

interface DenyPattern {
  pattern: RegExp;
  hint: string;
}

async function checkRetiredCliVerbs(): Promise<void> {
  // Files that intentionally reference the old verbs (rename map, narrative
  // explanation of the cleanup). These pages are exempt from the deny list.
  const ALLOWLIST = new Set<string>([
    "docs/explanation/migrating-cli-v1.md",
    "docs/reference/cli/change.md",
    "docs/reference/cli/initiative.md",
    "docs/reference/cli/registry.md",
    "docs/reference/cli/status.md",
  ]);

  const PATTERNS: DenyPattern[] = [
    {
      pattern: /\bspecify validate /,
      hint: "use `specify change validate <name>`",
    },
    {
      pattern: /\bspecify merge /,
      hint: "use `specify change merge run <name>`",
    },
    {
      pattern: /\bspecify spec /,
      hint:
        "use `specify change merge {preview, conflict-check}` (the `spec` group is retired)",
    },
    {
      pattern: /\bspecify task /,
      hint:
        "use `specify change task {progress, mark}` (the `task` group is retired)",
    },
    {
      pattern: /\bspecify initiative brief\b/,
      hint: "use `specify initiative {init, show}`",
    },
    {
      pattern: /\bspecify initiative registry\b/,
      hint: "use `specify registry {show, validate}`",
    },
    {
      pattern: /\bspecify change phase-outcome\b/,
      hint: "use `specify change outcome set ...`",
    },
    {
      pattern: /\bspecify change journal-append\b/,
      hint: "use `specify change journal append ...`",
    },
    {
      // The bare `specify change outcome <name>` form (no `set`/`show` after
      // `outcome`) is ambiguous after the cleanup. Reads must use `outcome
      // show`; writes must use `outcome set`.
      pattern: /\bspecify change outcome (?!set\b|show\b)/,
      hint:
        "use `specify change outcome show <name>` to read or `specify change outcome set <name> <phase> <outcome> ...` to write",
    },
  ];

  const SCAN_ROOTS = [
    join(REPO_ROOT, "plugins", "spec", "skills"),
    join(REPO_ROOT, "plugins", "change", "skills"),
    join(REPO_ROOT, "docs"),
  ];

  for (const root of SCAN_ROOTS) {
    for await (const entry of walk(root, {
      exts: [".md"],
      includeDirs: false,
    })) {
      if (await isUnderSymlink(entry.path)) continue;
      const rel = relative(REPO_ROOT, entry.path);
      if (ALLOWLIST.has(rel)) continue;

      let content: string;
      try {
        content = await Deno.readTextFile(entry.path);
      } catch {
        continue;
      }

      const lines = content.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        for (const { pattern, hint } of PATTERNS) {
          if (pattern.test(line)) {
            fail(
              `Retired CLI verb in ${rel}:${i + 1} -- ${
                line.trim()
              } -- ${hint}`,
            );
          }
        }
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 12. Instruction files contain output location preamble
// ──────────────────────────────────────────────────────────────

async function checkInstructionPreambles(): Promise<void> {
  const OUTPUT_LOCATION_RE = /^> \*\*Output location\*\*: `\.specify\/changes\//m;

  for await (const entry of walk(CAPABILITIES_DIR, {
    maxDepth: 3,
    includeDirs: false,
    match: [/instructions\/[a-z]+\.md$/],
  })) {
    const rel = relative(REPO_ROOT, entry.path);
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    if (!OUTPUT_LOCATION_RE.test(content)) {
      fail(
        `Missing output location preamble: ${rel} — instruction files must declare output location to prevent cross-plugin path contamination`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 13. Retired plan-schema fields do not appear in fixture YAML
//     (`affects` was removed from the plan schema; delta targeting
//     is now description-driven — see RFC-9 §1A.)
// ──────────────────────────────────────────────────────────────

async function checkRetiredAffectsField(): Promise<void> {
  // Matches plan/execute fixture YAML, including the suffix variants
  // used to pin lifecycle state (`plan.yaml.before`, `plan.yaml.after`,
  // `plan.yaml.after-crash`, `journal.yaml.after`, etc.).
  const FIXTURE_NAME_RE = /\.ya?ml(\.[a-z-]+)?$/;
  const AFFECTS_RE = /^\s*affects:/;

  // RFC-13 §3.9 moved the plan/execute fixtures from `plugins/spec/skills/`
  // to `plugins/change/skills/`. Both locations are scanned here so the
  // retired-affects check tolerates partial-rollback states (e.g. a
  // checkout pre-3.9 still has the spec-plugin paths) without losing
  // coverage on the post-3.9 layout.
  const FIXTURE_ROOTS = [
    join(REPO_ROOT, "plugins", "change", "skills", "execute", "fixtures"),
    join(REPO_ROOT, "plugins", "change", "skills", "plan", "fixtures"),
    join(REPO_ROOT, "plugins", "spec", "skills", "execute", "fixtures"),
    join(REPO_ROOT, "plugins", "spec", "skills", "plan", "fixtures"),
  ];

  for (const root of FIXTURE_ROOTS) {
    let rootExists = true;
    try {
      await Deno.stat(root);
    } catch {
      rootExists = false;
    }
    if (!rootExists) continue;

    for await (const entry of walk(root, { includeDirs: false })) {
      if (!FIXTURE_NAME_RE.test(entry.path)) continue;
      if (await isUnderSymlink(entry.path)) continue;

      let content: string;
      try {
        content = await Deno.readTextFile(entry.path);
      } catch {
        continue;
      }

      const rel = relative(REPO_ROOT, entry.path);
      const lines = content.split("\n");
      for (let i = 0; i < lines.length; i++) {
        if (AFFECTS_RE.test(lines[i])) {
          fail(
            `Retired schema field in ${rel}:${i + 1} -- ${
              lines[i].trim()
            } -- the 'affects' field was removed from the plan schema; use description-driven delta targeting`,
          );
        }
      }
    }
  }
}

async function checkV1LayoutPaths(): Promise<void> {
  // The v2 layout (specify-cli 0.2.0) moved operator-facing platform
  // artifacts from `.specify/` to the repo root. Doc/skill prose that
  // still references the v1 paths is drift; this check pins the new
  // shape so doc edits cannot regress quietly. Allowed exceptions:
  //
  // - rfcs/archive/* — historical RFCs carry v2-layout banners and
  //   intentionally retain their original paths.
  // - rfcs/roadmap.md — narrative may still reference legacy shapes.
  // - docs/how-to/migrate-to-v2-layout.md, docs/reference/cli/migrate.md,
  //   docs/explanation/whats-new.md, docs/explanation/decision-log.md,
  //   docs/appendices/{glossary,troubleshooting}.md,
  //   docs/reference/directory-layout.md — these documents *describe*
  //   the migration and so must mention both the old and new paths.
  // - plugins/spec/skills/init/fixtures/v2-layout-migration/* — the
  //   illustrative fixture for the migration.
  // - The CLI's own legacy-layout error message (in scripts that quote it).
  const FORBIDDEN_PATTERNS: RegExp[] = [
    /\.specify\/registry\.yaml/,
    /\.specify\/plan\.yaml/,
    /\.specify\/initiative\.md/,
    /\.specify\/contracts\b/,
  ];

  const ALLOWED_PREFIXES = [
    "rfcs/archive/",
    "rfcs/roadmap.md",
    "docs/how-to/migrate-to-v2-layout.md",
    "docs/reference/cli/migrate.md",
    "docs/reference/directory-layout.md",
    "docs/explanation/whats-new.md",
    "docs/explanation/decision-log.md",
    "docs/appendices/glossary.md",
    "docs/appendices/troubleshooting.md",
    "plugins/spec/skills/init/fixtures/v2-layout-migration/",
    "scripts/checks.ts",
  ];

  const SCAN_ROOTS = [
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "capabilities"),
  ];
  const SCAN_FILES = [
    join(REPO_ROOT, "README.md"),
    join(REPO_ROOT, "AGENTS.md"),
    join(REPO_ROOT, ".cursor", "rules", "project.mdc"),
  ];

  const targets: string[] = [];
  for (const root of SCAN_ROOTS) {
    let exists = true;
    try {
      await Deno.stat(root);
    } catch {
      exists = false;
    }
    if (!exists) continue;
    for await (const entry of walk(root, {
      includeDirs: false,
      exts: [".md", ".mdx", ".mdc", ".yaml", ".yml", ".json", ".toml"],
    })) {
      if (await isUnderSymlink(entry.path)) continue;
      targets.push(entry.path);
    }
  }
  for (const path of SCAN_FILES) {
    try {
      await Deno.stat(path);
      targets.push(path);
    } catch {
      // File doesn't exist — skip.
    }
  }

  for (const path of targets) {
    const rel = relative(REPO_ROOT, path);
    if (ALLOWED_PREFIXES.some((p) => rel.startsWith(p))) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      continue;
    }
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      for (const pattern of FORBIDDEN_PATTERNS) {
        if (pattern.test(lines[i])) {
          fail(
            `v1-layout path in ${rel}:${i + 1} -- ${
              lines[i].trim()
            } -- the v2 layout moved this artifact to the repo root; ` +
              `update the reference or add the file to the allow-list in scripts/checks.ts ` +
              `(see docs/how-to/migrate-to-v2-layout.md)`,
          );
          break;
        }
      }
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
  validateCapabilityYaml(),
  checkCapabilityIntegrity(),
  checkInstructionPreambles(),
  checkRetiredCliVerbs(),
  checkRetiredAffectsField(),
  checkV1LayoutPaths(),
]);
await Promise.all([
  validateSkillFrontmatter(),
  checkSkillBodyLineCount(),
  checkSkillDescriptionLength(),
  checkSkillArgumentHint(),
  checkSkillNoLicense(),
  checkSkillReferences(),
  checkSkillVariables(),
  checkSkillDirectives(),
  checkPluginConsistency(),
  checkRetiredSlashCommands(),
]);

console.log();
if (errors > 0) {
  console.log(`${RED}${errors} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
