// SKILL.md body shape:
//   - body line count is bounded,
//   - per-H2 section line count is bounded so depth migrates to
//     `references/` rather than letting the SKILL.md body sprawl,
//   - long bodies must include a 5-7 item Critical Path block,
//   - inline `json` / `jsonc` fences must not exceed 30 lines,
//   - inline `json` / `jsonc` fences must not show CLI envelope shapes
//     (`envelope-version` or wrapped `ok` + `data`/`error`); those live
//     in `plugins/references/cli-output-shapes.md`,
//   - `$VAR`s defined in the Arguments section must be referenced in the
//     body (and vice versa).

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  skillBodyLines,
  underSymlink,
  walk,
} from "./_shared.ts";

const MAX_BODY_LINES = 200;
const CRITICAL_PATH_MIN_LINES = 150;
const CRITICAL_PATH_HEADING = "## Critical Path";
const MAX_INLINE_JSON_LINES = 30;
const MAX_SECTION_LINES = 45;

// Single walk: cheaper than two passes and keeps the body and per-H2
// section budgets co-located.
export async function checkBodyAndSectionLineCounts(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const lines = skillBodyLines(content);
    if (!lines) continue;

    assertBodyLineCount(rel, lines);
    assertSectionLineCounts(rel, lines);
  }
}

function assertBodyLineCount(
  rel: string,
  lines: string[],
): void {
  if (lines.length > MAX_BODY_LINES) {
    fail(
      `Skill body too long: ${rel} — ${lines.length} body lines (limit ${MAX_BODY_LINES})`,
    );
  }
}

function assertSectionLineCounts(
  rel: string,
  lines: string[],
): void {
  const h2Indices: number[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startsWith("## ")) h2Indices.push(i);
  }

  const violations: { title: string; count: number }[] = [];
  for (let i = 0; i < h2Indices.length; i++) {
    const start = h2Indices[i];
    const end = i + 1 < h2Indices.length ? h2Indices[i + 1] : lines.length;
    const title = lines[start].slice(3).trim();
    const sectionLines = lines.slice(start + 1, end);
    const cnt = countSectionBodyLines(sectionLines);
    if (cnt > MAX_SECTION_LINES) violations.push({ title, count: cnt });
  }

  if (violations.length > 0) {
    const detail = violations
      .map((v) => `'${v.title}' (${v.count} lines)`)
      .join(", ");
    fail(
      `Skill section too long: ${rel} — ${violations.length} section(s) over ${MAX_SECTION_LINES} lines: ${detail} (move depth into references/ and link from the H2)`,
    );
  }
}

// Count lines that contribute to the per-section budget: blank lines
// and HTML comments are free; everything else (prose, list items,
// table rows, code-fence delimiters, fenced content) costs one line.
function countSectionBodyLines(sectionLines: string[]): number {
  let count = 0;
  let inFence = false;
  for (const line of sectionLines) {
    if (line.startsWith("```")) {
      inFence = !inFence;
      count++;
      continue;
    }
    if (inFence) {
      count++;
      continue;
    }
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (trimmed.startsWith("<!--") && trimmed.endsWith("-->")) continue;
    count++;
  }
  return count;
}

