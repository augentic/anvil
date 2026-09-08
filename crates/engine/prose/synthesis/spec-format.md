# Spec format

Hard-coded conventions of the rendered `spec.md`. These are not configurable and none of them is yours to write.

- **Title**: `# Specification`, then your `preamble` paragraphs.
- **Requirement heading**: `### Requirement: <subject>[ <tag>]`
- **Provenance lines**: `ID: REQ-NNN`, `Sources: [<source>, …]`, `Status: <status>`
- **Scenario heading**: `#### Scenario: <name>`, then `- **GIVEN**` / `- **WHEN**` / `- **THEN**` bullets

Your `preamble` opens the document: a short overview of what was bound and what the rows say, as paragraphs. One flat document: no delta sections, no per-domain splits — `spec.md` is the whole reviewable set.
