---
name: review
description: Validate artifacts with structured checks and cross-artifact consistency. Sets status to reviewed when all checks pass. Use when the user wants to verify artifact quality before implementation.
license: MIT
---

# Review

Validate all artifacts for a change using structured checks from the schema. Produces a review summary and sets status to `reviewed` when all checks pass.

## Input

Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`.

   Read `schema.yaml` for artifact definitions, `spec-format` heading conventions, `validate-checks`, and `cross-artifact-checks`.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `proposing`: warn that artifacts may be incomplete. Suggest running `/spec:propose` to finish first, but allow proceeding if user confirms.
   - If `status` is `reviewed`: inform the user that the change has already been reviewed. Offer to re-review.
   - If `status` is not `proposed` and not `proposing` and not `reviewed`: warn that review is normally run on `proposed` changes. Allow proceeding if user confirms.

3. **Read all artifacts**

   For each artifact defined in `schema.yaml`, read the file(s) at `.specify/changes/<name>/<generates>`. For glob patterns (e.g., `specs/**/*.md`), read all matching files in the directory.

   If an artifact file is missing, record it as a `MISSING` failure for that artifact and continue.

4. **Run per-artifact validate-checks**

   For each artifact that has a `validate-checks` field in `schema.yaml`, run every check against the artifact content. See [check-types.md](../../references/check-types.md) for the full check type definitions and parameters.

   Record each check result as **PASS** or **FAIL** with a reason.

5. **Run cross-artifact consistency checks**

   If the schema defines `cross-artifact-checks`, run each one. See [check-types.md](../../references/check-types.md) for the cross-artifact check type definitions.

   Record each check result as **PASS** or **FAIL** with details.

6. **Produce review summary**

   Format the results as a structured report:

   ```text
   ## Review Summary: <change-name>

   ### Per-Artifact Checks

   **proposal.md**
   - PASS: heading-exists (## Why)
   - PASS: heading-exists (## Source)
   - FAIL: heading-exists (## Crates) — heading found but no content below it

   **specs/user-auth/spec.md**
   - PASS: spec-structure
   - PASS: requirement-has-id
   - FAIL: requirement-has-scenario — REQ-003 has no scenario

   **design.md**
   - PASS: heading-exists (## Context)
   - PASS: heading-exists-or-waived (## Domain Model)

   **tasks.md**
   - PASS: pattern-match (task-lines)
   - PASS: min-count (task-lines)

   ### Cross-Artifact Checks

   - PASS: proposal-crates-have-specs
   - FAIL: design-references-valid — REQ-005 referenced in design.md not found in specs
   - PASS: spec-format-valid

   ### Result

   6 passed, 2 failed
   Status: NOT READY (fix failures before implementation)
   ```

7. **Update status**

   If **all checks pass**:
   - Update `.specify/changes/<name>/.metadata.yaml`: set `status` to `reviewed`
   - Report: "All checks passed. Status set to `reviewed`. Run `/spec:apply` to start implementation."

   If **any check fails**:
   - Do NOT change the status
   - Report the failures and suggest fixes:
     - For missing artifacts: "Run `/spec:propose <name> <artifact-id>` to regenerate."
     - For spec format issues: "Edit the spec file to match the required structure."
     - For cross-artifact issues: "Update the referenced artifact to fix the inconsistency."
   - Offer to attempt automatic fixes for simple failures (missing scenarios, normative language) if the user confirms.

## Guardrails

- Read-only until the final status update — do not modify artifact files unless the user requests automatic fixes
- Run ALL checks before reporting — do not stop at the first failure
- Use heading conventions from `schema.yaml`'s `spec-format` — do not hard-code heading patterns
- If `validate-checks` is not defined for an artifact, skip structured checks for that artifact (fall back to `validate` string rules)
- Always report both passes and failures for full visibility
- The `reviewed` status is optional in the workflow — `/spec:apply` accepts both `proposed` and `reviewed` as valid entry states
