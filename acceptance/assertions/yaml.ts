// YAML-field assertion helper (RM-01 plan, C09).
//
// `assertYamlField` reads a YAML document at `path`, navigates to the
// node addressed by an RFC 6901 JSON Pointer (`/foo/0/bar`), and
// compares the resolved scalar against `expected`. The handler returns
// a structured `AssertionRecord` with the expected/actual values
// surfaced in the evidence pointer so a maintainer reading
// `assertions.json` does not need to re-open the source YAML.
//
// This is the reusable cousin of the bespoke YAML probes inside
// `acceptance/assertions/setup.ts` (which pre-dates this helper). New
// suites — registry shape rules, plan-file field rules, project.yaml
// invariants — should reach for `assertYamlField` first; only drop
// down to `parseYaml` directly when the rule needs cross-field logic.
//
// Failure semantics:
//   - file unreadable                             -> fail (runner-setup)
//   - YAML invalid                                -> fail (cli-substrate)
//   - pointer resolves to undefined / wrong shape -> fail (caller-supplied)
//   - actual !== expected (deep equality)         -> fail (caller-supplied)
//
// Pointer syntax:
//   * `/`           → root document
//   * `/key`        → property of an object (escape `~` as `~0`, `/` as `~1`)
//   * `/0`          → first element of an array
//   * `/projects/0/name`
//
// Equality:
//   * scalars (string / number / boolean / null) — strict equality
//   * arrays / objects — deep structural equality (JSON serialisation
//     match). Use `assertYamlField` for leaf comparisons, not whole
//     subtree audits.

import { parse as parseYaml } from "jsr:@std/yaml@1";

import { fail, pass } from "./types.ts";
import type { AssertionEvidence, AssertionRecord, FaultDomain } from "./types.ts";

/** Scalar or simple-structured value the helper can compare. */
export type YamlExpected = string | number | boolean | null | YamlExpected[] | {
  [k: string]: YamlExpected;
};

export interface AssertYamlFieldOptions {
  /** Assertion id surfaced in the record. */
  id: string;
  /** Absolute or run-relative path to the YAML file. */
  path: string;
  /** RFC 6901 JSON Pointer. Empty string or `/` selects the root. */
  jsonPointer: string;
  /** Expected leaf value (deep-equal). */
  expected: YamlExpected;
  /**
   * Optional fault-domain attribution. The helper itself surfaces
   * `cli-substrate` for invalid YAML and `runner-setup` for missing
   * files; callers should pass the domain that fits the rule the
   * field expresses (`skill-orchestration` for plan fields,
   * `cli-substrate` for registry fields, etc.).
   */
  faultDomain?: FaultDomain;
  /** Optional override for the human-readable description. */
  description?: string;
}

/**
 * Read `path`, parse as YAML, resolve `jsonPointer`, compare against
 * `expected`. Returns a single `AssertionRecord`.
 */
export async function assertYamlField(
  opts: AssertYamlFieldOptions,
): Promise<AssertionRecord> {
  const description = opts.description ??
    `YAML field at \`${opts.jsonPointer || "/"}\` matches expected.`;
  const fault = opts.faultDomain ?? "cli-substrate";

  let body: string;
  try {
    body = await Deno.readTextFile(opts.path);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return fail(
      opts.id,
      description,
      {
        summary: `cannot read YAML file: ${msg}`,
        paths: [opts.path],
      },
      "runner-setup",
    );
  }

  let parsed: unknown;
  try {
    parsed = parseYaml(body);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return fail(
      opts.id,
      description,
      {
        summary: `invalid YAML: ${msg}`,
        paths: [opts.path],
      },
      "cli-substrate",
    );
  }

  const resolved = resolveJsonPointer(parsed, opts.jsonPointer);
  if (resolved.kind === "missing") {
    return fail(
      opts.id,
      description,
      {
        summary: `pointer not found: ${opts.jsonPointer || "/"} (stopped at '${resolved.failedSegment}')`,
        paths: [opts.path],
        details: { expected: opts.expected, actual: undefined },
      },
      fault,
    );
  }

  if (deepEqual(resolved.value, opts.expected)) {
    const evidence: AssertionEvidence = {
      summary: `${opts.jsonPointer || "/"} = ${formatScalar(resolved.value)}`,
      paths: [opts.path],
    };
    return pass(opts.id, description, evidence);
  }

  return fail(
    opts.id,
    description,
    {
      summary:
        `${opts.jsonPointer || "/"} mismatch: expected ${formatScalar(opts.expected)}, got ${formatScalar(resolved.value)}`,
      paths: [opts.path],
      details: { expected: opts.expected, actual: resolved.value },
    },
    fault,
  );
}

/** Result of a JSON-pointer resolution against a parsed YAML document. */
type PointerResolution =
  | { kind: "found"; value: unknown }
  | { kind: "missing"; failedSegment: string };

/**
 * RFC 6901 JSON Pointer resolution. Empty string or `/` selects the
 * root; otherwise tokens are split on `/` and decoded (`~1` → `/`,
 * `~0` → `~`). Numeric segments index into arrays; non-numeric
 * segments key into objects.
 */
export function resolveJsonPointer(doc: unknown, pointer: string): PointerResolution {
  if (pointer === "" || pointer === "/") return { kind: "found", value: doc };
  if (!pointer.startsWith("/")) {
    return { kind: "missing", failedSegment: `(pointer must start with '/'): ${pointer}` };
  }
  const tokens = pointer.slice(1).split("/").map(decodeToken);
  let cursor: unknown = doc;
  for (const tok of tokens) {
    if (Array.isArray(cursor)) {
      const idx = Number.parseInt(tok, 10);
      if (!Number.isInteger(idx) || idx < 0 || idx >= cursor.length) {
        return { kind: "missing", failedSegment: tok };
      }
      cursor = cursor[idx];
      continue;
    }
    if (cursor && typeof cursor === "object") {
      const obj = cursor as Record<string, unknown>;
      if (!(tok in obj)) return { kind: "missing", failedSegment: tok };
      cursor = obj[tok];
      continue;
    }
    return { kind: "missing", failedSegment: tok };
  }
  return { kind: "found", value: cursor };
}

function decodeToken(t: string): string {
  return t.replace(/~1/g, "/").replace(/~0/g, "~");
}

function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null) return a === b;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false;
    return a.every((v, i) => deepEqual(v, b[i]));
  }
  if (typeof a === "object") {
    const ao = a as Record<string, unknown>;
    const bo = b as Record<string, unknown>;
    const ak = Object.keys(ao);
    const bk = Object.keys(bo);
    if (ak.length !== bk.length) return false;
    return ak.every((k) => deepEqual(ao[k], bo[k]));
  }
  return false;
}

function formatScalar(v: unknown): string {
  if (v === null) return "null";
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}
