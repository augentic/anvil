---
name: verify
description: Compare current code against baseline specs to detect drift. Use when the user wants to check whether the codebase still matches the merged specifications.
license: MIT
argument-hint: [capability-name?]
---

# Verify

Detect drift between baseline specs and the current codebase.

## Input

Optionally specify a capability name to verify. If omitted, verify all capabilities that have baseline specs.

## Steps

1. **Check initialization and resolve schema**

   Verify `.specify/config.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

   Read `.specify/config.yaml` for the `schema` value and **resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`. Read `schema.yaml` for `terminology.deliverable` (infer plural and heading forms from the deliverable name). Read `references/spec-format.md` for heading conventions.

2. **Locate baseline specs**

   List directories in `.specify/specs/`. Each subdirectory with a `spec.md` file is a capability with a merged baseline.

   If no baseline specs exist:
   > "No baseline specs found under `.specify/specs/`. Nothing to verify."

   If a capability name was specified, filter to just that capability. If it doesn't exist in the baseline, report the error and stop.

3. **Locate source code for each capability**

   For each capability, locate the corresponding source directory. The mapping depends on project conventions:
   - Try `crates/<capability>/` first (Omnia convention)
   - Try `src/<capability>/` as a fallback
   - Try the project root if there is only one capability

   If no source directory can be found for a capability, record it as **NO SOURCE** and continue to the next capability.

4. **Extract current behavior from source**

   For each capability with a source directory, analyze the source code to build a current-state requirement inventory:

   a. Read the source files. Check the schema's `defaults.context` field (from the resolved `schema.yaml`) for tech-stack hints to narrow file extensions (e.g., `.rs` for the omnia schema). Fall back to common extensions (`.rs`, `.ts`, `.js`, `.go`, `.py`) when the context does not specify a language.
   b. Identify distinct behaviors: handlers, business rules, validation logic, error handling, external calls
   c. For each identified behavior, note:
      - A brief description of what the code does
      - The source file and approximate location
      - Whether it maps to an existing baseline requirement (by reading the spec and matching semantically)

   **Do not invoke the full code-analyzer skill** -- this step performs a lightweight read-and-compare, not a full artifact reconstruction. Focus on identifying whether baseline requirements are implemented and whether new unspecified behaviors exist.

5. **Compare against baseline requirements**

   Read each baseline spec from `.specify/specs/<capability>/spec.md`. Parse requirement blocks using the heading conventions from `references/spec-format.md`.

   For each requirement in the baseline, classify it:

   - **COVERED**: The requirement's behavior is present in the source code and appears consistent with the spec
   - **DRIFTED**: The requirement's behavior exists in the source but has diverged (additional logic, changed conditions, different error handling, modified thresholds)
   - **MISSING**: The requirement is in the baseline spec but no corresponding implementation was found in the source

   For each behavior found in the source that doesn't match any baseline requirement:

   - **UNSPECIFIED**: Code behavior with no corresponding baseline requirement

   When comparing, use the requirement's scenarios as the primary matching signal. A requirement is COVERED when its WHEN/THEN scenarios are reflected in the code. It is DRIFTED when the code handles the same trigger but produces different behavior.

6. **Produce drift report**

   Display the report grouped by capability:

   ```text
   ## Drift Report

   ### <capability-name>

   | Status | ID | Requirement | Detail |
   |--------|----|-------------|--------|
   | COVERED | REQ-001 | <name> | |
   | DRIFTED | REQ-002 | <name> | Code adds rate limiting not in spec |
   | MISSING | REQ-003 | <name> | No implementation found |
   | UNSPECIFIED | -- | handleTimeout | New behavior at src/timeout.rs:42 |

   ### Summary

   - N/M requirements covered
   - N requirements drifted
   - N requirements missing from code
   - N unspecified behaviors found
   ```

   If all requirements are COVERED and no UNSPECIFIED behaviors exist:
   > "All baseline specs are consistent with the current codebase."

   If drift is detected, suggest next steps:
   - For DRIFTED: "Update the baseline spec to match the code, or fix the code to match the spec."
   - For MISSING: "Implement the requirement, or remove it from the baseline via a new change (`/spec:define`)."
   - For UNSPECIFIED: "Add a new requirement to cover this behavior via `/spec:define`, or remove the code if it is unintended."

## Output

```text
## Drift Report

### user-auth
| Status | ID | Requirement | Detail |
|--------|----|-------------|--------|
| COVERED | REQ-001 | Password login | |
| COVERED | REQ-002 | Session timeout | |
| DRIFTED | REQ-003 | OAuth login | Code adds PKCE flow not in spec |
| UNSPECIFIED | -- | Rate limiter | New middleware at src/middleware.rs:15 |

### Summary
- 2/3 requirements covered
- 1 requirement drifted
- 0 requirements missing from code
- 1 unspecified behavior found

### Suggested Actions
- DRIFTED REQ-003: Update spec to include PKCE flow, or revert code to match spec.
- UNSPECIFIED Rate limiter: Add a requirement via `/spec:define`, or remove if unintended.
```

## Guardrails

- Read-only -- do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- If no baseline specs exist, report clearly and stop
- Do not run the full code-analyzer skill -- perform lightweight comparison only
- When uncertain whether code matches a requirement, classify as DRIFTED rather than COVERED
- Report all findings before suggesting actions
