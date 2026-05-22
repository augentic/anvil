// Cross-document prose enforcement:
//   - operational vocabulary stays current (no `.specify/changes/`,
//     no retired `specify validate|merge|plan` invocations, no
//     `initiative` term),
//   - skill numeric caps (description / body limits) stay in sync
//     across the schema, rules, standards, and check sources,
//   - slash-skill invocations stay positional — no `--flags` after the
//     skill token.
//
// Each predicate scans a configurable set of roots and tolerates
// missing trees so partial checkouts (a doc-only PR, a sparse clone)
// still finish cleanly.

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

export async function checkOperationalVocabulary(): Promise<void> {
  const SCAN_ROOTS = [
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, ".cursor"),
  ];
  const SCAN_FILES = [
    join(REPO_ROOT, "AGENTS.md"),
    join(REPO_ROOT, "README.md"),
  ];
  const ALLOWED_PREFIXES = [
    "rfcs/",
    "docs/explanation/decision-log.md",
    "docs/explanation/release-notes.md",
  ];
  const ALLOWED_SEGMENTS = [
    "/fixtures/",
    "/archive/",
  ];
  const FORBIDDEN: Array<[RegExp, string]> = [
    [/\.specify\/changes\//, "use `.specify/slices/` for slice-local state"],
    [/\bspecify validate\b/, "use `specify slice validate`"],
    [/\bspecify merge\b/, "use `specify slice merge run`"],
    [/\bspecify change plan\b/, "use `specify plan`"],
    [/\b[Ii]nitiative\b/, "use `change` for the umbrella and `slice` for entries"],
  ];

  const targets: string[] = [];
  for (const root of SCAN_ROOTS) {
    try {
      await Deno.stat(root);
    } catch {
      continue;
    }
    for await (
      const entry of walk(root, {
        includeDirs: false,
        exts: [".md", ".mdc", ".json", ".yaml", ".yml"],
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
      // Optional top-level files may not exist in downstream checkouts.
    }
  }

  for (const path of targets) {
    const rel = relative(REPO_ROOT, path);
    if (ALLOWED_PREFIXES.some((prefix) => rel.startsWith(prefix))) continue;
    if (ALLOWED_SEGMENTS.some((segment) => rel.includes(segment))) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      continue;
    }
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      for (const [pattern, fix] of FORBIDDEN) {
        if (pattern.test(lines[i])) {
          fail(
            `Stale Specify vocabulary in ${rel}:${i + 1} -- ${
              lines[i].trim()
            } -- ${fix}`,
          );
        }
      }
    }
  }
}

export async function checkSkillNumericCaps(): Promise<void> {
  const EXPECTED_DESCRIPTION = 512;
  const EXPECTED_BODY = 200;
  const FILES: Array<[string, boolean, boolean]> = [
    [".cursor/schemas/skill.schema.json", true, false],
    ["docs/standards/skill-authoring.md", true, true],
    ["scripts/checks/skill_frontmatter.ts", true, false],
    ["scripts/checks/skill_body.ts", false, true],
  ];

  for (const [rel, checksDescription, checksBody] of FILES) {
    const path = join(REPO_ROOT, rel);
    let content: string;
    try {
      content = await Deno.readTextFile(path);
    } catch {
      fail(`Skill numeric cap source missing: ${rel}`);
      continue;
    }
    if (checksDescription && !content.includes(String(EXPECTED_DESCRIPTION))) {
      fail(`Skill description cap drift in ${rel}; expected ${EXPECTED_DESCRIPTION}`);
    }
    if (checksBody && !content.includes(String(EXPECTED_BODY))) {
      fail(`Skill body cap drift in ${rel}; expected ${EXPECTED_BODY}`);
    }
  }
}

export async function checkInvocationPositionals(): Promise<void> {
  const SCAN_ROOTS = [
    join(REPO_ROOT, "docs"),
    join(REPO_ROOT, "plugins"),
    join(REPO_ROOT, "adapters", "sources"),
    join(REPO_ROOT, "adapters", "targets"),
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

      // Fenced examples often wrap slash invocations with backslashes
      // and indented continuation rows. Normalize a short logical
      // invocation so `[--flag]` on the next row cannot hide from the
      // check.
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
