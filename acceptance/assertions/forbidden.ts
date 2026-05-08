// Forbidden-path helper: confirm a slice did not touch paths that
// belong to another capability or layer (e.g. a contracts slice writing
// implementation files, or an implementation slice writing into the
// contracts baseline).

import { assertNoMatchingPath } from "./files.ts";
import type { AssertionRecord } from "./types.ts";

/**
 * Confirm the workspace contains no files matching any of the supplied
 * globs. Returns a single record per call (the helper rolls all matches
 * into one record so the operator sees one assertion per declared
 * boundary id, with the offending paths in `evidence`).
 */
export async function assertForbiddenPathsUntouched(
  id: string,
  workspace: string,
  globs: string[],
): Promise<AssertionRecord> {
  return await assertNoMatchingPath(id, workspace, globs);
}
