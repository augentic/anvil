// User-facing documentation hygiene:
//   - Obsolete layer-number terminology ("Layer 3", "Layer 4",
//     "Layers 3 and 4") must not appear anywhere in user-facing docs:
//     the architecture stack collapsed to Layer 0/1/2 and any lingering
//     reference to the old four-layer numbering is a stale citation.
//     The current Layer 0/1/2 vocabulary is allowed everywhere.
//   - RFC citations belong in the decision log and release notes, not
//     in tutorials, how-tos, references, or explanations. Linking the
//     archived RFC file via a markdown link target is still allowed so
//     long as the visible prose does not name the RFC.
//
// Both predicates scan `docs/**/*.md` and tolerate the docs tree being
// absent so partial checkouts still finish cleanly.

import {
  fail,
  join,
  relative,
  REPO_ROOT,
  underSymlink,
  walk,
} from "./_shared.ts";

export async function checkNoLayerNumbersInDocs(): Promise<void> {
  const SCAN_ROOT = join(REPO_ROOT, "docs");
  const ALLOWED_PREFIXES = [
    "docs/explanation/layered-stack.md",
    "docs/explanation/decision-log.md",
    "docs/contributing/",
  ];
  const PATTERNS: RegExp[] = [
    /\bLayer\s+[34]\b/,
    /Layers\s+[34]/,
  ];

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
    if (ALLOWED_PREFIXES.some((prefix) => rel.startsWith(prefix))) continue;

    let content: string;
    try {
      content = await Deno.readTextFile(entry.path);
    } catch {
      continue;
    }
    const lines = content.split("\n");
    for (let i = 0; i < lines.length; i++) {
      for (const pattern of PATTERNS) {
        if (pattern.test(lines[i])) {
          fail(
            `Obsolete Layer 3/4 terminology in ${rel}:${i + 1} -- ${
              lines[i].trim()
            } -- the stack is Layer 0/1/2 (configuration / executing a change / planning a change); update or remove the citation`,
          );
          break;
        }
      }
    }
  }
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
