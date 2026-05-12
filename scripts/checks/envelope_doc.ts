// CI guard: the generated section of
// `plugins/references/cli-output-shapes.md` must match what
// `scripts/gen-envelope-doc.ts` would produce from the canonical
// envelope fixtures in `augentic/specify-cli`. Skip silently when the
// sibling CLI repo (or its fixtures dir) is not present (e.g.
// plugins-only clones) — `make doc-envelopes` is the local refresh
// entrypoint that requires it.

import { fail, join, relative, REPO_ROOT } from "./_shared.ts";
import {
  DOC_PATH,
  renderGenerated,
  resolveSpecifyCliDir,
  spliceGenerated,
} from "../gen-envelope-doc.ts";

export async function checkEnvelopeDoc(): Promise<void> {
  const specifyCliDir = resolveSpecifyCliDir();
  try {
    const stat = await Deno.stat(
      join(specifyCliDir, "tests", "fixtures", "plan"),
    );
    if (!stat.isDirectory) return;
  } catch {
    return;
  }

  let generated: string;
  try {
    generated = await renderGenerated(specifyCliDir);
  } catch (err) {
    fail(
      `envelope doc generator failed: ${
        err instanceof Error ? err.message : String(err)
      }`,
    );
    return;
  }
  const current = await Deno.readTextFile(DOC_PATH);
  let next: string;
  try {
    next = spliceGenerated(current, generated);
  } catch (err) {
    fail(
      `${relative(REPO_ROOT, DOC_PATH)} is missing the generation markers (${
        err instanceof Error ? err.message : String(err)
      })`,
    );
    return;
  }
  if (next !== current) {
    fail(
      `${
        relative(REPO_ROOT, DOC_PATH)
      } is stale vs the CLI envelope fixtures — run 'make doc-envelopes' to regenerate`,
    );
  }
}