export async function checkCriticalPath(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");
  const LIST_ITEM_RE = /^(?:\d+\.|-)\s+\S/;

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const lines = skillBodyLines(content);
    if (!lines || lines.length < CRITICAL_PATH_MIN_LINES) continue;

    const headingIndex = lines.findIndex((line) =>
      line.trim() === CRITICAL_PATH_HEADING
    );
    if (headingIndex < 0) {
      fail(
        `Missing Critical Path: ${rel} — ${lines.length} body lines requires '${CRITICAL_PATH_HEADING}'`,
      );
      continue;
    }

    const nextH2Offset = lines.slice(headingIndex + 1).findIndex((line) =>
      line.startsWith("## ")
    );
    const sectionLines = nextH2Offset >= 0
      ? lines.slice(headingIndex + 1, headingIndex + 1 + nextH2Offset)
      : lines.slice(headingIndex + 1);
    // Items may be expressed either as a flat 5-7 entry numbered/bullet
    // list (compact form) or as 5-7 `### ` H3 headings (one per step
    // when each step has its own concise body). Count whichever form
    // appears first.
    let itemCount = 0;
    let mode: "list" | "h3" | null = null;
    for (const line of sectionLines) {
      const trimmed = line.trim();
      if (mode === null) {
        if (trimmed === "") continue;
        if (line.startsWith("### ")) {
          mode = "h3";
          itemCount++;
          continue;
        }
        if (LIST_ITEM_RE.test(line)) {
          mode = "list";
          itemCount++;
          continue;
        }
        // Lead-in prose before the items is allowed; keep scanning.
        continue;
      }
      if (mode === "h3") {
        if (line.startsWith("### ")) itemCount++;
        continue;
      }
      // List mode: empty line ends the list, additional list items add.
      if (trimmed === "") break;
      if (LIST_ITEM_RE.test(line)) itemCount++;
    }

    if (itemCount < 5 || itemCount > 7) {
      fail(
        `Invalid Critical Path: ${rel} — expected 5-7 bullets or numbered items, found ${itemCount}`,
      );
    }
  }
}

export async function checkInlineJsonBlocks(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);
    const lines = content.split("\n");

    let inBlock = false;
    let blockStart = 0;
    let blockLength = 0;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (!inBlock && /^```(json|jsonc)\b/.test(line)) {
        inBlock = true;
        blockStart = i + 1;
        blockLength = 0;
        continue;
      }
      if (inBlock && line.startsWith("```")) {
        if (blockLength > MAX_INLINE_JSON_LINES) {
          fail(
            `Inline JSON too long: ${rel}:${blockStart} — ${blockLength} body lines (limit ${MAX_INLINE_JSON_LINES}); move large output shapes to plugins/references/cli-output-shapes.md and link to them`,
          );
        }
        inBlock = false;
        continue;
      }
      if (inBlock) blockLength++;
    }
  }
}

