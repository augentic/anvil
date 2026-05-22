// User-facing documentation hygiene:
//   - RFC citations belong in the decision log and release notes, not
//     in tutorials, how-tos, references, or explanations. Linking the
//     archived RFC file via a markdown link target is still allowed so
//     long as the visible prose does not name the RFC.
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
