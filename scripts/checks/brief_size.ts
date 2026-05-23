// Brief-size and brief-frontmatter discipline for target and source
// adapter briefs.
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
//
// Briefs additionally MUST NOT carry YAML frontmatter. Skills carry
// frontmatter because Stage 1 discovery competes their `description`
// against every other installed skill; briefs are resolved by path
// from `adapter.yaml` after the operator has already committed to a
// phase, so a brief `description:` is decoration that drifts and
// duplicates the body H1. The `checkBriefNoFrontmatter` predicate
// rejects any leading `---` block on a brief.

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

// Brief paths under the repo root take the shape
// `adapters/<axis>/<adapter>/briefs/...`. The `axis` slot is always
// `targets` or `sources`.

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
  if (parts.length !== 5) return false;
  if (parts[0] !== "adapters") return false;
  if (parts[1] !== "targets" && parts[1] !== "sources") return false;
  if (parts[3] !== "briefs") return false;
  return PARENT_BRIEF_NAMES.has(parts[4]);
}

function isPhaseSubBrief(relPath: string): boolean {
  const parts = relPath.split("/");
  if (parts.length < 6) return false;
  if (parts[0] !== "adapters") return false;
  if (parts[1] !== "targets" && parts[1] !== "sources") return false;
  if (parts[3] !== "briefs") return false;
  if (parts[4] !== "build" && parts[4] !== "extract") return false;
  return relPath.endsWith(".md");
}

async function* walkBriefs(): AsyncGenerator<{ path: string; relPath: string }> {
  for (const axis of ["targets", "sources"]) {
    const root = join(REPO_ROOT, "adapters", axis);
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
      yield { path: entry.path, relPath };
    }
  }
}

export async function checkBriefSize(): Promise<void> {
  for await (const { path, relPath } of walkBriefs()) {
    const parent = isParentBrief(relPath);
    const phase = !parent && isPhaseSubBrief(relPath);
    if (!parent && !phase) continue;

    const content = await Deno.readTextFile(path);
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

export async function checkBriefNoFrontmatter(): Promise<void> {
  for await (const { path, relPath } of walkBriefs()) {
    const parent = isParentBrief(relPath);
    const phase = !parent && isPhaseSubBrief(relPath);
    if (!parent && !phase) continue;

    const content = await Deno.readTextFile(path);
    if (content.startsWith("---\n") || content.startsWith("---\r\n")) {
      fail(
        `${relPath}: brief has YAML frontmatter. Briefs are not skills — ` +
          `they are resolved by path from adapter.yaml and the loader ` +
          `never reads brief frontmatter. Strip the leading '---' block ` +
          `and rely on the body H1 for the brief title. See ` +
          `docs/standards/skill-authoring.md#brief-authoring.`,
      );
    }
  }
}
