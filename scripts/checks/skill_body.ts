// SKILL.md body shape (RFC-10 §D + AGENTS.md "Skill body discipline"):
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
  baselineFor,
  fail,
  join,
  relative,
  REPO_ROOT,
  skillBodyLines,
  underSymlink,
  walk,
} from "./_shared.ts";

const MAX_BODY_LINES = 250;
const CRITICAL_PATH_MIN_LINES = 150;
const CRITICAL_PATH_HEADING = "## Critical Path";
const MAX_INLINE_JSON_LINES = 30;
// Per-H2 section cap. Default 50 in the original RFC; the 21-section
// audit at cap 50 exceeded the >5 budget so the cap was bumped to 60
// per the S2 chunk plan. Per-file baselines in
// `scripts/standards-allowlist.toml` grandfather the irreducible
// remainder; new sections still fail fast.
const MAX_SECTION_LINES = 60;

export async function checkBodyLineCount(): Promise<void> {
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
    const lineCount = lines.length;

    // Per-file `bodyLineCount` baselines in
    // `scripts/standards-allowlist.toml` grandfather files that exceed
    // the cap (`bodyLineCount = <observed-line-count>`); the baseline
    // raises the effective cap for that file only and is expected to
    // ratchet down as content is migrated into siblings.
    const baseline = await baselineFor("bodyLineCount", rel);
    const effectiveCap = Math.max(MAX_BODY_LINES, baseline);
    if (lineCount > effectiveCap) {
      const extra = baseline > 0 ? ` > grandfathered baseline ${baseline}` : "";
      fail(
        `Skill body too long: ${rel} — ${lineCount} body lines (limit ${MAX_BODY_LINES})${extra}`,
      );
    }
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

export async function checkSectionLineCount(): Promise<void> {
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
      if (cnt > MAX_SECTION_LINES) {
        violations.push({ title, count: cnt });
      }
    }

    const baseline = await baselineFor("sectionLineCount", rel);
    if (violations.length > baseline) {
      const detail = violations
        .map((v) => `'${v.title}' (${v.count} lines)`)
        .join(", ");
      fail(
        `Skill section too long: ${rel} — ${violations.length} section(s) over ${MAX_SECTION_LINES} lines > baseline ${baseline}: ${detail} (move depth into references/ and link from the H2)`,
      );
    }
  }
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
//
// Per-file baselines come from `scripts/standards-allowlist.toml` so
// legitimate orientation demos can be grandfathered while new
// embeddings fail fast.
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

    const baseline = await baselineFor("noEnvelopeExamples", rel);
    if (count > baseline) {
      const where = violations.map((n) => `line ${n}`).join(", ");
      fail(
        `Envelope JSON in skill body: ${rel} — ${count} block(s) at ${where} > baseline ${baseline} (link to plugins/references/cli-output-shapes.md instead of embedding the envelope shape)`,
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
