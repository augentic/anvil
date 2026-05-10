// SKILL.md body shape (RFC-10 §D + AGENTS.md "Skill body discipline"):
//   - body line count is bounded,
//   - long bodies must include a 5-7 item Critical Path block,
//   - inline `json` / `jsonc` fences must not exceed 30 lines,
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

const MAX_BODY_LINES = 400;
const CRITICAL_PATH_MIN_LINES = 150;
const CRITICAL_PATH_HEADING = "## Critical Path (Quick Reference)";
const MAX_INLINE_JSON_LINES = 30;

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

    if (lineCount > MAX_BODY_LINES) {
      fail(
        `Skill body too long: ${rel} — ${lineCount} body lines (limit ${MAX_BODY_LINES})`,
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
    let itemCount = 0;
    let inCriticalPathList = false;
    for (const line of sectionLines) {
      if (line.trim() === "") {
        if (inCriticalPathList) break;
        continue;
      }
      if (LIST_ITEM_RE.test(line)) {
        inCriticalPathList = true;
        itemCount++;
      }
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
