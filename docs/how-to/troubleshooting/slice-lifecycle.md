# Slice lifecycle issues

Use this page when a Specify skill refuses to act on a slice -- because it cannot find it, says the slice is in the wrong state, or reports missing artifacts after define.

## Prerequisites

- A project initialised with `/spec:init` (`.specify/` exists at the repo root).
- The name of the slice you were operating on (from `specify status` or the skill's error output).

## "Slice not found"

**Symptom:** A skill reports that no slice exists or cannot find the specified slice.

**Cause:** The slice name is misspelled, or `/spec:init` has not been run.

**Resolution:**
1. Check active slices: `specify status`
2. Verify `.specify/` exists. If not, run `/spec:init`.

## "Slice not in expected state"

**Symptom:** A skill refuses to proceed because the slice is in the wrong lifecycle state (e.g. trying to build a slice that is not yet defined).

**Cause:** A previous phase did not complete, or the slice was manually transitioned.

**Resolution:**
1. Check the state: `specify slice status <name>`
2. Complete the missing phase (e.g. run `/spec:define`) or manually transition: `specify slice transition <name> <target>`

## Artifacts incomplete after define

**Symptom:** `/spec:build` reports missing artifacts even though `/spec:define` appeared to complete.

**Cause:** Define may have encountered an error mid-pipeline and not generated all artifacts.

**Resolution:**
1. Check which artifacts exist in `.specify/slices/<name>/`.
2. Re-run define to regenerate: `/spec:define <name>` or regenerate a specific artifact: `/spec:define <name> <artifact-id>`
