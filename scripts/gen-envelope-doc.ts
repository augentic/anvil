// Generate the per-command examples section of
// `plugins/references/cli-output-shapes.md` from the canonical envelope
// fixtures in `augentic/specify-cli`.
//
// The top matter (title, conventions, generation note) is hand-written and
// preserved. Only the block between `<!-- generated:begin -->` and
// `<!-- generated:end -->` is rewritten.
//
// Usage:
//   deno run --allow-read --allow-write --allow-env scripts/gen-envelope-doc.ts
//   deno run --allow-read --allow-env             scripts/gen-envelope-doc.ts --check
//
// Resolves the sibling CLI repo via `SPECIFY_CLI_DIR` (default
// `../specify-cli` relative to this repo root).

import {
  dirname,
  fromFileUrl,
  join,
  relative,
  resolve,
} from "jsr:@std/path@1";

export const REPO_ROOT = resolve(dirname(fromFileUrl(import.meta.url)), "..");
export const DOC_PATH = join(
  REPO_ROOT,
  "plugins",
  "references",
  "cli-output-shapes.md",
);

export const BEGIN_MARKER = "<!-- generated:begin -->";
export const END_MARKER = "<!-- generated:end -->";

export function resolveSpecifyCliDir(): string {
  const override = (() => {
    try {
      return Deno.env.get("SPECIFY_CLI_DIR");
    } catch {
      // No --allow-env at runtime: fall back to the default.
      return undefined;
    }
  })();
  return resolve(REPO_ROOT, override ?? "../specify-cli");
}

// `plan/<verb>-*.json` maps to `specify plan <verb>`. Override only
// when the verb -> CLI command mapping is non-obvious.
const PLAN_VERB_TO_COMMAND: Record<string, string> = {};

// `e2e/goldens/<stem>.json` is curated; every fixture must be mapped
// explicitly so a new fixture forces a deliberate doc decision rather
// than silently producing a section under a guessed name.
const E2E_STEM_TO_GROUP: Record<string, { command: string; variant: string }> =
  {
    "merge-two-spec": {
      command: "specify slice merge run",
      variant: "two-spec",
    },
    "task-mark": { command: "specify slice task mark", variant: "" },
    "task-progress": { command: "specify slice task progress", variant: "" },
    "validate-good": { command: "specify slice validate", variant: "clean" },
    "validate-bad": {
      command: "specify slice validate",
      variant: "with-findings",
    },
  };

interface Fixture {
  variant: string;
  body: string;
  relPath: string;
}

interface Group {
  command: string;
  fixtures: Fixture[];
}

