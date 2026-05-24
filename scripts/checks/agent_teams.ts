// Per-target-adapter agent-teams canonicalisation guard.
//
// The shared review-team protocol lives at
// `docs/reference/review-team-protocol.md`. Every target adapter exposes
// it through `adapters/targets/<name>/references/agent-teams.md` so the
// brief-relative link stays self-contained. This predicate asserts each
// such file is either:
//   - a real symlink resolving to the canonical doc inside the repo, or
//   - a regular file whose SHA-256 matches the canonical doc.
//
// Either shape keeps the per-adapter brief link working without
// allowing silent content drift. The symlink form is preferred.

import { fail, join, relative, REPO_ROOT, TARGETS_DIR } from "./_shared.ts";

const CANONICAL_REL = "docs/reference/review-team-protocol.md";

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export async function checkAgentTeamsCanonical(): Promise<void> {
  const canonicalPath = join(REPO_ROOT, CANONICAL_REL);

  let canonicalBytes: Uint8Array;
  try {
    canonicalBytes = await Deno.readFile(canonicalPath);
  } catch {
    fail(
      `Agent-teams canonical: ${CANONICAL_REL} is missing — cannot validate per-adapter copies`,
    );
    return;
  }
  const canonicalHash = await sha256(canonicalBytes);

  let targets: Deno.DirEntry[];
  try {
    targets = [];
    for await (const entry of Deno.readDir(TARGETS_DIR)) {
      if (entry.isDirectory) targets.push(entry);
    }
  } catch {
    return;
  }

  for (const target of targets) {
    const refPath = join(
      TARGETS_DIR,
      target.name,
      "references",
      "agent-teams.md",
    );
    const refRel = relative(REPO_ROOT, refPath);

    let info: Deno.FileInfo;
    try {
      info = await Deno.lstat(refPath);
    } catch {
      continue;
    }

    if (info.isSymlink) {
      let resolved: string;
      try {
        resolved = await Deno.realPath(refPath);
      } catch {
        fail(`Agent-teams overlay: ${refRel} — symlink does not resolve`);
        continue;
      }
      const expected = await Deno.realPath(canonicalPath);
      if (resolved !== expected) {
        fail(
          `Agent-teams overlay: ${refRel} — symlink resolves to '${
            relative(REPO_ROOT, resolved)
          }', expected '${CANONICAL_REL}'`,
        );
      }
      continue;
    }

    if (info.isFile) {
      const localBytes = await Deno.readFile(refPath);
      const localHash = await sha256(localBytes);
      if (localHash !== canonicalHash) {
        fail(
          `Agent-teams overlay: ${refRel} — content drifted from canonical '${CANONICAL_REL}' (replace with a symlink or re-sync the file)`,
        );
      }
      continue;
    }

    fail(
      `Agent-teams overlay: ${refRel} — must be a regular file or symlink, found unsupported entry type`,
    );
  }
}
