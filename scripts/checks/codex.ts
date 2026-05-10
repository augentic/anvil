// First-party codex rule shape (RM-03 Change 07):
//   - discovers rule markdown under capabilities/<cap>/codex/** plus
//     the optional repo-root codex/** overlay,
//   - validates frontmatter against codex-rule.schema.json,
//   - enforces a `## Rule` body heading and capability-namespace
//     ownership (e.g. `omnia` may only emit `OMNIA-*`, `RUST-*`, `SEC-*`
//     rule ids).

import {
  Ajv2020,
  CAPABILITIES_DIR,
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

    const capability = capabilityOwnerForCodexPath(path);
    if (!capability) continue;

    const allowedNamespaces = CODEX_CAPABILITY_NAMESPACES[capability];
    if (!allowedNamespaces) {
      fail(
        `Codex namespace ownership: ${rel} — capability '${capability}' has no configured codex namespace owner; update scripts/checks/codex.ts before adding first-party rules here`,
      );
      continue;
    }

    const namespace = namespaceForRuleId(id);
    if (namespace && !allowedNamespaces.has(namespace)) {
      fail(
        `Codex namespace ownership: ${rel} — capability '${capability}' may only use ${
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
