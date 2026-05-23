// Shared helpers, constants, and the run-wide failure counter for the
// modular checks runner. Every concern module under `scripts/checks/`
// imports from here so that:
//   - the failure counter is single-process-global,
//   - REPO_ROOT and the `walk` / YAML / TOML / Ajv imports are pinned in
//     exactly one place,
//   - the standards allowlist is loaded once per process.
//
// `scripts/checks.ts` is the thin entry point that orchestrates the
// modules and reports the final tally.

import { walk } from "jsr:@std/fs@1/walk";
import { parse as parseYaml } from "jsr:@std/yaml@1";
import { dirname, fromFileUrl, join, relative, resolve } from "jsr:@std/path@1";
import Ajv2020Module from "npm:ajv@8/dist/2020.js";

export { dirname, join, parseYaml, relative, resolve, walk };

// `_shared.ts` lives one extra directory deeper than the original
// `scripts/checks.ts`, so REPO_ROOT walks up two levels instead of one.
export const REPO_ROOT = resolve(
  dirname(fromFileUrl(import.meta.url)),
  "..",
  "..",
);
export const ADAPTERS_SHARED_DIR = join(REPO_ROOT, "adapters", "shared");
export const SOURCES_DIR = join(REPO_ROOT, "adapters", "sources");
export const TARGETS_DIR = join(REPO_ROOT, "adapters", "targets");

export function resolveSpecifyCliSchemasDir(): string {
  const override = (() => {
    try {
      return Deno.env.get("SPECIFY_CLI_DIR");
    } catch {
      return undefined;
    }
  })();
  return join(resolve(REPO_ROOT, override ?? "../specify-cli"), "schemas");
}
export const CURSOR_SCHEMA_DIR = join(REPO_ROOT, ".cursor", "schemas");

export const RED = "\x1b[0;31m";
export const NC = "\x1b[0m";

export type AjvValidationError = {
  instancePath?: string;
  message?: string;
  keyword?: string;
  params?: Record<string, unknown>;
};

export const Ajv2020 = Ajv2020Module as unknown as {
  new (opts: { allErrors?: boolean }): {
    compile(schema: unknown): ((data: unknown) => boolean) & {
      errors?: AjvValidationError[];
    };
  };
};

let errors = 0;

export function fail(msg: string): void {
  console.log(`${RED}FAIL${NC}: ${msg}`);
  errors++;
}

export function errorCount(): number {
  return errors;
}

export async function underSymlink(filepath: string): Promise<boolean> {
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

export function stripHtmlComments(content: string): string {
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

export function skillBodyLines(content: string): string[] | null {
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) return null;

  const lines = content.slice(fmMatch[0].length).split("\n");
  // Drop leading separator newline and trailing terminating newline so the
  // count matches what an editor displays after the closing `---`.
  if (lines.length > 0 && lines[0] === "") lines.shift();
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

export function skillFrontmatter(
  content: string,
): Record<string, unknown> | null {
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) return null;
  try {
    const parsed = parseYaml(fmMatch[1]);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

// Walk every SKILL.md under plugins/, skipping files that live under
// (or are themselves) symlinks. Used by every skill-body discipline
// predicate so the traversal is defined once.
export async function walkSkillFiles(): Promise<string[]> {
  const out: string[] = [];
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    out.push(entry.path);
  }
  return out;
}

export function formatSchemaError(err: AjvValidationError): string {
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
