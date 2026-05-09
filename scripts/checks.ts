// Documentation consistency checks for the Augentic Plugins repository.
// Run via: make checks
// Exit code 0 = all checks pass; non-zero = one or more failures.

import { walk } from "jsr:@std/fs@1/walk";
import { parse as parseYaml } from "jsr:@std/yaml@1";
import { dirname, fromFileUrl, join, relative, resolve } from "jsr:@std/path@1";
import Ajv2020Module from "npm:ajv@8/dist/2020.js";

const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");
const CAPABILITIES_DIR = join(REPO_ROOT, "capabilities");
const CURSOR_SCHEMA_DIR = join(REPO_ROOT, ".cursor", "schemas");
const RED = "\x1b[0;31m";
const NC = "\x1b[0m";
const MAX_BODY_LINES = 500;
const CRITICAL_PATH_MIN_LINES = 150;
const CRITICAL_PATH_HEADING = "## Critical Path (Quick Reference)";
type AjvValidationError = {
  instancePath?: string;
  message?: string;
  keyword?: string;
  params?: Record<string, unknown>;
};

const Ajv2020 = Ajv2020Module as unknown as {
  new (opts: { allErrors?: boolean }): {
    compile(schema: unknown): ((data: unknown) => boolean) & {
      errors?: AjvValidationError[];
    };
  };
};

let errors = 0;

function fail(msg: string): void {
  console.log(`${RED}FAIL${NC}: ${msg}`);
  errors++;
}

