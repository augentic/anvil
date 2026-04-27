# Iterating on a Baseline

In the [previous tutorial](first-change.md), you created a greeting capability and merged it into the baseline. Now you will modify that capability, learning how delta specs work and how to verify that your code matches the baseline.

**Prerequisites:** Complete [Your First Change](first-change.md). You should have a baseline spec at `.specify/specs/greeting/spec.md`.

## 1. Define a change that modifies an existing capability

```text
/spec:define "Add support for an optional language parameter to the greeting endpoint, defaulting to English"
```

<details>
<summary>Expected output</summary>

```text
Change created: add-language-parameter

Baseline detected: specs/greeting/spec.md
  Generating delta spec...

Generating artifacts...
  ✓ proposal.md
  ✓ specs/greeting/spec.md (delta)
  ✓ design.md
  ✓ tasks.md

Change defined (3 tasks).
```

</details>

This time, Specify detects that the `greeting` capability already exists in the baseline. Instead of writing a fresh spec, it generates a **delta spec** (see [Glossary](../appendices/glossary.md)) that describes only what changed.

Open `.specify/changes/<name>/specs/greeting/spec.md` and notice the delta structure:

```markdown
## ADDED Requirements

### Requirement: Language Parameter

ID: REQ-003

The system SHALL accept an optional language parameter and
return the greeting in the specified language.

#### Scenario: Language specified

- **WHEN** a valid name and language "es" are provided
- **THEN** return "Hola, {name}!"

#### Scenario: Language not specified

- **WHEN** a valid name is provided without a language parameter
- **THEN** return the greeting in English (default)

## MODIFIED Requirements

### Requirement: Greeting Response

ID: REQ-001

The system SHALL accept a name and an optional language parameter
and return a personalised greeting in the specified language.
```

Key observations:

- **`ADDED`** sections introduce new requirements with new IDs.
- **`MODIFIED`** sections reference existing requirements by their stable `REQ-XXX` ID.
- The original `REQ-001` is being updated, not replaced. The ID stays the same.
- Requirements that did not change are not mentioned in the delta.

## 2. Build and merge

The build and merge steps work exactly as before:

```text
/spec:build
/spec:merge
```

When merge runs, it applies the delta to the baseline. Open `.specify/specs/greeting/spec.md` afterwards and you will see:

- `REQ-001` now includes the language parameter (modified).
- `REQ-003` is a new requirement in the baseline (added).
- `REQ-002` (the error case from the first change) is unchanged.

The baseline is the accumulated state of all merged changes.

## 3. Verify the baseline

After merging, confirm that your code matches the updated specs:

```text
/spec:verify
```

<details>
<summary>Expected output</summary>

```text
Verifying baseline...

  greeting:
    REQ-001  COVERED  Greeting Response
    REQ-002  COVERED  Empty Name Error
    REQ-003  COVERED  Language Parameter

Summary: 3 requirements, 3 covered, 0 drifted, 0 missing
```

</details>

Verify reads each baseline spec and compares it against the implementation. It classifies each requirement:

| Classification | Meaning |
|---------------|---------|
| **COVERED** | Code implements the requirement as specified |
| **DRIFTED** | Code behavior diverges from the spec |
| **MISSING** | Requirement specified but not implemented |
| **UNSPECIFIED** | Code behavior exists with no spec |

If everything went well, all requirements should be `COVERED`. If someone later changes the greeting logic without going through Specify, a future `/spec:verify` will flag the drift.

## 4. Verify a specific capability

You can also verify a single capability:

```text
/spec:verify greeting
```

This is useful when you suspect drift in a particular area and do not want to verify the entire baseline.

## Understanding delta merges

The delta spec format is how Specify manages change over time without losing history. Here is how each operation works at merge time:

| Delta operation | Effect on baseline |
|----------------|-------------------|
| `ADDED` | New requirement block appended to the baseline spec |
| `MODIFIED` | Existing requirement block (matched by ID) is replaced |
| `REMOVED` | Requirement block (matched by ID) is removed, with reason recorded |
| `RENAMED` | Requirement title updated, ID preserved |

The merge key (see [Glossary](../appendices/glossary.md)) is always the `ID: REQ-XXX` line, not the requirement title. Titles can change freely across deltas.

## What you learned

- Changes against existing capabilities produce **delta specs**, not full rewrites.
- Delta sections (`ADDED`, `MODIFIED`, `REMOVED`, `RENAMED`) describe precisely what changed.
- Stable `REQ-XXX` IDs are the merge keys that connect deltas to the baseline.
- `/spec:verify` checks whether your code still matches the accumulated baseline.
- The baseline is the cumulative record of all merged specs.

## Next

[Thinking Before Defining](thinking-before-defining.md) -- use explore mode to investigate a problem before committing to a change.