// Detect fenced ```json / ```jsonc blocks whose contents look like a
// `specify *` CLI envelope (the wrapper that lives in
// `plugins/references/cli-output-shapes.md`). Forbid those in the
// SKILL.md body so envelope shapes drift in exactly one place. Body
// shapes that are NOT wrapped envelopes (e.g. a one-line config
// snippet, or a sidecar artifact like analyze's `metadata.json`)
// remain allowed; the predicate is intentionally narrow.
export async function checkNoEnvelopeExamples(): Promise<void> {
  const FENCE_OPEN_RE = /^\s*(`{3,})(json|jsonc)\b/;
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);
    const lines = content.split("\n");

    let inBlock = false;
    let blockStart = 0;
    let blockBody: string[] = [];
    let openFence: string | null = null;
    let count = 0;
    const violations: number[] = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (!inBlock) {
        const m = line.match(FENCE_OPEN_RE);
        if (m) {
          inBlock = true;
          openFence = m[1];
          blockStart = i + 1;
          blockBody = [];
        }
        continue;
      }
      // Close on a fence of the same length (or longer) at the start of
      // the line, ignoring leading whitespace, and with no trailing
      // language tag.
      const closeRe = new RegExp(`^\\s*${openFence}\\s*$`);
      if (closeRe.test(line)) {
        if (isEnvelopeBody(blockBody)) {
          violations.push(blockStart);
          count++;
        }
        inBlock = false;
        openFence = null;
        blockBody = [];
        continue;
      }
      blockBody.push(line);
    }

    if (count > 0) {
      const where = violations.map((n) => `line ${n}`).join(", ");
      fail(
        `Envelope JSON in skill body: ${rel} — ${count} block(s) at ${where} (link to plugins/references/cli-output-shapes.md instead of embedding the envelope shape)`,
      );
    }
  }
}

// True when the block body looks like a CLI envelope wrapper or one of
// its discriminator keys. Body shapes that merely describe a
// command's `data` payload (no `envelope-version`, no `ok`/`data` pair)
// do not match.
function isEnvelopeBody(body: string[]): boolean {
  const text = body.join("\n");
  if (/"envelope[-_]version"\s*:/.test(text)) return true;
  const hasOk = /"ok"\s*:\s*(true|false)\b/.test(text);
  const hasData = /"data"\s*:/.test(text);
  const hasError = /"error"\s*:\s*\{/.test(text);
  if (hasOk && (hasData || hasError)) return true;
  return false;
}

// `## Critical Path` is the table of contents; step bodies are short
// pointers to references. Verbatim duplication between a Critical
// Path entry and any line elsewhere in the skill body collapses into
// the triplication pattern (Critical Path → Step body → Guardrails)
// the body cap is meant to eliminate.
//
// The predicate parses the `## Critical Path` block (numbered list,
// bullet list, or `### Step` headings — same shapes accepted by
// `checkCriticalPath`), normalises each entry to its prose, then scans
// every line in the rest of the body (everything after the Critical
// Path block ends). A whitespace-normalised exact match between a
// Critical Path entry and a downstream line — when that downstream
// line is a list item or H3/H4 heading — is flagged.
//
// Lines inside fenced code blocks are ignored (they are templates or
// snippets, not narrative prose).
function normaliseEntry(text: string): string {
  return text
    .replace(/^(?:\d+\.|-|\*)\s+/, "")
    .replace(/^#{2,4}\s+/, "")
    .replace(/^Step\s+\d+\s*[:.\-]\s*/i, "")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

function isListOrHeadingLine(line: string): boolean {
  return /^(?:\d+\.|-|\*)\s+\S/.test(line) || line.startsWith("### ") ||
    line.startsWith("#### ");
}

export async function checkNoStepBodyDuplicatesCriticalPath(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);
    const lines = skillBodyLines(content);
    if (!lines) continue;

    const cpStart = lines.findIndex((l) => l.trim() === "## Critical Path");
    if (cpStart < 0) continue;
    const cpEndOffset = lines.slice(cpStart + 1).findIndex((l) =>
      l.startsWith("## ")
    );
    const cpEnd = cpEndOffset < 0 ? lines.length : cpStart + 1 + cpEndOffset;

    const cpEntries = new Set<string>();
    let inFence = false;
    for (let i = cpStart + 1; i < cpEnd; i++) {
      const line = lines[i];
      if (line.startsWith("```")) {
        inFence = !inFence;
        continue;
      }
      if (inFence) continue;
      if (!isListOrHeadingLine(line)) continue;
      const norm = normaliseEntry(line);
      if (norm.length === 0) continue;
      cpEntries.add(norm);
    }
    if (cpEntries.size === 0) continue;

    const violations: { line: number; text: string }[] = [];
    inFence = false;
    for (let i = cpEnd; i < lines.length; i++) {
      const raw = lines[i];
      if (raw.startsWith("```")) {
        inFence = !inFence;
        continue;
      }
      if (inFence) continue;
      if (!isListOrHeadingLine(raw)) continue;
      const norm = normaliseEntry(raw);
      if (norm.length === 0) continue;
      if (cpEntries.has(norm)) {
        violations.push({ line: i + 1, text: raw.trim() });
      }
    }

    if (violations.length > 0) {
      const detail = violations
        .slice(0, 3)
        .map((v) => `line ${v.line}: '${v.text.slice(0, 80)}'`)
        .join("; ");
      const more = violations.length > 3
        ? ` (+${violations.length - 3} more)`
        : "";
      fail(
        `Step body duplicates Critical Path: ${rel} — ${violations.length} match(es): ${detail}${more} (Critical Path is the TOC; keep step bodies as short pointers to references)`,
      );
    }
  }
}

// `## Input` is the canonical frontmatter-restatement smell: every
// historical instance paraphrased the slice-name placeholder already
// rendered by `argument-hint`, the description, or `## Critical Path`
// step 1. Flag any SKILL.md that reintroduces the H2 so the
// frontmatter ↔ body separation documented in
// `docs/standards/skill-authoring.md` (§Skill body discipline,
// rule 1) stays mechanically enforced.
export async function checkNoFrontmatterRestatement(): Promise<void> {
  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);
    const lines = skillBodyLines(content);
    if (!lines) continue;

    const idx = lines.findIndex((line) => line.trim() === "## Input");
    if (idx >= 0) {
      fail(
        `Frontmatter restated in skill body: ${rel}:${idx + 1} — '## Input' restates the argument-hint already rendered on every invocation; drop the H2 (the inference / prompt instruction belongs in Critical Path step 1)`,
      );
    }
  }
}

