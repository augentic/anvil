// Scenario file parser: frontmatter + canonical body sections.
//
// A markdown file under one of the four discovery roots is treated as a
// scenario only when it carries YAML frontmatter with at least `id` and
// `kind`. Prose-only fixtures are skipped (opt-in discovery contract).

import { parse as parseYaml } from "jsr:@std/yaml@1";

import type {
  Scenario,
  ScenarioBody,
  ScenarioFrontmatter,
  ScenarioSource,
} from "./types.ts";

const FRONTMATTER_RE = /^---\n([\s\S]*?)\n---\n?/;

/** Parse error thrown when the scenario file is structurally invalid. */
export class ScenarioParseError extends Error {
  constructor(public readonly path: string, message: string) {
    super(`${path}: ${message}`);
    this.name = "ScenarioParseError";
  }
}

/**
 * Try to parse a markdown file as a scenario.
 *
 * Returns `null` when the file has no frontmatter (so the discovery walk
 * can skip prose-only fixtures without failing).
 */
export async function tryParseScenarioFile(
  filePath: string,
  relPath: string,
  source: ScenarioSource,
): Promise<Scenario | null> {
  const content = await Deno.readTextFile(filePath);
  const fmMatch = content.match(FRONTMATTER_RE);
  if (!fmMatch) return null;

  let fm: Record<string, unknown>;
  try {
    fm = parseYaml(fmMatch[1]) as Record<string, unknown>;
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new ScenarioParseError(relPath, `invalid YAML frontmatter — ${msg}`);
  }

  if (typeof fm.id !== "string" || typeof fm.kind !== "string") {
    return null;
  }

  const frontmatter = coerceFrontmatter(fm, relPath);
  const body = parseBody(content.slice(fmMatch[0].length));

  return { frontmatter, body, filePath, relPath, source };
}

function coerceFrontmatter(
  fm: Record<string, unknown>,
  relPath: string,
): ScenarioFrontmatter {
  const required = ["id", "owner", "kind", "backend", "entrypoint", "stages", "isolation"] as const;
  for (const key of required) {
    if (!(key in fm)) {
      throw new ScenarioParseError(relPath, `missing required frontmatter field '${key}'`);
    }
  }

  const stages = fm.stages;
  if (!Array.isArray(stages) || stages.some((s) => typeof s !== "string")) {
    throw new ScenarioParseError(relPath, "'stages' must be a list of strings");
  }

  return {
    id: String(fm.id),
    owner: String(fm.owner),
    kind: String(fm.kind),
    capability: typeof fm.capability === "string" ? fm.capability : undefined,
    backend: String(fm.backend) as ScenarioFrontmatter["backend"],
    entrypoint: String(fm.entrypoint),
    stages: stages as ScenarioFrontmatter["stages"],
    isolation: String(fm.isolation),
    "authorship-mode": typeof fm["authorship-mode"] === "string"
      ? (fm["authorship-mode"] as string)
      : undefined,
    assertions: Array.isArray(fm.assertions) ? (fm.assertions as string[]) : undefined,
    "expected-artifacts": Array.isArray(fm["expected-artifacts"])
      ? (fm["expected-artifacts"] as string[])
      : undefined,
    "negative-expectations": Array.isArray(fm["negative-expectations"])
      ? (fm["negative-expectations"] as string[])
      : undefined,
    "stubbed-stages": Array.isArray(fm["stubbed-stages"])
      ? (fm["stubbed-stages"] as ScenarioFrontmatter["stubbed-stages"])
      : undefined,
    "stub-fixtures": isStringMap(fm["stub-fixtures"])
      ? (fm["stub-fixtures"] as ScenarioFrontmatter["stub-fixtures"])
      : undefined,
  };
}

function isStringMap(value: unknown): boolean {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  for (const v of Object.values(value as Record<string, unknown>)) {
    if (typeof v !== "string") return false;
  }
  return true;
}

/**
 * Slice the markdown body into the canonical scenario sections.
 *
 * Sections are recognised by their `## ` headings. Missing sections
 * become empty strings rather than parse errors so the runner can still
 * execute scenarios that omit optional sections (e.g. a scenario with no
 * Inputs).
 */
function parseBody(rawBody: string): ScenarioBody {
  const titleMatch = rawBody.match(/^\s*#\s+(.+)$/m);
  const title = titleMatch ? titleMatch[1].trim() : "";

  const sections = sliceSections(rawBody);
  const get = (heading: string) => sections.get(heading.toLowerCase()) ?? "";

  return {
    title,
    intent: get("intent"),
    workspace: get("workspace"),
    inputs: get("inputs"),
    invocation: get("invocation"),
    expectedArtifacts: get("expected artifacts"),
    assertions: get("assertions"),
    negativeExpectations: get("negative expectations"),
    cleanup: get("cleanup"),
    raw: rawBody,
  };
}

function sliceSections(body: string): Map<string, string> {
  const out = new Map<string, string>();
  const lines = body.split("\n");
  let currentHeading: string | null = null;
  let buf: string[] = [];

  const flush = () => {
    if (currentHeading !== null) {
      out.set(currentHeading.toLowerCase(), buf.join("\n").trim());
    }
  };

  for (const line of lines) {
    const m = line.match(/^##\s+(.+?)\s*$/);
    if (m) {
      flush();
      currentHeading = m[1].trim();
      buf = [];
    } else {
      buf.push(line);
    }
  }
  flush();
  return out;
}
