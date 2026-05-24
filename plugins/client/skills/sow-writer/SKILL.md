---
name: client-sow-writer
description: Generate a Statement of Work (SoW) document from Specify artifacts and project context. Use when a slice (or change) is fully defined and the operator wants to package its artifacts as a client deliverable; not while artifacts are still being authored (`define`) or implemented (`build`).
argument-hint: <slice-dir> [output-path] [client-name] [company-name] [pdf]
---

# SoW Generator Skill

## Authority

The SoW translates technical artifacts into business-oriented deliverables. Do not reproduce implementation details; focus on what the client receives, not how it is built. The output Markdown must match the standard $COMPANY_NAME template structure and writing voice and is suitable for export to Google Docs.

## Writing Style

The SoW must match the $COMPANY_NAME house style — first person plural, direct prose, business language, lettered items for exclusions/dependencies/assumptions, and section-specific prose patterns. See [`template.md`](template.md) for the full voice-and-tone rules and per-section formatting patterns. Follow it consistently when drafting any SoW section.

Defaults: `<output-path>` derives to `dirname(<slice-dir>)/../SOW-basename(<slice-dir>).md`; `<client-name>` falls back to `"unknown — to be confirmed"`; `<company-name>` defaults to `"Propellerhead"`; `pdf` is a literal positional that, when present, also renders a branded PDF. Project name and source reference are extracted from `design.md` at runtime.

## Process

Read the Specify artifacts from `$SLICE_DIR`, validate Context plus at least one Business Logic or Requirements source, determine `code-analysis` vs `requirements` origin, derive runtime variables/defaults, then follow [`references/section-templates.md`](references/section-templates.md) from cover page through optional PDF rendering; it is the source of truth for section order, boilerplate, validation cues, output summary, and optional PDF rendering.

Generated SoWs are drafts for Client Strategist review. Use placeholder costs only, include Appendix A verbatim from [`references/sow-template.md`](references/sow-template.md), include an Automated Test Suite deliverable, and turn material `[unknown]` tokens into assumptions or exclusions.

## Reference Documentation

- [section-templates.md](references/section-templates.md) — Verbatim Markdown templates and standard clauses for every step above.
- [sow-template.md](references/sow-template.md) — Complete SoW document template with standard sections and boilerplate text.
- [specify-to-sow-mapping.md](references/specify-to-sow-mapping.md) — Detailed mapping from artifact sections to SoW sections with examples.

## Examples

1. [migration-sow.md](examples/migration-sow.md) — SoW generated from `code-analysis` artifacts (TypeScript to Rust migration).
2. [greenfield-sow.md](examples/greenfield-sow.md) — SoW generated from `requirements` artifacts (design document).
