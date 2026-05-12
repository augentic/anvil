// SKILL.md frontmatter shape (RFC-10 §A, §D):
//   - validates against `.cursor/schemas/skill.schema.json`,
//   - enforces the `<plugin>-` prefix convention on `name`,
//   - bounds `description` length, restricts `argument-hint` syntax,
//   - rejects retired keys (`license`),
//   - whitelists `allowed-tools` against the known Cursor tool set.

import {
  Ajv2020,
  baselineFor,
  CURSOR_SCHEMA_DIR,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  skillBodyLines,
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
  const MAX_DESCRIPTION_CHARS = 512;

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

// Canonical `argument-hint:` grammar (S3). A hint is a whitespace-
// separated sequence of tokens; each token must be one of:
//
//   <name>          required positional         e.g. <slice-dir>
//   [name]          optional positional         e.g. [crate-name]
//   <a|b|c>         required, mutually exclusive
//   [a|b|c]         optional, mutually exclusive
//   <name>...       repeated required positional
//   [name]...       repeated optional positional
//   --flag          long flag (no value; values follow as a separate
//                   <value>/[value] token, e.g. `--kind <kind>`)
//
// `name` is kebab-case: `[a-z][a-z0-9-]*` per alternative. Bare prose
// ("the slice name", "kind <foo>") and mixed punctuation (`<arg>: foo`)
// are rejected. Unset hints are ignored — the field is optional.
const HINT_NAME = "[a-z][a-z0-9]*(?:-[a-z0-9]+)*";
const HINT_ALT = `${HINT_NAME}(?:\\|${HINT_NAME})*`;
const ARGUMENT_HINT_TOKEN_RE = new RegExp(
  "^(?:" +
    `<${HINT_ALT}>(?:\\.\\.\\.)?` +
    "|" +
    `\\[${HINT_ALT}\\](?:\\.\\.\\.)?` +
    "|" +
    "--[a-z][a-z0-9]*(?:-[a-z0-9]+)*" +
    ")$",
);

// Pure per-hint predicate. Returns `null` when the hint is well-formed
// (or empty), or a human-readable failure message otherwise. The `ctx`
// is threaded through so future call sites (lints, IDE plugins) can
// produce richer diagnostics without coupling to the `fail()` counter.
export function checkArgumentHintGrammar(
  hint: string,
  ctx: { rel: string },
): string | null {
  const trimmed = hint.trim();
  if (trimmed === "") return null;
  const tokens = trimmed.split(/\s+/);
  for (const token of tokens) {
    if (!ARGUMENT_HINT_TOKEN_RE.test(token)) {
      return `Invalid argument-hint in ${ctx.rel}: token '${token}' (in '${hint}') does not match grammar — allowed tokens are <name>, [name], <a|b>, [a|b], <name>..., [name]..., --flag (kebab-case names)`;
    }
  }
  return null;
}

export async function validateArgumentHints(): Promise<void> {
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

    const hint = fm["argument-hint"];
    if (hint === undefined || hint === null) continue;
    if (typeof hint !== "string") {
      fail(`Invalid argument-hint type in ${rel}: must be a string`);
      continue;
    }

    const err = checkArgumentHintGrammar(hint, { rel });
    if (err !== null) fail(err);
  }
}

// Skills-§7: every `$VAR_NAME`-style reference in the SKILL.md body
// must correspond to a token in the frontmatter `argument-hint:`
// (matching by kebab-case form: `$SOURCE_PATH` ↔ `<source-path>`).
//
// Skip the framework-provided positional array (`$ARGUMENTS` and
// indexed accesses like `$ARGUMENTS[0]`) plus common shell environment
// variables (`$HOME`, `$PATH`, `$USER`, `$PWD`, `$SHELL`, `$TMPDIR`).
//
// Skip variables defined inside the body via a `$X = ...` line in any
// fenced code block — those are derived working variables (extracted
// from artifacts at runtime, computed from another argument, or used
// as shell-locals in a bash snippet) and not skill arguments.
//
// Variables whose only occurrences are inside fenced shell blocks
// (` ```bash ` / ` ```sh ` / ` ```shell ` / ` ```zsh `) are also
// skipped — those are shell-language references where `$X` is shell
// syntax, not a skill argument.
//
// The check is intentionally conservative: only `$NAME_LIKE_THIS` (all
// caps + underscores) is considered. Lowercase shell variables, money
// placeholders escaped with `\$`, and anything that already maps to an
// argument-hint token are silent.
const VAR_USE_RE = /\$([A-Z][A-Z0-9_]*)/g;
const VAR_DEF_RE = /^\s*\$([A-Z][A-Z0-9_]*)\s*=/gm;
const FENCED_BLOCK_RE = /```[\s\S]*?```/g;
const SHELL_FENCED_BLOCK_RE = /```(?:bash|sh|shell|zsh)\b[\s\S]*?```/g;
const HINT_TOKEN_RE = /[<\[]([a-z][a-z0-9-]*(?:\|[a-z][a-z0-9-]*)*)[>\]]/g;
const ARGUMENT_BUILTINS = new Set([
  "ARGUMENTS",
  "HOME",
  "PATH",
  "USER",
  "PWD",
  "SHELL",
  "TMPDIR",
]);

