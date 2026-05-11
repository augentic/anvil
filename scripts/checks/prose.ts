// Cross-document prose enforcement:
//   - retired/stale phrasing must not creep back ("109-point", retired
//     slash commands, RFC-14 workspace-merge automation, v1 layout
//     paths),
//   - retired plan-schema fields ("affects:") must not appear in
//     plan/execute fixture YAML,
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

export async function checkStaleClaims(): Promise<void> {
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

export async function checkRetiredSlashCommands(): Promise<void> {
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
    [/\bspecify plan\b/, "use `specify change plan`"],
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
  const EXPECTED_BODY = 400;
  const FILES: Array<[string, boolean, boolean]> = [
    [".cursor/schemas/skill.schema.json", true, false],
    [".cursor/rules/project.mdc", true, true],
    ["docs/contributing/skill-authoring.md", true, true],
    ["docs/contributing/skill-anatomy.md", true, true],
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

export async function checkWorkspaceLanding(): Promise<void> {
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
  const ALLOWED_FILES = new Set<string>();

  const ALLOWED_WORKSPACE_MERGE_CONTEXT =
    /\b(no longer|removed|must not|never|does not|do not|outside orchestration|operator-owned|operator merge|pre-RFC-14|old `specify workspace merge`)\b/i;
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
          } -- ${line.trim()} -- describe it only as removed or as a command Specify must not call`,
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

export async function checkRetiredAffectsField(): Promise<void> {
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

export async function checkLegacyLayout(): Promise<void> {
  // The v2 layout (specify-cli 0.2.0) moved operator-facing platform
  // artifacts from `.specify/` to the repo root. Doc/skill prose that
  // still references the v1 paths is drift; this check pins the new
  // shape so doc edits cannot regress quietly. Allowed exceptions:
  //
  // - rfcs/archive/* — historical RFCs carry v2-layout banners and
  //   intentionally retain their original paths.
  // - rfcs/roadmap.md — narrative may still reference legacy shapes.
  // - docs/explanation/release-notes.md, docs/explanation/decision-log.md,
  //   docs/reference/directory-layout.md — these documents *describe*
  //   the migration and so must mention both the old and new paths.
  // - The CLI's own legacy-layout error message (in scripts that quote
  //   it).
  const FORBIDDEN_PATTERNS: RegExp[] = [
    /\.specify\/registry\.yaml/,
    /\.specify\/plan\.yaml/,
    // The umbrella brief was renamed initiative.md → change.md by
    // RFC-13 §3.5. Either spelling under `.specify/` is wrong post-v2-
    // layout — the brief lives at the repo root.
    /\.specify\/initiative\.md/,
    /\.specify\/change\.md/,
    /\.specify\/contracts\b/,
  ];

  const ALLOWED_PREFIXES = [
    "rfcs/archive/",
    "rfcs/roadmap.md",
    "docs/reference/directory-layout.md",
    "docs/explanation/release-notes.md",
    "docs/explanation/decision-log.md",
    "scripts/checks.ts",
    "scripts/checks/",
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
              `update the reference or add the file to the allow-list in scripts/checks/prose.ts`,
          );
          break;
        }
      }
    }
  }
}

export async function checkInvocationPositionals(): Promise<void> {
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
