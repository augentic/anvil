// One-time (or --check) expansion of mdbook-template {{#template …}} invocations
// into static HTML / native admonitions. Run from repo root:
//   deno run --allow-read --allow-write scripts/expand-doc-templates.ts
//   deno run --allow-read scripts/expand-doc-templates.ts --check

import { basename, dirname, join, resolve } from "jsr:@std/path";

const REPO_ROOT = resolve(new URL(".", import.meta.url).pathname, "..");
const DOCS_ROOT = join(REPO_ROOT, "docs");

const CHAPTER_FILES = [
  "index.md",
  "explanation/concepts.md",
  "tutorials/index.md",
  "tutorials/quick-start.md",
  "how-to/drive-slice-manually.md",
  "reference/quick-reference.md",
  "reference/change-skills/index.md",
  "reference/change-skills/plan.md",
];

const TEMPLATE_LINE = /^\{\{#template\s+(\S+)(?:\s+(.*))?\}\}$/;
const CALLOUT_CLOSE = /^\{\{#template\s+\S*callout-close\.md\s*\}\}$/;

const ADMONITION_BY_VARIANT: Record<string, string> = {
  gate: "IMPORTANT",
  gotcha: "WARNING",
  success: "TIP",
  unchanged: "NOTE",
};

function parseArgs(argString: string | undefined): Record<string, string> {
  const args: Record<string, string> = {};
  if (!argString?.trim()) return args;

  const s = argString.trim();
  let i = 0;
  while (i < s.length) {
    const eq = s.indexOf("=", i);
    if (eq === -1) break;

    let keyStart = eq;
    while (keyStart > i && s[keyStart - 1] !== " ") keyStart--;
    const key = s.slice(keyStart, eq);

    const rest = s.slice(eq + 1);
    const nextKey = rest.match(/\s(\w+)=/);
    const valEnd = nextKey?.index !== undefined
      ? eq + 1 + nextKey.index
      : s.length;

    args[key] = s.slice(eq + 1, valEnd).trim();
    i = valEnd;
  }
  return args;
}

function substitutePartial(content: string, args: Record<string, string>): string {
  return content.replace(/\[\[#(\w+)\s*\]\]/g, (_, key: string) => args[key] ?? "");
}

function partialName(templatePath: string): string {
  return basename(templatePath, ".md");
}

function variantToAdmonition(variant: string | undefined): string {
  if (!variant) return "NOTE";
  return ADMONITION_BY_VARIANT[variant] ?? "NOTE";
}

function toAdmonition(bodyLines: string[], variant: string | undefined): string[] {
  const kind = variantToAdmonition(variant);
  const out = [`> [!${kind}]`];
  for (const line of bodyLines) {
    out.push(line.length === 0 ? ">" : `> ${line}`);
  }
  return out;
}

async function expandChapter(relativePath: string): Promise<string> {
  const filePath = join(DOCS_ROOT, relativePath);
  const chapterDir = dirname(filePath);
  const lines = (await Deno.readTextFile(filePath)).split("\n");
  const result: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const match = line.match(TEMPLATE_LINE);
    if (!match) {
      result.push(line);
      continue;
    }

    const templateRel = match[1];
    const args = parseArgs(match[2]);
    const partialPath = resolve(chapterDir, templateRel);
    const name = partialName(templateRel);

    if (name === "callout-open") {
      const bodyLines: string[] = [];
      i++;
      while (i < lines.length) {
        if (CALLOUT_CLOSE.test(lines[i])) break;
        bodyLines.push(lines[i]);
        i++;
      }
      result.push(...toAdmonition(bodyLines, args.variant));
      continue;
    }

    if (name === "callout-close") {
      continue;
    }

    const partialContent = substitutePartial(
      await Deno.readTextFile(partialPath),
      args,
    );
    for (const pl of partialContent.split("\n")) {
      result.push(pl);
    }
  }

  return result.join("\n");
}

async function main(): Promise<void> {
  const checkOnly = Deno.args.includes("--check");
  let drift = false;

  for (const rel of CHAPTER_FILES) {
    const expanded = await expandChapter(rel);
    const filePath = join(DOCS_ROOT, rel);
    const current = await Deno.readTextFile(filePath);

    if (expanded !== current) {
      drift = true;
      if (checkOnly) {
        console.error(`drift: docs/${rel} needs template expansion`);
      } else {
        await Deno.writeTextFile(filePath, expanded);
        console.log(`expanded docs/${rel}`);
      }
    }
  }

  if (checkOnly && drift) {
    Deno.exit(1);
  }
}

main();
