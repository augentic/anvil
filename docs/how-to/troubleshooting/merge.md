# Merge issues

Use this page when `/spec:merge` or `specify slice merge` refuses to fold a slice's specs into the baseline.

## Prerequisites

- A defined slice that is ready to merge (or that has just failed to merge).
- The slice name and the error reported by the merge skill or CLI.

## Baseline conflict detected

**Symptom:** `/spec:merge` fails with a conflict-check error.

**Cause:** The baseline changed since the slice was defined (another slice was merged in between).

**Resolution:**
1. Review the conflict: `specify slice merge conflict-check <name>`
2. Options:
   - Re-run `/spec:define` to update specs against the current baseline.
   - Manually resolve conflicts in the spec files.
   - Drop and redefine: `/spec:drop`, then `/spec:define` with updated description.

## Coherence failure after merge

**Symptom:** `specify slice merge run` fails during coherence validation.

**Cause:** The merged baseline has structural issues (e.g. duplicate requirement IDs, broken references).

**Resolution:**
1. Review the error message for the specific coherence issue.
2. Fix the spec files in the slice directory.
3. Retry: `/spec:merge`