async function underSymlink(filepath: string): Promise<boolean> {
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

  for await (
    const entry of walk(REPO_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await underSymlink(entry.path)) continue;

    const relFile = relative(REPO_ROOT, entry.path);
    const parent = dirname(entry.path);
    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }

    const stripped = stripHtmlComments(content.replace(FENCE_RE, ""));

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

function stripHtmlComments(content: string): string {
  let stripped = "";
  let cursor = 0;

  while (cursor < content.length) {
    const start = content.indexOf("<!--", cursor);
    if (start === -1) {
      stripped += content.slice(cursor);
      break;
    }

    stripped += content.slice(cursor, start);
    const end = content.indexOf("-->", start + "<!--".length);
    if (end === -1) break;
    cursor = end + "-->".length;
  }

  return stripped;
}

function skillBodyLines(content: string): string[] | null {
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) return null;

  const lines = content.slice(fmMatch[0].length).split("\n");
  // Drop leading separator newline and trailing terminating newline so the
  // count matches what an editor displays after the closing `---`.
  if (lines.length > 0 && lines[0] === "") lines.shift();
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

// ──────────────────────────────────────────────────────────────
// 2. No "109-point" claims remain
// ──────────────────────────────────────────────────────────────

async function checkStaleClaims(): Promise<void> {
  const PATTERNS = [/109-point/, /109 items/, /109 Items/];

  for await (
    const entry of walk(REPO_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (/node_modules|\.git/.test(entry.path)) continue;
    if (await underSymlink(entry.path)) continue;
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

  for await (
    const entry of walk(CAPABILITIES_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/capability\.yaml$/],
    })
  ) {
    const rel = relative(REPO_ROOT, entry.path);
    const data = parseYaml(await Deno.readTextFile(entry.path));
    if (!validate(data)) {
      for (const err of validate.errors ?? []) {
        fail(
          `Capability validation failed: ${rel} — ${err.instancePath} ${err.message}`,
        );
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
  for await (
    const entry of walk(CAPABILITIES_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/capability\.yaml$/],
    })
  ) {
    const dirPath = dirname(entry.path);
    const name = dirPath.split("/").pop()!;
    const manifest = parseYaml(
      await Deno.readTextFile(entry.path),
    ) as CapabilityYaml;

    const pipeline = manifest.pipeline;
    if (!pipeline) continue;

    // Post-RFC-13 §3.11 the manifest carries only the slice phases
    // (define, build, merge); planning is owned by the change-planning
    // skill and `pipeline.plan` is rejected by `capability.schema.json`.
    const allEntries: PipelineEntry[] = [
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

  for await (
    const entry of walk(PLUGINS_DIR, {
      includeDirs: true,
      includeFiles: true,
    })
  ) {
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
  "Read",
  "Write",
  "StrReplace",
  "Shell",
  "Grep",
  "Glob",
  "ReadLints",
  "WebFetch",
  "WebSearch",
  "AskQuestion",
  "Task",
  "TodoWrite",
  "SemanticSearch",
  "EditNotebook",
  "GenerateImage",
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

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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

async function checkBodyLineCount(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const lines = skillBodyLines(content);
    if (!lines) continue;
    const lineCount = lines.length;

    if (lineCount > MAX_BODY_LINES) {
      fail(
        `Skill body too long: ${rel} — ${lineCount} body lines (limit ${MAX_BODY_LINES})`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6c. Long SKILL.md bodies must include a Critical Path block
// ──────────────────────────────────────────────────────────────

async function checkCriticalPath(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const LIST_ITEM_RE = /^(?:\d+\.|-)\s+\S/;

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const lines = skillBodyLines(content);
    if (!lines || lines.length < CRITICAL_PATH_MIN_LINES) continue;

    const headingIndex = lines.findIndex((line) =>
      line.trim() === CRITICAL_PATH_HEADING
    );
    if (headingIndex < 0) {
      fail(
        `Missing Critical Path: ${rel} — ${lines.length} body lines requires '${CRITICAL_PATH_HEADING}'`,
      );
      continue;
    }

    const nextH2Offset = lines.slice(headingIndex + 1).findIndex((line) =>
      line.startsWith("## ")
    );
    const sectionLines = nextH2Offset >= 0
      ? lines.slice(headingIndex + 1, headingIndex + 1 + nextH2Offset)
      : lines.slice(headingIndex + 1);
    let itemCount = 0;
    let inCriticalPathList = false;
    for (const line of sectionLines) {
      if (line.trim() === "") {
        if (inCriticalPathList) break;
        continue;
      }
      if (LIST_ITEM_RE.test(line)) {
        inCriticalPathList = true;
        itemCount++;
      }
    }

    if (itemCount < 5 || itemCount > 7) {
      fail(
        `Invalid Critical Path: ${rel} — expected 5-7 bullets or numbered items, found ${itemCount}`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6d. SKILL.md description length ceiling (RFC-10 §D)
// ──────────────────────────────────────────────────────────────

async function checkDescriptionLength(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const MAX_DESCRIPTION_CHARS = 1024;

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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
// 6e. SKILL.md argument-hint shape (RFC-10 §A.3, §D)
// ──────────────────────────────────────────────────────────────

async function checkArgumentHint(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const FORBIDDEN: { token: string; reason: string }[] = [
    { token: "?", reason: "trailing optional marker" },
    { token: "--", reason: "flag dashes" },
    { token: "|", reason: "alternative-value pipe" },
  ];

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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
// 6f. Slash-skill invocations use positional arguments (Claude Skills parity)
// ──────────────────────────────────────────────────────────────

async function checkInvocationPositionals(): Promise<void> {
  const SCAN_ROOTS = [
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, "capabilities"),
  ];
  const SCAN_FILES = [
    join(REPO_ROOT, "README.md"),
    join(REPO_ROOT, "AGENTS.md"),
    join(REPO_ROOT, "rfcs", "roadmap.md"),
    join(REPO_ROOT, ".cursor", "rules", "project.mdc"),
  ];
  const SKILL_TOKEN_RE = /\/[a-z][a-z0-9-]*:[a-z][a-z0-9-]*/;
  const FLAG_TOKEN_RE = /--[a-z][a-z0-9-]*/;

  const targets: string[] = [];
  for (const file of SCAN_FILES) {
    try {
      const stat = await Deno.stat(file);
      if (stat.isFile) targets.push(file);
    } catch {
      // Optional top-level files may not exist in downstream checkouts.
    }
  }
  for (const root of SCAN_ROOTS) {
    for await (
      const entry of walk(root, {
        exts: [".md", ".mdc"],
        includeDirs: false,
      })
    ) {
      if (await underSymlink(entry.path)) continue;
      targets.push(entry.path);
    }
  }

  for (const path of targets) {
    const rel = relative(REPO_ROOT, path);
    const content = await Deno.readTextFile(path);
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      let logical = lines[i];
      let end = i;

      // Fenced examples often wrap slash invocations with backslashes and
      // indented continuation rows. Normalize a short logical invocation so
      // `[--flag]` on the next row cannot hide from the check.
      for (let j = i + 1; j < Math.min(lines.length, i + 8); j++) {
        const previousContinues = logical.trimEnd().endsWith("\\");
        const nextIsIndented = /^[ \t]+/.test(lines[j]);
        if (!previousContinues && !nextIsIndented) break;
        logical += "\n" + lines[j];
        end = j;
        if (!lines[j].trimEnd().endsWith("\\") && !nextIsIndented) break;
      }

      const scanLogical = logical.replace(/\]\([^)]+\)/g, "]");
      const skillMatch = SKILL_TOKEN_RE.exec(scanLogical);
      const flagMatch = FLAG_TOKEN_RE.exec(scanLogical);
      if (
        skillMatch && flagMatch &&
        flagMatch.index > skillMatch.index + skillMatch[0].length &&
        !/\b(specify|cargo|gh|git|deno|npm|pnpm|yarn)\s/.test(
          scanLogical.slice(
            skillMatch.index + skillMatch[0].length,
            flagMatch.index,
          ),
        )
      ) {
        fail(
          `Slash skill invocation uses flag-style arguments in ${rel}:${
            i + 1
          }` +
            (end > i ? `-${end + 1}` : "") +
            " — use positional skill arguments; reserve --flags for underlying CLI commands",
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 6g. SKILL.md frontmatter must not declare `license` (RFC-10 §A.4, §D)
// ──────────────────────────────────────────────────────────────

async function checkNoLicense(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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
// 6h. Retired slash commands must not appear in active prose (RFC-10 §D)
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
            `Retired slash command in ${rel}:${i + 1} -- '${pattern}' (line: ${
              lines[i].trim()
            })`,
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

    for await (
      const entry of walk(root, {
        exts: [".md"],
        includeDirs: false,
      })
    ) {
      if (await underSymlink(entry.path)) continue;
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

async function checkReferences(): Promise<void> {
  const REF_LINK_RE = /\[([^\]]*)\]\((references\/[^)]+|examples\/[^)]+)\)/g;
  const FENCE_RE = /```[\s\S]*?```/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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

async function checkVariables(): Promise<void> {
  const DEF_RE = /^\$([A-Z_][A-Z_0-9]*)\s*=/gm;
  const USE_RE = /\$([A-Z_][A-Z_0-9]*)/g;
  const ARGS_HEADING_RE = /^## (?:Derived )?Arguments/m;
  const CODE_BLOCK_RE = /```text\n([\s\S]*?)```/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;
  const BUILTIN = new Set(["ARGUMENTS", "HOME"]);

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
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

async function checkDirectives(): Promise<void> {
  const DIRECTIVE_RE = /<!-- skill: ([a-z][a-z0-9-]*):([a-z][a-z0-9-]*) -->/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  const registry = new Map<string, Set<string>>();
  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    const parts = relative(PLUGINS_DIR, entry.path).split("/");
    if (parts.length >= 4 && parts[1] === "skills") {
      const plugin = parts[0];
      const skill = parts[2];
      if (!registry.has(plugin)) registry.set(plugin, new Set());
      registry.get(plugin)!.add(skill);
    }
  }

  const SKIP_DIRS = [/node_modules/, /\.git/, /temp/, /rfcs/];

  for await (
    const entry of walk(REPO_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (SKIP_DIRS.some((re) => re.test(entry.path))) continue;
    if (await underSymlink(entry.path)) continue;

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
  for await (
    const entry of walk(PLUGINS_DIR, {
      maxDepth: 3,
      match: [/plugin\.json$/],
      includeDirs: false,
    })
  ) {
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
    const pluginDir = join(PLUGINS_DIR, p.source);
    const skillsDir = join(pluginDir, "skills");
    let hasSkillsDir = false;
    try {
      const stat = await Deno.stat(skillsDir);
      if (!stat.isDirectory) {
        fail(`Plugin '${p.name}' has no skills/ directory`);
      } else {
        hasSkillsDir = true;
      }
    } catch {
      fail(
        `Plugin '${p.name}' declared in marketplace.json but skills/ not found`,
      );
    }

    if (hasSkillsDir) {
      const pluginManifestPath = join(
        pluginDir,
        ".cursor-plugin",
        "plugin.json",
      );
      try {
        const stat = await Deno.stat(pluginManifestPath);
        if (!stat.isFile) {
          fail(
            `Plugin '${p.name}' has skills/ but .cursor-plugin/plugin.json is not a file`,
          );
        }
      } catch {
        fail(
          `Plugin '${p.name}' has skills/ but .cursor-plugin/plugin.json not found`,
        );
      }
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
    "docs/reference/cli/slice.md",
    "docs/reference/cli/plan.md",
    "docs/reference/cli/registry.md",
    "docs/reference/cli/status.md",
  ]);

  const PATTERNS: DenyPattern[] = [
    {
      pattern: /\bspecify validate /,
      hint: "use `specify slice validate <name>`",
    },
    {
      pattern: /\bspecify merge /,
      hint: "use `specify slice merge run <name>`",
    },
    {
      pattern: /\bspecify spec /,
      hint:
        "use `specify slice merge {preview, conflict-check}` (the `spec` group is retired)",
    },
    {
      pattern: /\bspecify task /,
      hint:
        "use `specify slice task {progress, mark}` (the `task` group is retired)",
    },
    {
      pattern: /\bspecify initiative brief\b/,
      hint:
        "use `specify change {create, show}` (the `initiative` family was renamed to `change` by RFC-13 §3.5)",
    },
    {
      pattern: /\bspecify initiative registry\b/,
      hint: "use `specify registry {show, validate}`",
    },
    {
      pattern: /\bspecify change phase-outcome\b/,
      hint:
        "use `specify slice outcome set ...` (per-loop verbs moved from `change` to `slice` by RFC-13 §3.2)",
    },
    {
      pattern: /\bspecify change journal-append\b/,
      hint:
        "use `specify slice journal append ...` (per-loop verbs moved from `change` to `slice` by RFC-13 §3.2)",
    },
    {
      // The bare `specify slice outcome <name>` form (no `set`/`show` after
      // `outcome`) is ambiguous after the cleanup. Reads must use `outcome
      // show`; writes must use `outcome set`.
      pattern: /\bspecify slice outcome (?!set\b|show\b)/,
      hint:
        "use `specify slice outcome show <name>` to read or `specify slice outcome set <name> <phase> <outcome> ...` to write",
    },
    {
      // Likewise for the bare `specify slice journal <name>` form.
      pattern: /\bspecify slice journal (?!append\b|show\b)/,
      hint:
        "use `specify slice journal show <name>` to read or `specify slice journal append <name> <phase> <kind> ...` to write",
    },
  ];

  const SCAN_ROOTS = [
    join(REPO_ROOT, "plugins", "spec", "skills"),
    join(REPO_ROOT, "plugins", "change", "skills"),
    join(REPO_ROOT, "docs"),
  ];

  for (const root of SCAN_ROOTS) {
    for await (
      const entry of walk(root, {
        exts: [".md"],
        includeDirs: false,
      })
    ) {
      if (await underSymlink(entry.path)) continue;
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
              `Retired CLI verb in ${rel}:${
                i + 1
              } -- ${line.trim()} -- ${hint}`,
            );
          }
        }
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 12. RFC-14 workspace landing automation stays retired
// ──────────────────────────────────────────────────────────────

async function checkWorkspaceLanding(): Promise<void> {
  const SCAN_ROOTS = [
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "capabilities"),
    join(REPO_ROOT, ".cursor"),
  ];
  const SCAN_FILES = [
    join(REPO_ROOT, "AGENTS.md"),
    join(REPO_ROOT, "README.md"),
  ];

  const ALLOWED_PREFIXES = [
    "rfcs/",
  ];
  const ALLOWED_FILES = new Set([
    // These pages intentionally document the one-release non-zero shim and
    // migration story, including command synopsis lines with no prose context.
    "docs/reference/cli/workspace.md",
    "docs/explanation/whats-new.md",
    "docs/explanation/migrating-cli-v1.md",
  ]);

  const ALLOWED_WORKSPACE_MERGE_CONTEXT =
    /\b(retir(?:ed|es|ing)|deprecat(?:ed|ion)|shim|non-zero|no longer|removed|must not|never|does not|do not|outside orchestration|operator-owned|operator merge|migration|pre-RFC-14|old `specify workspace merge`|compatibility)\b/i;
  const ALLOWED_AUTO_MERGE_CONTEXT =
    /\b(retir(?:ed|es|ing)|hard error|reject|rejected|pre-flight|without|not set|must not|never|does not|do not|migration|post-RFC|compatibility)\b/i;
  const ALLOWED_GH_MERGE_CONTEXT =
    /\b(operator|forge UI|hand-run|explicit|outside orchestration|never|must not|does not|do not|not call|retir(?:ed|es|ing)|shim|manual)\b/i;

  const targets: string[] = [];
  for (const root of SCAN_ROOTS) {
    let exists = true;
    try {
      await Deno.stat(root);
    } catch {
      exists = false;
    }
    if (!exists) continue;
    for await (
      const entry of walk(root, {
        includeDirs: false,
        exts: [".md", ".mdx", ".mdc"],
      })
    ) {
      if (await underSymlink(entry.path)) continue;
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
    if (ALLOWED_PREFIXES.some((prefix) => rel.startsWith(prefix))) continue;
    if (ALLOWED_FILES.has(rel)) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      continue;
    }

    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      if (
        /\bworkspace merge\b/.test(line) &&
        !ALLOWED_WORKSPACE_MERGE_CONTEXT.test(line)
      ) {
        fail(
          `RFC-14 workspace merge automation in ${rel}:${
            i + 1
          } -- ${line.trim()} -- describe it only as a retired/non-zero shim or migration note`,
        );
      }

      if (
        /--auto-merge\b/.test(line) &&
        !ALLOWED_AUTO_MERGE_CONTEXT.test(line)
      ) {
        fail(
          `RFC-14 retired --auto-merge mention in ${rel}:${
            i + 1
          } -- ${line.trim()} -- the flag must be described as retired/rejected, not active`,
        );
      }

      if (
        /\bgh pr merge\b/.test(line) &&
        /\b(?:umbrella|orchestration|skill|Specify|specify)\b/.test(line) &&
        !ALLOWED_GH_MERGE_CONTEXT.test(line)
      ) {
        fail(
          `RFC-14 automated gh merge instruction in ${rel}:${
            i + 1
          } -- ${line.trim()} -- Specify may only point operators at gh pr merge; it must not invoke it`,
        );
      }
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 13. Instruction files contain output location preamble
// ──────────────────────────────────────────────────────────────

async function checkInstructionPreambles(): Promise<void> {
  // Per-Phase-3 the slice working dir moved from `.specify/changes/` to
  // `.specify/slices/`. Both paths are accepted here for the duration of
  // the cut-over so vendored capability instruction files that still
  // reference the historical path do not silently fail this check.
  const OUTPUT_LOCATION_RE =
    /^> \*\*Output location\*\*: `\.specify\/(changes|slices)\//m;

  for await (
    const entry of walk(CAPABILITIES_DIR, {
      maxDepth: 3,
      includeDirs: false,
      match: [/instructions\/[a-z]+\.md$/],
    })
  ) {
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
      if (await underSymlink(entry.path)) continue;

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

async function checkLegacyLayout(): Promise<void> {
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
    // The umbrella brief was renamed initiative.md → change.md by RFC-13 §3.5
    // (Phase 3.7 ships `specify migrate change-noun`). Either spelling under
    // `.specify/` is wrong post-v2-layout — the brief lives at the repo root.
    /\.specify\/initiative\.md/,
    /\.specify\/change\.md/,
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
    for await (
      const entry of walk(root, {
        includeDirs: false,
        exts: [".md", ".mdx", ".mdc", ".yaml", ".yml", ".json", ".toml"],
      })
    ) {
      if (await underSymlink(entry.path)) continue;
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
// 14. Acceptance scenario frontmatter validation (C03)
//
// Discovers opted-in scenario files under the accepted roots, validates frontmatter
// against `.cursor/schemas/scenario.schema.json`, and runs cross-file
// invariants (id uniqueness, body-id consistency, stages prefix,
// expected-artifact path safety, capability-boundary requirements).
//
// Opt-in rule: a markdown file under one of those roots is validated
// only if it begins with YAML frontmatter. Prose-only docs (READMEs,
// templates, narrative) are skipped silently.
// ──────────────────────────────────────────────────────────────

interface ScenarioFile {
  path: string;
  rel: string;
  content: string;
  frontmatter: Record<string, unknown>;
}

const STAGES_ORDER = ["define", "build", "merge", "drop"] as const;
const SCENARIO_ID_BODY_RE = /^Scenario ID:\s*`?([a-z][a-z0-9-]*)`?\s*$/m;

async function discoverScenarioCandidates(): Promise<string[]> {
  const candidates: string[] = [];

  // Discovery root 1: tests/<suite>/scenario.md
  const testsDir = join(REPO_ROOT, "tests");
  try {
    const stat = await Deno.stat(testsDir);
    if (stat.isDirectory) {
      for await (
        const entry of walk(testsDir, {
          maxDepth: 2,
          includeDirs: false,
          match: [/scenario\.md$/],
        })
      ) {
        const rel = relative(testsDir, entry.path).split("/");
        if (rel.length === 2 && rel[1] === "scenario.md") {
          candidates.push(entry.path);
        }
      }
    }
  } catch {
    // Optional root.
  }

  // Discovery root 2: tests/suites/<suite>/scenario.md
  const suitesDir = join(REPO_ROOT, "tests", "suites");
  try {
    const stat = await Deno.stat(suitesDir);
    if (stat.isDirectory) {
      for await (
        const entry of walk(suitesDir, {
          maxDepth: 2,
          includeDirs: false,
          match: [/scenario\.md$/],
        })
      ) {
        const rel = relative(suitesDir, entry.path).split("/");
        if (rel.length === 2 && rel[1] === "scenario.md") {
          candidates.push(entry.path);
        }
      }
    }
  } catch {
    // Optional root.
  }

  // Discovery roots 3 & 4: capabilities/<cap>/tests/<scenario>.md
  // and capabilities/<cap>/tests/<scenario>/scenario.md
  try {
    const stat = await Deno.stat(CAPABILITIES_DIR);
    if (stat.isDirectory) {
      for await (
        const entry of walk(CAPABILITIES_DIR, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        const rel = relative(CAPABILITIES_DIR, entry.path).split("/");
        // Flat: <cap>/tests/<scenario>.md  → 3 parts
        if (rel.length === 3 && rel[1] === "tests") {
          candidates.push(entry.path);
        }
        // Directory: <cap>/tests/<scenario>/scenario.md → 4 parts
        if (
          rel.length === 4 &&
          rel[1] === "tests" &&
          rel[3] === "scenario.md"
        ) {
          candidates.push(entry.path);
        }
      }
    }
  } catch {
    // No capabilities/.
  }

  // Discovery root 5: plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md
  const pluginsDir = join(REPO_ROOT, "plugins");
  try {
    const stat = await Deno.stat(pluginsDir);
    if (stat.isDirectory) {
      for await (
        const entry of walk(pluginsDir, {
          includeDirs: false,
          match: [/scenario\.md$/],
        })
      ) {
        if (await underSymlink(entry.path)) continue;
        const rel = relative(pluginsDir, entry.path).split("/");
        if (
          rel.length === 6 &&
          rel[1] === "skills" &&
          rel[3] === "fixtures" &&
          rel[5] === "scenario.md"
        ) {
          candidates.push(entry.path);
        }
      }
    }
  } catch {
    // No plugins/.
  }

  return candidates;
}

function isContiguousStagesPrefix(stages: unknown): boolean {
  if (!Array.isArray(stages) || stages.length === 0) return false;
  for (let i = 0; i < stages.length; i++) {
    if (i >= STAGES_ORDER.length) return false;
    if (stages[i] !== STAGES_ORDER[i]) return false;
  }
  return true;
}

async function validateScenarioFrontmatter(): Promise<void> {
  const scenarioSchema = JSON.parse(
    await Deno.readTextFile(join(CURSOR_SCHEMA_DIR, "scenario.schema.json")),
  );
  const ajv = new Ajv2020({ allErrors: true });
  const validate = ajv.compile(scenarioSchema);

  const candidatePaths = await discoverScenarioCandidates();
  // Stable order for reproducible failure output.
  candidatePaths.sort();

  const opted: ScenarioFile[] = [];

  for (const path of candidatePaths) {
    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      continue;
    }
    const rel = relative(REPO_ROOT, path);
    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    // Opt-in rule: only files that lead with YAML frontmatter are scenarios.
    if (!fmMatch) continue;

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Scenario frontmatter: ${rel} — invalid YAML: ${msg}`);
      continue;
    }
    if (fm === null || typeof fm !== "object") {
      fail(
        `Scenario frontmatter: ${rel} — frontmatter must be a YAML mapping`,
      );
      continue;
    }

    opted.push({ path, rel, content, frontmatter: fm });
  }

  // Schema validation per file.
  for (const sc of opted) {
    if (!validate(sc.frontmatter)) {
      for (const err of validate.errors ?? []) {
        const at = err.instancePath || "/";
        fail(
          `Scenario frontmatter: ${sc.rel} — ${at} ${err.message ?? ""}`.trim(),
        );
      }
    }
  }

  // Stages contiguous-prefix rule (cannot be expressed in JSON Schema
  // cleanly; the schema only enforces enum membership and minItems).
  for (const sc of opted) {
    const stages = sc.frontmatter.stages;
    if (stages === undefined) continue;
    if (!isContiguousStagesPrefix(stages)) {
      fail(
        `Scenario frontmatter: ${sc.rel} — stages must be a contiguous prefix of [define, build, merge, drop] starting at 'define'; got ${
          JSON.stringify(stages)
        }`,
      );
    }
  }

  // Body Scenario ID consistency (C02 doubles the id in body prose for
  // resilience; if the body line is present, it must equal frontmatter id).
  for (const sc of opted) {
    const id = sc.frontmatter.id;
    if (typeof id !== "string") continue;
    const body = sc.content.slice(
      sc.content.match(/^---\n[\s\S]*?\n---/)?.[0].length ?? 0,
    );
    const m = body.match(SCENARIO_ID_BODY_RE);
    if (!m) continue;
    if (m[1] !== id) {
      fail(
        `Scenario frontmatter: ${sc.rel} — body 'Scenario ID: \`${
          m[1]
        }\`' does not match frontmatter id '${id}'; align the visible line with the frontmatter id`,
      );
    }
  }

  // expected-artifacts path safety (relative, no '..', no absolute).
  for (const sc of opted) {
    const arts = sc.frontmatter["expected-artifacts"];
    if (!Array.isArray(arts)) continue;
    for (const a of arts) {
      if (typeof a !== "string") continue;
      if (a.length === 0) {
        fail(
          `Scenario frontmatter: ${sc.rel} — expected-artifacts entry is empty`,
        );
        continue;
      }
      if (a.startsWith("/")) {
        fail(
          `Scenario frontmatter: ${sc.rel} — expected-artifact '${a}' must be relative to the scenario workspace, not absolute`,
        );
        continue;
      }
      const segments = a.split("/");
      if (segments.some((seg) => seg === "..")) {
        fail(
          `Scenario frontmatter: ${sc.rel} — expected-artifact '${a}' must not escape the scenario workspace ('..' segment not allowed)`,
        );
      }
    }
  }

  // Cross-file id uniqueness.
  const idsByValue = new Map<string, string[]>();
  for (const sc of opted) {
    const id = sc.frontmatter.id;
    if (typeof id !== "string") continue;
    const seen = idsByValue.get(id) ?? [];
    seen.push(sc.rel);
    idsByValue.set(id, seen);
  }
  for (const [id, paths] of idsByValue) {
    if (paths.length > 1) {
      fail(
        `Scenario frontmatter: duplicate scenario id '${id}' across files: ${
          paths.join(", ")
        }`,
      );
    }
  }
}

// ──────────────────────────────────────────────────────────────
// 15. Recorded acceptance trace header freshness (C16)
//
// Every `tests/recorded/**/*.jsonl` trace must lead with a
// `recorded-trace-header` line carrying a non-empty `schemaVersion: 1`,
// `sourceBackend`, `sourceRunId`, `sourceTimestamp`, and `scenarioId`.
// The check is opt-in / lenient: a missing `tests/recorded/`
// directory or zero trace files is fine (fresh checkout / no recorded
// coverage yet). Only present `.jsonl` files are validated.
//
// In addition, when the most recent commit (HEAD~1..HEAD) touches one
// of these traces, emit a non-fatal warning suggesting the commit body
// quote the source run id from the header so reviewers can correlate
// the trace back to the live run that produced it. The warning is
// printed with the `WARN:` prefix and does NOT increment the failure
// counter (operators must be able to push from a shallow clone where
// HEAD~1 is unavailable). When git itself is missing, the diff probe
// is silently skipped.
// ──────────────────────────────────────────────────────────────

const TRACE_REQUIRED_FIELDS = [
  "kind",
  "schemaVersion",
  "sourceBackend",
  "sourceRunId",
  "sourceTimestamp",
  "scenarioId",
] as const;

async function checkRecordedTraceFreshness(): Promise<void> {
  const recordedRoot = join(REPO_ROOT, "tests", "recorded");
  let rootExists = true;
  try {
    const stat = await Deno.stat(recordedRoot);
    if (!stat.isDirectory) rootExists = false;
  } catch {
    rootExists = false;
  }
  if (!rootExists) return;

  const tracePaths: string[] = [];
  for await (
    const entry of walk(recordedRoot, {
      exts: [".jsonl"],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    tracePaths.push(entry.path);
  }
  // Stable ordering for deterministic output across runs.
  tracePaths.sort();

  for (const path of tracePaths) {
    const rel = relative(REPO_ROOT, path);
    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Recorded trace: ${rel} — cannot read: ${msg}`);
      continue;
    }
    const firstLine = content.split("\n")[0]?.trim() ?? "";
    if (firstLine.length === 0) {
      fail(
        `Recorded trace: ${rel} — empty file (expected a 'recorded-trace-header' line first)`,
      );
      continue;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(firstLine);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Recorded trace: ${rel} — first line is not valid JSON: ${msg}`);
      continue;
    }
    if (
      parsed === null || typeof parsed !== "object" || Array.isArray(parsed)
    ) {
      fail(`Recorded trace: ${rel} — first line must be a JSON object`);
      continue;
    }
    const header = parsed as Record<string, unknown>;
    if (header.kind !== "recorded-trace-header") {
      fail(
        `Recorded trace: ${rel} — first line kind must be 'recorded-trace-header' (got ${
          JSON.stringify(header.kind)
        })`,
      );
      continue;
    }
    if (header.schemaVersion !== 1) {
      fail(
        `Recorded trace: ${rel} — recorded-trace-header.schemaVersion must be 1 (got ${
          JSON.stringify(header.schemaVersion)
        })`,
      );
    }
    for (const field of TRACE_REQUIRED_FIELDS) {
      const value = header[field];
      if (
        value === undefined ||
        value === null ||
        (typeof value === "string" && value.length === 0)
      ) {
        fail(
          `Recorded trace: ${rel} — recorded-trace-header missing required field '${field}'`,
        );
      }
    }
  }

  // Best-effort recency hint: if `git diff --name-only HEAD~1..HEAD`
  // surfaces any of the present trace files, suggest the operator
  // disclose the source run in their commit message. Failures here
  // (no git, shallow clone, single-commit history, no `--allow-run`
  // permission) are non-fatal — `make checks` keeps its narrow
  // `--allow-read` posture by default.
  try {
    const perm = await Deno.permissions.query({ name: "run", command: "git" });
    if (perm.state !== "granted") return;
    const proc = new Deno.Command("git", {
      args: ["diff", "--name-only", "HEAD~1..HEAD"],
      cwd: REPO_ROOT,
      stdout: "piped",
      stderr: "null",
    });
    const out = await proc.output();
    if (out.code !== 0) return;
    const diff = new TextDecoder()
      .decode(out.stdout)
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    const tracesByRel = new Map(
      tracePaths.map((p) => [relative(REPO_ROOT, p), p]),
    );
    for (const rel of diff) {
      if (!tracesByRel.has(rel)) continue;
      const path = tracesByRel.get(rel)!;
      let firstLine = "";
      try {
        firstLine = (await Deno.readTextFile(path)).split("\n")[0] ?? "";
      } catch {
        continue;
      }
      let header: Record<string, unknown> | null = null;
      try {
        const parsed = JSON.parse(firstLine.trim());
        if (parsed && typeof parsed === "object") {
          header = parsed as Record<string, unknown>;
        }
      } catch {
        // Header issues already reported above; skip the recency hint.
      }
      const runId = header?.sourceRunId ?? "<unknown>";
      const ts = header?.sourceTimestamp ?? "<unknown>";
      console.log(
        `WARN: Recorded trace updated in HEAD: ${rel} — ` +
          `consider quoting sourceRunId='${runId}' / sourceTimestamp='${ts}' ` +
          `in the commit message so reviewers can trace it back to the live run.`,
      );
    }
  } catch {
    // git missing or shallow checkout; the recency hint is opt-in.
  }
}

// ──────────────────────────────────────────────────────────────
// 16. First-party codex rule shape validation (RM-03 Change 07)
//
// Discovers first-party rule markdown under `capabilities/*/codex/**/*.md`
// plus the optional repo-root `codex/**/*.md` overlay, validates frontmatter
// against `.cursor/schemas/codex-rule.schema.json`, and checks repo-local
// invariants that are intentionally outside the per-file schema.
// ──────────────────────────────────────────────────────────────

interface CodexFile {
  rel: string;
  frontmatter: Record<string, unknown>;
}

const CODEX_RULE_HEADING_RE = /^## Rule\s*$/m;
const CODEX_CAPABILITY_NAMESPACES: Record<string, Set<string>> = {
  default: new Set(["UNI"]),
  omnia: new Set(["OMNIA", "RUST", "SEC"]),
  contracts: new Set(["IFACE"]),
  vectis: new Set(["VECTIS"]),
};

async function discoverCodexRuleFiles(): Promise<string[]> {
  const paths: string[] = [];

  try {
    const stat = await Deno.stat(CAPABILITIES_DIR);
    if (stat.isDirectory) {
      for await (
        const entry of walk(CAPABILITIES_DIR, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        if (await underSymlink(entry.path)) continue;
        const parts = relative(CAPABILITIES_DIR, entry.path).split("/");
        if (parts.length >= 3 && parts[1] === "codex") {
          paths.push(entry.path);
        }
      }
    }
  } catch {
    // No capabilities/.
  }

  const rootCodexDir = join(REPO_ROOT, "codex");
  try {
    const stat = await Deno.stat(rootCodexDir);
    if (stat.isDirectory) {
      for await (
        const entry of walk(rootCodexDir, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        if (await underSymlink(entry.path)) continue;
        paths.push(entry.path);
      }
    }
  } catch {
    // Repo-root codex overlay is optional.
  }

  return Array.from(new Set(paths)).sort();
}

function formatSchemaError(err: AjvValidationError): string {
  const at = err.instancePath || "/";
  if (
    err.keyword === "required" &&
    typeof err.params?.missingProperty === "string"
  ) {
    return `${at} missing required property '${err.params.missingProperty}'`;
  }
  if (
    err.keyword === "additionalProperties" &&
    typeof err.params?.additionalProperty === "string"
  ) {
    return `${at} unknown property '${err.params.additionalProperty}'`;
  }
  if (err.keyword === "enum" && Array.isArray(err.params?.allowedValues)) {
    return `${at} must be one of ${
      err.params.allowedValues.map((v) => JSON.stringify(v)).join(", ")
    }`;
  }
  if (err.keyword === "pattern" && typeof err.params?.pattern === "string") {
    return `${at} must match ${err.params.pattern}`;
  }
  return `${at} ${err.message ?? "schema violation"}`.trim();
}

function capabilityOwnerForCodexPath(path: string): string | null {
  const parts = relative(CAPABILITIES_DIR, path).split("/");
  if (parts.length >= 3 && parts[1] === "codex") return parts[0];
  return null;
}

function namespaceForRuleId(id: string): string | null {
  return id.match(/^([A-Z]+)-[0-9]{3}$/)?.[1] ?? null;
}

function namespaceList(namespaces: Set<string>): string {
  return [...namespaces].map((ns) => `${ns}-*`).join(", ");
}

async function validateCodexRuleShape(): Promise<void> {
  const codexSchema = JSON.parse(
    await Deno.readTextFile(join(CURSOR_SCHEMA_DIR, "codex-rule.schema.json")),
  );
  const ajv = new Ajv2020({ allErrors: true });
  const validate = ajv.compile(codexSchema);

  const rulePaths = await discoverCodexRuleFiles();
  const rules: CodexFile[] = [];

  for (const path of rulePaths) {
    const rel = relative(REPO_ROOT, path);
    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Codex rule: ${rel} — cannot read: ${msg}`);
      continue;
    }

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) {
      fail(
        `Codex rule: ${rel} — missing leading YAML frontmatter delimited by ---`,
      );
      continue;
    }

    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      fail(`Codex rule frontmatter: ${rel} — invalid YAML: ${msg}`);
      continue;
    }
    if (fm === null || typeof fm !== "object" || Array.isArray(fm)) {
      fail(`Codex rule frontmatter: ${rel} — frontmatter must be a YAML mapping`);
      continue;
    }

    const rule: CodexFile = { rel, frontmatter: fm };
    rules.push(rule);

    if (!validate(fm)) {
      for (const err of validate.errors ?? []) {
        fail(`Codex rule frontmatter: ${rel} — ${formatSchemaError(err)}`);
      }
    }

    const body = content.slice(fmMatch[0].length);
    if (!CODEX_RULE_HEADING_RE.test(body)) {
      fail(`Codex rule body: ${rel} — missing required '## Rule' heading`);
    }

    const id = fm.id;
    if (typeof id !== "string") continue;

    const capability = capabilityOwnerForCodexPath(path);
    if (!capability) continue;

    const allowedNamespaces = CODEX_CAPABILITY_NAMESPACES[capability];
    if (!allowedNamespaces) {
      fail(
        `Codex namespace ownership: ${rel} — capability '${capability}' has no configured codex namespace owner; update scripts/checks.ts before adding first-party rules here`,
      );
      continue;
    }

    const namespace = namespaceForRuleId(id);
    if (namespace && !allowedNamespaces.has(namespace)) {
      fail(
        `Codex namespace ownership: ${rel} — capability '${capability}' may only use ${namespaceList(allowedNamespaces)} ids, got '${id}'`,
      );
    }
  }

  const idsByValue = new Map<string, string[]>();
  for (const rule of rules) {
    const id = rule.frontmatter.id;
    if (typeof id !== "string") continue;
    const seen = idsByValue.get(id) ?? [];
    seen.push(rule.rel);
    idsByValue.set(id, seen);
  }
  for (const [id, paths] of idsByValue) {
    if (paths.length > 1) {
      fail(
        `Codex rule duplicate id '${id}' across files: ${paths.join(", ")}`,
      );
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
  checkWorkspaceLanding(),
  checkRetiredAffectsField(),
  checkLegacyLayout(),
  validateScenarioFrontmatter(),
  checkRecordedTraceFreshness(),
  validateCodexRuleShape(),
]);
await Promise.all([
  validateSkillFrontmatter(),
  checkBodyLineCount(),
  checkCriticalPath(),
  checkDescriptionLength(),
  checkArgumentHint(),
  checkInvocationPositionals(),
  checkNoLicense(),
  checkReferences(),
  checkVariables(),
  checkDirectives(),
  checkPluginConsistency(),
  checkRetiredSlashCommands(),
]);

console.log();
if (errors > 0) {
  console.log(`${RED}${errors} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