export async function checkVariables(): Promise<void> {
  const DEF_RE = /^\$([A-Z_][A-Z_0-9]*)\s*=/gm;
  const USE_RE = /\$([A-Z_][A-Z_0-9]*)/g;
  const ARGS_HEADING_RE = /^## (?:Derived )?Arguments/m;
  const CODE_BLOCK_RE = /```text\n([\s\S]*?)```/g;
  const FENCE_RE = /```[\s\S]*?```/g;
  const INLINE_CODE_RE = /`[^`]+`/g;
  const BUILTIN = new Set(["ARGUMENTS", "HOME"]);

  const PLUGINS_DIR = join(REPO_ROOT, "plugins");

  for await (
    const entry of walk(PLUGINS_DIR, {
      match: [/SKILL\.md$/],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    const headingMatch = content.match(ARGS_HEADING_RE);
    if (!headingMatch || headingMatch.index === undefined) continue;
    const headingIdx = headingMatch.index;

    const afterHeading = content.slice(headingIdx + headingMatch[0].length);
    const nextH2 = afterHeading.match(/\n## /);
    const sectionEnd = nextH2
      ? headingIdx + headingMatch[0].length + nextH2.index!
      : content.length;
    const argsSection = content.slice(headingIdx, sectionEnd);

    const defined = new Set<string>();
    const usedInDefs = new Set<string>();

    for (const block of argsSection.matchAll(CODE_BLOCK_RE)) {
      for (const m of block[1].matchAll(DEF_RE)) {
        defined.add(m[1]);
      }
      for (const line of block[1].split("\n")) {
        const eqIdx = line.indexOf("=");
        if (eqIdx < 0) continue;
        const rhs = line.slice(eqIdx + 1);
        for (const m of rhs.matchAll(USE_RE)) {
          if (!BUILTIN.has(m[1])) usedInDefs.add(m[1]);
        }
      }
    }

    if (defined.size === 0) continue;

    const body = content.slice(sectionEnd);
    const bodyNoFences = body.replace(FENCE_RE, "");

    const usedInBody = new Set<string>();
    for (const m of bodyNoFences.matchAll(USE_RE)) {
      if (!BUILTIN.has(m[1])) usedInBody.add(m[1]);
    }

    const bodyStrict = bodyNoFences.replace(INLINE_CODE_RE, "");
    const usedInBodyStrict = new Set<string>();
    for (const m of bodyStrict.matchAll(USE_RE)) {
      if (!BUILTIN.has(m[1])) usedInBodyStrict.add(m[1]);
    }

    for (const v of defined) {
      if (!usedInBody.has(v) && !usedInDefs.has(v)) {
        fail(
          `Unused variable: ${rel} — $${v} defined but never referenced in body`,
        );
      }
    }
    for (const v of usedInBodyStrict) {
      if (!defined.has(v) && !BUILTIN.has(v)) {
        if (/^[A-Z][A-Z_]+$/.test(v)) {
          fail(
            `Undefined variable: ${rel} — $${v} used but not defined in Arguments`,
          );
        }
      }
    }
  }
}