function kebabToScreamingSnake(name: string): string {
  return name.replace(/-/g, "_").toUpperCase();
}

function collectHintVarNames(hint: string): Set<string> {
  const out = new Set<string>();
  for (const m of hint.matchAll(HINT_TOKEN_RE)) {
    for (const alt of m[1].split("|")) {
      out.add(kebabToScreamingSnake(alt));
    }
  }
  return out;
}

function collectBodyDefinitions(content: string): Set<string> {
  const out = new Set<string>();
  for (const block of content.matchAll(FENCED_BLOCK_RE)) {
    for (const m of block[0].matchAll(VAR_DEF_RE)) {
      out.add(m[1]);
    }
  }
  return out;
}

function collectBodyUses(bodyText: string): Set<string> {
  const stripped = bodyText.replace(SHELL_FENCED_BLOCK_RE, "");
  const out = new Set<string>();
  for (const m of stripped.matchAll(VAR_USE_RE)) {
    if (ARGUMENT_BUILTINS.has(m[1])) continue;
    out.add(m[1]);
  }
  return out;
}

export async function checkArgumentHintCoversBodyArguments(): Promise<void> {
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

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;
    let fm: Record<string, unknown>;
    try {
      fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
    } catch {
      continue;
    }
    const hint = typeof fm["argument-hint"] === "string"
      ? (fm["argument-hint"] as string)
      : "";

    const declared = collectHintVarNames(hint);
    const bodyText = lines.join("\n");
    const definitions = collectBodyDefinitions(bodyText);
    const uses = collectBodyUses(bodyText);

    const missing: string[] = [];
    for (const v of uses) {
      if (declared.has(v)) continue;
      if (definitions.has(v)) continue;
      missing.push(v);
    }
    if (missing.length === 0) continue;

    const baseline = await baselineFor(
      "argumentHintCoversBodyArguments",
      rel,
    );
    if (missing.length > baseline) {
      const detail = missing.sort().map((v) => `$${v}`).join(", ");
      fail(
        `Body references undeclared argument(s): ${rel} — ${missing.length} > baseline ${baseline}: ${detail} (add a kebab-case token to the frontmatter argument-hint, or define the variable in a body code block)`,
      );
    }
  }
}

// Imperative verbs allowed as the first word of a SKILL.md
// `description:` field. Curated and intentionally inclusive — adding a
// verb here is cheaper than rejecting an otherwise-fine description.
// Anthropic's Skills guidance recommends descriptions start with an
// imperative verb so a model can quickly judge whether the skill is
// callable for the current task.
const IMPERATIVE_VERBS = new Set([
  "add",
  "annotate",
  "apply",
  "audit",
  "author",
  "build",
  "categorise",
  "categorize",
  "check",
  "compare",
  "compile",
  "complete",
  "compose",
  "compute",
  "configure",
  "convert",
  "create",
  "define",
  "describe",
  "design",
  "diff",
  "discover",
  "drive",
  "drop",
  "enforce",
  "execute",
  "expose",
  "export",
  "extract",
  "fetch",
  "fix",
  "format",
  "generate",
  "guard",
  "implement",
  "import",
  "infer",
  "ingest",
  "init",
  "initialize",
  "list",
  "load",
  "merge",
  "monitor",
  "orchestrate",
  "plan",
  "preview",
  "process",
  "produce",
  "propose",
  "publish",
  "reconstruct",
  "render",
  "resolve",
  "review",
  "run",
  "scaffold",
  "select",
  "show",
  "shorten",
  "split",
  "stage",
  "store",
  "summarize",
  "test",
  "translate",
  "transform",
  "trim",
  "validate",
  "verify",
  "wire",
  "wrap",
  "write",
]);

export async function checkDescriptionStartsWithVerb(): Promise<void> {
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

    const description = fm.description;
    if (typeof description !== "string") continue;

    const firstWordMatch = description.trimStart().match(/^([A-Za-z]+)/);
    if (!firstWordMatch) {
      fail(
        `Skill description must start with an imperative verb: ${rel} — no leading word found`,
      );
      continue;
    }

    const firstWord = firstWordMatch[1].toLowerCase();
    if (!IMPERATIVE_VERBS.has(firstWord)) {
      fail(
        `Skill description must start with an imperative verb: ${rel} — '${
          firstWordMatch[1]
        }' not in allow-list (add to IMPERATIVE_VERBS in scripts/checks/skill_frontmatter.ts if it is genuinely imperative)`,
      );
    }
  }
}

export async function checkDescriptionHasUseWhen(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const USE_WHEN_RE = /\b[Uu]se when\b/;

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

    if (!USE_WHEN_RE.test(description)) {
      fail(
        `Skill description missing 'Use when' clause: ${rel} — add a 'Use when …' sentence so the agent knows when to apply this skill`,
      );
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
