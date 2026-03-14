# Check Type Definitions

Structured check types used by `validate-checks` and `cross-artifact-checks`
in `schema.yaml`. Skills that run or reference these checks (review, propose)
should consult this document for semantics and parameters.

## Per-Artifact Check Types

These checks run against individual artifact files.

**`heading-exists`** -- Verify the heading exists and has content below it.
- `heading`: the markdown heading to find (exact match)
- `min-content-lines`: minimum non-empty lines between this heading and the next heading (default 1)
- `content-matches` (optional): regex pattern that must match at least one line in the section

**`heading-exists-or-waived`** -- Like `heading-exists`, but the section may contain a waiver instead of content.
- `heading`: the markdown heading to find
- `waiver-pattern`: regex that, if matched in the section, counts as a valid waiver

**`spec-structure`** -- Validate that spec files follow the heading hierarchy from `spec-format`.
- `heading-ref`: reference to `spec-format.requirement-heading`
- `id-ref`: reference to `spec-format.requirement-id-prefix`
- `scenario-ref`: reference to `spec-format.scenario-heading`
- For each requirement heading: verify an ID line follows immediately, then at least one scenario heading exists within the requirement block.

**`requirement-has-id`** -- Every requirement heading must be followed by a line starting with the ID prefix, matching the pattern.
- `pattern-ref`: reference to `spec-format.requirement-id-pattern`

**`requirement-has-scenario`** -- Every requirement block must contain at least one scenario heading.
- `scenario-ref`: reference to `spec-format.scenario-heading`

**`normative-language`** -- Requirement text must use normative terms.
- `required-terms`: at least one of these must appear in each requirement's description text
- `forbidden-as-normative`: these terms must not be used as normative language (informational use is acceptable)

**`scenario-keywords`** -- Scenario content must include the required keywords.
- `required`: list of keywords (e.g., `["WHEN", "THEN"]`) — each must appear in the scenario block

**`pattern-match`** -- Lines in a specific scope must match a regex.
- `scope`: which lines to check (e.g., `task-lines`, `crate-names`, `capability-names`)
- `pattern`: regex each matching line must satisfy

**`heading-match`** -- The file must contain headings matching a pattern.
- `pattern`: regex for heading lines
- `min-count`: minimum number of matching headings

**`min-count`** -- Minimum number of lines matching a pattern in a scope.
- `scope`: which lines to check
- `pattern`: regex to match
- `min`: minimum count

## Cross-Artifact Check Types

These checks validate consistency across multiple artifacts.

**`proposal-crates-have-specs`** / **`proposal-capabilities-have-specs`** -- For every crate or capability listed in the proposal (under New or Modified headings), verify a corresponding spec file exists at `specs/<name>/spec.md` in the change directory. Report any crates/capabilities without specs.

**`design-references-valid`** -- Scan `design.md` for requirement ID references matching the `spec-format.requirement-id-pattern` (e.g., `REQ-001`). For each referenced ID, verify it exists in one of the spec files. Report any orphaned references.

**`spec-format-valid`** -- For every spec file in the change, validate the complete heading structure:
- Every `### Requirement:` heading is followed by an `ID:` line
- The ID matches `spec-format.requirement-id-pattern`
- At least one `#### Scenario:` exists within each requirement block
- No content appears outside of recognized sections
