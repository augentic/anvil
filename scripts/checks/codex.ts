// First-party codex rule shape (RM-03 Change 07):
//   - discovers rule markdown under targets/<cap>/codex/** and
//     sources/<cap>/codex/** plus the optional repo-root codex/** overlay,
//   - validates frontmatter against codex-rule.schema.json,
//   - enforces a `## Rule` body heading and adapter-namespace
//     ownership (e.g. `omnia` may only emit `OMNIA-*`, `RUST-*`, `SEC-*`
//     rule ids).

import {
  Ajv2020,
  SOURCES_DIR,
  TARGETS_DIR,
  CURSOR_SCHEMA_DIR,
  fail,
  formatSchemaError,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

interface CodexFile {
  rel: string;
  frontmatter: Record<string, unknown>;
}

const CODEX_RULE_HEADING_RE = /^## Rule\s*$/m;
const CODEX_PROFILE_NAMESPACES: Record<string, Set<string>> = {
  default: new Set(["UNI"]),
  omnia: new Set(["OMNIA", "RUST", "SEC"]),
  contracts: new Set(["IFACE"]),
  vectis: new Set(["VECTIS"]),
};

const CODEX_DISCOVERY_ROOTS = [SOURCES_DIR, TARGETS_DIR];

async function discoverCodexRuleFiles(): Promise<string[]> {
  const paths: string[] = [];

  for (const root of CODEX_DISCOVERY_ROOTS) {
    try {
      const stat = await Deno.stat(root);
      if (!stat.isDirectory) continue;
      for await (
        const entry of walk(root, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        if (await underSymlink(entry.path)) continue;
        const parts = relative(root, entry.path).split("/");
        if (parts.length >= 3 && parts[1] === "codex") {
          paths.push(entry.path);
        }
      }
    } catch {
      // Optional root.
    }
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

function adapterOwnerForCodexPath(path: string): string | null {
  for (const root of CODEX_DISCOVERY_ROOTS) {
    const rel = relative(root, path);
    if (rel.startsWith("..") || rel.startsWith("/")) continue;
    const parts = rel.split("/");
    if (parts.length >= 3 && parts[1] === "codex") return parts[0];
  }
  return null;
}

function namespaceForRuleId(id: string): string | null {
  return id.match(/^([A-Z]+)-[0-9]{3}$/)?.[1] ?? null;
}

function namespaceList(namespaces: Set<string>): string {
  return [...namespaces].map((ns) => `${ns}-*`).join(", ");
}

export async function validateCodexRuleShape(): Promise<void> {
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
      fail(
        `Codex rule frontmatter: ${rel} — frontmatter must be a YAML mapping`,
      );
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

    const adapter = adapterOwnerForCodexPath(path);
    if (!adapter) continue;

    const allowedNamespaces = CODEX_PROFILE_NAMESPACES[adapter];
    if (!allowedNamespaces) {
      fail(
        `Codex namespace ownership: ${rel} — adapter '${adapter}' has no configured codex namespace owner; update scripts/checks/codex.ts before adding first-party rules here`,
      );
      continue;
    }

    const namespace = namespaceForRuleId(id);
    if (namespace && !allowedNamespaces.has(namespace)) {
      fail(
        `Codex namespace ownership: ${rel} — adapter '${adapter}' may only use ${
          namespaceList(allowedNamespaces)
        } ids, got '${id}'`,
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
