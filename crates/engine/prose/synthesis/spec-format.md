# Spec format

Hard-coded heading conventions the fail-closed spec parser enforces. These are not configurable.

- **Requirement heading**: `### Requirement:`
- **Requirement ID prefix**: `ID:`
- **Requirement ID pattern**: `^REQ-[0-9]{3}$`
- **Scenario heading**: `#### Scenario:`

A requirement block starts at a `### Requirement:` heading and continues until the next requirement heading or end of file. Open the document with a short title and overview before the first block; the parser treats everything before the first heading as preamble.

One flat document: no delta sections, no per-domain splits — `spec.md` is the whole reviewable set.
