// Acceptance scenario discipline (C03 + C16):
//   - opt-in scenario files (markdown leading with frontmatter)
//     validate against `.cursor/schemas/scenario.schema.json`,
//   - per-file invariants enforce id uniqueness, body-id consistency,
//     a contiguous-prefix `stages` value, and safe expected-artifact
//     paths,
//   - recorded `tests/recorded/**/*.jsonl` traces lead with a complete
//     `recorded-trace-header` line; HEAD recency is hinted (non-fatal)
//     when git is reachable.

import {
  Ajv2020,
  TARGETS_DIR,
  CURSOR_SCHEMA_DIR,
  fail,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

interface ScenarioFile {
  path: string;
  rel: string;
  content: string;
  frontmatter: Record<string, unknown>;
}

const STAGES_ORDER = ["plan", "refine", "build", "merge", "drop"] as const;
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

  // Discovery root 3: tests/plan/<scenario>.md
  const planTestsDir = join(REPO_ROOT, "tests", "plan");
  try {
    const stat = await Deno.stat(planTestsDir);
    if (stat.isDirectory) {
      for await (
        const entry of walk(planTestsDir, {
          maxDepth: 1,
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        const rel = relative(planTestsDir, entry.path).split("/");
        if (rel.length === 1) {
          candidates.push(entry.path);
        }
      }
    }
  } catch {
    // Optional root.
  }

  // Discovery roots 4 & 5: adapters/targets/<target>/tests/<scenario>.md
  // and adapters/targets/<target>/tests/<scenario>/scenario.md.
  try {
    const stat = await Deno.stat(TARGETS_DIR);
    if (stat.isDirectory) {
      for await (
        const entry of walk(TARGETS_DIR, {
          exts: [".md"],
          includeDirs: false,
        })
      ) {
        const rel = relative(TARGETS_DIR, entry.path).split("/");
        // Flat: <target>/tests/<scenario>.md  → 3 parts
        if (rel.length === 3 && rel[1] === "tests") {
          candidates.push(entry.path);
        }
        // Directory: <target>/tests/<scenario>/scenario.md → 4 parts
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
    // Optional root.
  }

  // Discovery root 6:
  // plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md
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
  // RFC-25: stages MUST be a contiguous slice of STAGES_ORDER. Scenarios
  // may anchor at the plan-authoring phase or at any later slice phase
  // (e.g. an adapter-scope scenario that starts in `refine`); the rule
  // is contiguity rather than always-starts-at-plan.
  if (!Array.isArray(stages) || stages.length === 0) return false;
  const first = stages[0];
  const start = STAGES_ORDER.indexOf(first as typeof STAGES_ORDER[number]);
  if (start < 0) return false;
  for (let i = 0; i < stages.length; i++) {
    if (start + i >= STAGES_ORDER.length) return false;
    if (stages[i] !== STAGES_ORDER[start + i]) return false;
  }
  return true;
}

export async function validateScenarioFrontmatter(): Promise<void> {
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
    // Opt-in rule: only files that lead with YAML frontmatter are
    // scenarios.
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
          `Scenario frontmatter: ${sc.rel} — ${at} ${err.message ?? ""}`
            .trim(),
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
        `Scenario frontmatter: ${sc.rel} — stages must be a contiguous slice of [plan, refine, build, merge, drop] anchored at any element; got ${
          JSON.stringify(stages)
        }`,
      );
    }
  }

  // Body Scenario ID consistency (C02 doubles the id in body prose for
  // resilience; if the body line is present, it must equal the
  // frontmatter id).
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

const TRACE_REQUIRED_FIELDS = [
  "kind",
  "schemaVersion",
  "sourceBackend",
  "sourceRunId",
  "sourceTimestamp",
  "scenarioId",
] as const;

export async function checkRecordedTraceFreshness(): Promise<void> {
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
  // permission) are non-fatal — `make check` keeps its narrow
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
