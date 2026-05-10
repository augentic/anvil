// SKILL.md frontmatter shape (RFC-10 §A, §D):
//   - validates against `.cursor/schemas/skill.schema.json`,
//   - enforces the `<plugin>-` prefix convention on `name`,
//   - bounds `description` length, restricts `argument-hint` syntax,
//   - rejects retired keys (`license`),
//   - whitelists `allowed-tools` against the known Cursor tool set.

import {
  Ajv2020,
  CURSOR_SCHEMA_DIR,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

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

export async function validateSkillFrontmatter(): Promise<void> {
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

export async function checkDescriptionLength(): Promise<void> {
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

export async function checkArgumentHint(): Promise<void> {
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

export async function checkNoLicense(): Promise<void> {
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
