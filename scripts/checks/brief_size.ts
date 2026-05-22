// Brief-size discipline for target and source adapter briefs.
//
// Adapter briefs split into two roles:
//
//   - Parent briefs orchestrate. They declare bindings, mode dispatch,
//     phase order, and cross-phase loops, then load phase sub-briefs by
//     relative-link instruction. Files:
//       adapters/targets/<name>/briefs/{shape,build,merge}.md
//       adapters/sources/<name>/briefs/{enumerate,extract}.md
//     Hard cap: 150 non-blank lines.
//
//   - Phase sub-briefs carry the operational body for one phase. Files:
//       adapters/targets/<name>/briefs/build/**/*.md
//       adapters/sources/<name>/briefs/extract/**/*.md
//     Soft cap (warning, non-fatal): 500 non-blank lines.
//     Hard cap (failure): 800 non-blank lines.
//
// The caps codify the orchestrator + sub-brief layering: depth migrates
// to plugins/<name>/references/ (worked examples, templates, mapping
// tables) rather than letting individual briefs sprawl. The "wave 2"
// migration deleted ~34 000 LOC by silently squeezing four+ skill
// bodies into a single brief; the cap turns that regression class into
// a hard `make checks` failure.

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

const PARENT_BRIEF_HARD_CAP = 150;
const PHASE_BRIEF_SOFT_CAP = 500;
const PHASE_BRIEF_HARD_CAP = 800;

const PARENT_BRIEF_NAMES = new Set([
  "shape.md",
  "build.md",
  "merge.md",
  "enumerate.md",
  "extract.md",
]);

function countNonBlankLines(content: string): number {
  let count = 0;
  let inBlockComment = false;
  for (const raw of content.split("\n")) {
    const line = raw.trim();
    if (inBlockComment) {
      if (line.includes("-->")) inBlockComment = false;
      continue;
    }
    if (line === "") continue;
    if (line.startsWith("<!--") && !line.includes("-->")) {
      inBlockComment = true;
      continue;
    }
    if (line.startsWith("<!--") && line.includes("-->")) continue;
    count++;
  }
  return count;
}

function isParentBrief(relPath: string): boolean {
  const parts = relPath.split("/");
  if (parts.length !== 4) return false;
  if (parts[0] !== "targets" && parts[0] !== "sources") return false;
  if (parts[2] !== "briefs") return false;
  return PARENT_BRIEF_NAMES.has(parts[3]);
}

function isPhaseSubBrief(relPath: string): boolean {
  const parts = relPath.split("/");
  if (parts.length < 5) return false;
  if (parts[0] !== "targets" && parts[0] !== "sources") return false;
  if (parts[2] !== "briefs") return false;
  if (parts[3] !== "build" && parts[3] !== "extract") return false;
  return relPath.endsWith(".md");
}

export async function checkBriefSize(): Promise<void> {
  for (const subtree of ["targets", "sources"]) {
    const root = join(REPO_ROOT, subtree);
    try {
      await Deno.stat(root);
    } catch {
      continue;
    }

    for await (
      const entry of walk(root, {
        exts: [".md"],
        includeDirs: false,
      })
    ) {
      if (await underSymlink(entry.path)) continue;
      const relPath = relative(REPO_ROOT, entry.path);

      const parent = isParentBrief(relPath);
      const phase = !parent && isPhaseSubBrief(relPath);
      if (!parent && !phase) continue;

      const content = await Deno.readTextFile(entry.path);
      const lines = countNonBlankLines(content);

      if (parent && lines > PARENT_BRIEF_HARD_CAP) {
        fail(
          `${relPath}: parent brief is ${lines} non-blank lines, ` +
            `exceeds hard cap ${PARENT_BRIEF_HARD_CAP}. Parent briefs orchestrate; ` +
            `move operational depth into a phase sub-brief under ` +
            `${relPath.replace(/\.md$/, "/")}<phase>.md or into ` +
            `plugins/<name>/references/.`,
        );
        continue;
      }

      if (phase && lines > PHASE_BRIEF_HARD_CAP) {
        fail(
          `${relPath}: phase sub-brief is ${lines} non-blank lines, ` +
            `exceeds hard cap ${PHASE_BRIEF_HARD_CAP}. Split into sub-phases ` +
            `or move material into plugins/<name>/references/.`,
        );
        continue;
      }

      if (phase && lines > PHASE_BRIEF_SOFT_CAP) {
        console.warn(
          `WARN: ${relPath}: phase sub-brief is ${lines} non-blank lines, ` +
            `above soft cap ${PHASE_BRIEF_SOFT_CAP}. Consider moving worked ` +
            `examples and templates into plugins/<name>/references/.`,
        );
      }
    }
  }
}
