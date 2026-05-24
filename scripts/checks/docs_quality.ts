// User-facing documentation hygiene:
//   - RFC citations belong in the decision log and release notes, not
//     in tutorials, how-tos, references, or explanations. Linking the
//     archived RFC file via a markdown link target is still allowed so
//     long as the visible prose does not name the RFC.
//   - Pipeline diagrams in explanation/orientation/tutorials/how-to use
//     committed SVGs, not ```text fences (reference pages may keep
//     ASCII command blocks).
//
// The predicate tolerates the docs tree being absent so partial
// checkouts still finish cleanly.

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

const EXPLANATION_ORIENTATION = [
  join(REPO_ROOT, "docs/explanation"),
  join(REPO_ROOT, "docs/orientation"),
  join(REPO_ROOT, "docs/tutorials"),
  join(REPO_ROOT, "docs/how-to"),
];

const TEXT_FENCE_ALLOWLIST = new Set<string>([
  // Empty — add relative paths if a page needs a grandfathered text diagram.
]);

const SVG_IMAGE_RE = /!\[[^\]]*\]\(([^)]+\.svg)\)/g;

function resolveMarkdownAsset(
  mdPath: string,
  target: string,
): string {
  const base = mdPath.replace(/\\/g, "/").split("/").slice(0, -1);
  const parts = target.split("/");
  const stack = [...base];
  for (const part of parts) {
    if (part === "." || part === "") continue;
    if (part === "..") {
      stack.pop();
    } else {
      stack.push(part);
    }
  }
  return stack.join("/");
}

export async function checkNoRfcCitationsInDocs(): Promise<void> {
  const SCAN_ROOT = join(REPO_ROOT, "docs");
  const ALLOWED_PREFIXES = [
    "docs/explanation/decision-log.md",
    "docs/explanation/release-notes.md",
    "docs/contributing/",
  ];
  const RFC_RE = /RFC[- ]?\d+/;
  // Strip markdown link targets so `](rfcs/archive/rfc-N-...)` is not
  // counted as an RFC citation in the visible prose.
  const LINK_TARGET_RE = /\]\([^)]*\)/g;

  try {
    await Deno.stat(SCAN_ROOT);
  } catch {
    return;
  }

  for await (
    const entry of walk(SCAN_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    if (rel.startsWith("docs/assets/")) continue;
    if (ALLOWED_PREFIXES.some((prefix) => rel.startsWith(prefix))) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const stripped = lines[i].replace(LINK_TARGET_RE, "");
      if (RFC_RE.test(stripped)) {
        fail(
          `RFC citation in user-facing docs at ${rel}:${i + 1} -- ${
            lines[i].trim()
          } -- move RFC context to docs/explanation/decision-log.md or strip`,
        );
      }
    }
  }
}

const TEXT_DIAGRAM_ARROW_RE = /(-->|->|→)/;

export async function checkNoTextPipelineDiagramsInExplanation(): Promise<void> {
  for (const root of EXPLANATION_ORIENTATION) {
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
      const rel = relative(REPO_ROOT, entry.path);
      if (TEXT_FENCE_ALLOWLIST.has(rel)) continue;

      let content: string;
      try {
        content = await Deno.readTextFile(entry.path);
      } catch {
        continue;
      }
      const blocks = content.match(/```text[\s\S]*?```/g) ?? [];
      for (const block of blocks) {
        if (TEXT_DIAGRAM_ARROW_RE.test(block)) {
          fail(
            `${rel} uses a \`\`\`text flow diagram — replace with SVG under docs/assets/diagrams/ (see docs/assets/diagrams/_STYLE.md)`,
          );
        }
      }
    }
  }
}

export async function checkDiagramAssetsExist(): Promise<void> {
  const SCAN_ROOT = join(REPO_ROOT, "docs");
  try {
    await Deno.stat(SCAN_ROOT);
  } catch {
    return;
  }

  for await (
    const entry of walk(SCAN_ROOT, {
      exts: [".md"],
      includeDirs: false,
    })
  ) {
    if (await underSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    if (rel.startsWith("docs/book/")) continue;
    if (rel === "docs/assets/diagrams/_STYLE.md") continue;
    if (rel === "docs/standards/doc-authoring.md") continue;

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }

    for (const match of content.matchAll(SVG_IMAGE_RE)) {
      const target = match[1];
      if (/^https?:\/\//.test(target)) continue;
      const abs = resolveMarkdownAsset(entry.path, target);
      try {
        await Deno.stat(abs);
      } catch {
        fail(
          `${rel} references missing SVG ${target} (resolved ${relative(REPO_ROOT, abs)})`,
        );
      }
    }
  }
}