async function readJsonFixtures(
  dir: string,
  specifyCliDir: string,
): Promise<Array<{ name: string; body: string; relPath: string }>> {
  const out: Array<{ name: string; body: string; relPath: string }> = [];
  for await (const entry of Deno.readDir(dir)) {
    if (!entry.isFile) continue;
    if (!entry.name.endsWith(".json")) continue;
    const file = join(dir, entry.name);
    const body = (await Deno.readTextFile(file)).trimEnd();
    out.push({
      name: entry.name,
      body,
      relPath: relative(specifyCliDir, file),
    });
  }
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

function classifyPlan(
  stem: string,
): { command: string; variant: string } {
  const dashIdx = stem.indexOf("-");
  const verb = dashIdx >= 0 ? stem.slice(0, dashIdx) : stem;
  const variant = dashIdx >= 0 ? stem.slice(dashIdx + 1) : "";
  const command = PLAN_VERB_TO_COMMAND[verb] ?? `specify plan ${verb}`;
  return { command, variant };
}

async function loadPlanGroups(specifyCliDir: string): Promise<Group[]> {
  const planDir = join(specifyCliDir, "tests", "fixtures", "plan");
  const byCommand = new Map<string, Group>();
  for (const f of await readJsonFixtures(planDir, specifyCliDir)) {
    const stem = f.name.slice(0, -".json".length);
    const { command, variant } = classifyPlan(stem);
    if (!byCommand.has(command)) {
      byCommand.set(command, { command, fixtures: [] });
    }
    byCommand.get(command)!.fixtures.push({
      variant,
      body: f.body,
      relPath: f.relPath,
    });
  }
  return groupsToSortedArray(byCommand);
}

async function loadE2EGroups(specifyCliDir: string): Promise<Group[]> {
  const e2eDir = join(specifyCliDir, "tests", "fixtures", "e2e", "goldens");
  const byCommand = new Map<string, Group>();
  for (const f of await readJsonFixtures(e2eDir, specifyCliDir)) {
    const stem = f.name.slice(0, -".json".length);
    const mapped = E2E_STEM_TO_GROUP[stem];
    if (!mapped) {
      throw new Error(
        `Unmapped fixture ${f.relPath}; add it to E2E_STEM_TO_GROUP in scripts/gen-envelope-doc.ts`,
      );
    }
    if (!byCommand.has(mapped.command)) {
      byCommand.set(mapped.command, { command: mapped.command, fixtures: [] });
    }
    byCommand.get(mapped.command)!.fixtures.push({
      variant: mapped.variant,
      body: f.body,
      relPath: f.relPath,
    });
  }
  return groupsToSortedArray(byCommand);
}

function groupsToSortedArray(byCommand: Map<string, Group>): Group[] {
  for (const g of byCommand.values()) {
    g.fixtures.sort((a, b) => a.variant.localeCompare(b.variant));
  }
  return [...byCommand.values()].sort((a, b) =>
    a.command.localeCompare(b.command)
  );
}

function renderGroup(group: Group): string {
  const lines: string[] = [];
  lines.push(`### \`${group.command}\``);
  lines.push("");
  if (group.fixtures.length === 1) {
    const f = group.fixtures[0];
    lines.push(`Source fixture: \`${f.relPath}\``);
    lines.push("");
    lines.push("```json");
    lines.push(f.body);
    lines.push("```");
    return lines.join("\n");
  }
  for (const f of group.fixtures) {
    const label = f.variant === "" ? "default" : f.variant;
    lines.push(`#### \`${label}\``);
    lines.push("");
    lines.push(`Source fixture: \`${f.relPath}\``);
    lines.push("");
    lines.push("```json");
    lines.push(f.body);
    lines.push("```");
    lines.push("");
  }
  while (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines.join("\n");
}

export async function renderGenerated(specifyCliDir: string): Promise<string> {
  const allGroups = [
    ...await loadPlanGroups(specifyCliDir),
    ...await loadE2EGroups(specifyCliDir),
  ].sort((a, b) => a.command.localeCompare(b.command));

  const sections: string[] = [];
  for (const g of allGroups) {
    sections.push(renderGroup(g));
    sections.push("");
  }
  while (sections.length > 0 && sections[sections.length - 1] === "") {
    sections.pop();
  }
  return sections.join("\n");
}

export function spliceGenerated(current: string, generated: string): string {
  const beginIdx = current.indexOf(BEGIN_MARKER);
  const endIdx = current.indexOf(END_MARKER);
  if (beginIdx < 0 || endIdx < 0 || endIdx < beginIdx) {
    throw new Error(
      `Generation markers ${BEGIN_MARKER} / ${END_MARKER} not found (or out of order) in ${DOC_PATH}`,
    );
  }
  const before = current.slice(0, beginIdx + BEGIN_MARKER.length);
  const after = current.slice(endIdx);
  return `${before}\n\n${generated}\n\n${after}`;
}

async function main(): Promise<void> {
  const check = Deno.args.includes("--check");
  const specifyCliDir = resolveSpecifyCliDir();
  const generated = await renderGenerated(specifyCliDir);
  const current = await Deno.readTextFile(DOC_PATH);
  const next = spliceGenerated(current, generated);

  if (check) {
    if (next !== current) {
      console.error(
        `${
          relative(REPO_ROOT, DOC_PATH)
        } is out of date with the CLI fixtures; run 'make doc-envelopes' to regenerate.`,
      );
      Deno.exit(1);
    }
    return;
  }
  if (next === current) {
    console.log(`${relative(REPO_ROOT, DOC_PATH)} already up to date.`);
    return;
  }
  await Deno.writeTextFile(DOC_PATH, next);
  console.log(`Wrote ${relative(REPO_ROOT, DOC_PATH)}`);
}

if (import.meta.main) {
  await main();
}
