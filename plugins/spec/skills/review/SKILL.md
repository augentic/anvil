---
name: review
description: Validate artifacts with structured checks and cross-artifact consistency. Sets status to reviewed when all checks pass. Use when the user wants to verify artifact quality before implementation.
license: MIT
metadata:
  author: specify
  version: "1.0"
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

   Read `schema.yaml` for artifact definitions, `spec_format` heading conventions, `validate_checks`, and `cross_artifact_checks`.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `proposing`: warn that artifacts may be incomplete. Suggest running `/spec:propose` to finish first, but allow proceeding if user confirms.
   - If `status` is `reviewed`: inform the user that the change has already been reviewed. Offer to re-review.
   - If `status` is not `proposed` and not `proposing` and not `reviewed`: warn that review is normally run on `proposed` changes. Allow proceeding if user confirms.

3. **Read all artifacts**

   For each artifact defined in `schema.yaml`, read the file(s) at `.specify/changes/<name>/<generates>`. For glob patterns (e.g., `specs/**/*.md`), read all matching files in the directory.

   If an artifact file is missing, record it as a `MISSING` failure for that artifact and continue.

4. **Run per-artifact validate_checks**

   For each artifact that has a `validate_checks` field in `schema.yaml`, run every check against the artifact content. The check types are:

   **`heading_exists`** -- Verify the heading exists and has content below it.
   - `heading`: the markdown heading to find (exact match)
   - `min_content_lines`: minimum non-empty lines between this heading and the next heading (default 1)
   - `content_matches` (optional): regex pattern that must match at least one line in the section

   **`heading_exists_or_waived`** -- Like `heading_exists`, but the section may contain a waiver instead of content.
   - `heading`: the markdown heading to find
   - `waiver_pattern`: regex that, if matched in the section, counts as a valid waiver

   **`spec_structure`** -- Validate that spec files follow the heading hierarchy from `spec_format`.
   - `heading_ref`: reference to `spec_format.requirement_heading`
   - `id_ref`: reference to `spec_format.requirement_id_prefix`
   - `scenario_ref`: reference to `spec_format.scenario_heading`
   - For each requirement heading: verify an ID line follows immediately, then at least one scenario heading exists within the requirement block.

   **`requirement_has_id`** -- Every requirement heading must be followed by a line starting with the ID prefix, matching the pattern.
   - `pattern_ref`: reference to `spec_format.requirement_id_pattern`

   **`requirement_has_scenario`** -- Every requirement block must contain at least one scenario heading.
   - `scenario_ref`: reference to `spec_format.scenario_heading`

   **`normative_language`** -- Requirement text must use normative terms.
   - `required_terms`: at least one of these must appear in each requirement's description text
   - `forbidden_as_normative`: these terms must not be used as normative language (informational use is acceptable)

   **`scenario_keywords`** -- Scenario content must include the required keywords.
   - `required`: list of keywords (e.g., `["WHEN", "THEN"]`) — each must appear in the scenario block

   **`pattern_match`** -- Lines in a specific scope must match a regex.
   - `scope`: which lines to check (e.g., `task_lines`, `crate_names`, `capability_names`)
   - `pattern`: regex each matching line must satisfy

   **`heading_match`** -- The file must contain headings matching a pattern.
   - `pattern`: regex for heading lines
   - `min_count`: minimum number of matching headings

   **`min_count`** -- Minimum number of lines matching a pattern in a scope.
   - `scope`: which lines to check
   - `pattern`: regex to match
   - `min`: minimum count

   Record each check result as **PASS** or **FAIL** with a reason.

5. **Run cross-artifact consistency checks**

   If the schema defines `cross_artifact_checks`, run each one:

   **`proposal_crates_have_specs`** / **`proposal_capabilities_have_specs`** -- For every crate or capability listed in the proposal (under New or Modified headings), verify a corresponding spec file exists at `specs/<name>/spec.md` in the change directory. Report any crates/capabilities without specs.

   **`design_references_valid`** -- Scan `design.md` for requirement ID references matching the `spec_format.requirement_id_pattern` (e.g., `REQ-001`). For each referenced ID, verify it exists in one of the spec files. Report any orphaned references.

   **`spec_format_valid`** -- For every spec file in the change, validate the complete heading structure:
   - Every `### Requirement:` heading is followed by an `ID:` line
   - The ID matches `spec_format.requirement_id_pattern`
   - At least one `#### Scenario:` exists within each requirement block
   - No content appears outside of recognized sections

   Record each check result as **PASS** or **FAIL** with details.

6. **Produce review summary**

   Format the results as a structured report:

   ```text
   ## Review Summary: <change-name>

   ### Per-Artifact Checks

   **proposal.md**
   - PASS: heading_exists (## Why)
   - PASS: heading_exists (## Source)
   - FAIL: heading_exists (## Crates) — heading found but no content below it

   **specs/user-auth/spec.md**
   - PASS: spec_structure
   - PASS: requirement_has_id
   - FAIL: requirement_has_scenario — REQ-003 has no scenario

   **design.md**
   - PASS: heading_exists (## Context)
   - PASS: heading_exists_or_waived (## Domain Model)

   **tasks.md**
   - PASS: pattern_match (task_lines)
   - PASS: min_count (task_lines)

   ### Cross-Artifact Checks

   - PASS: proposal_crates_have_specs
   - FAIL: design_references_valid — REQ-005 referenced in design.md not found in specs
   - PASS: spec_format_valid

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
- Use heading conventions from `schema.yaml`'s `spec_format` — do not hard-code heading patterns
- If `validate_checks` is not defined for an artifact, skip structured checks for that artifact (fall back to `validate` string rules)
- Always report both passes and failures for full visibility
- The `reviewed` status is optional in the workflow — `/spec:apply` accepts both `proposed` and `reviewed` as valid entry states
