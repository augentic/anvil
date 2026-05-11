---
name: client-sow-writer
description: Generate a Statement of Work (SoW) document from Specify artifacts and project context. Use when a slice (or change) is fully defined and the operator wants to package its artifacts as a client deliverable; not while artifacts are still being authored (`define`) or implemented (`build`).
argument-hint: <slice-dir>
---

# SoW Generator Skill

## Critical Path

1. Read Specify artifacts at `$SLICE_DIR` (specs + `design.md`); validate required Context + Business Logic sections; determine `code-analysis` (migration) vs `requirements` (greenfield) origin.
2. Extract project metadata (name, purpose, source reference) and derive `$OUTPUT_PATH` from `$SLICE_DIR` unless one was supplied.
3. Compose the SoW shell — cover page, Background framed by origin, Objectives prose paragraph, Reference Agreement clause.
4. Generate Services — Scope statement + In-Scope bullets, Design Inputs table, Deliverables table with placeholder costs (always include an Automated Test Suite line item) and lettered Exclusions.
5. Generate Fees + Payment Schedule with placeholder amounts and the Other Issues block (Dependencies, Assumptions, Change Requests, Warranty).
6. Generate Acceptance, Appendix A (verbatim from `references/sow-template.md`), and Appendix B only when test-related artifact content is available.
7. Write Markdown to `$OUTPUT_PATH`, emit a review checklist, and optionally render a branded PDF when `pdf` is passed.

## Authority

The SoW translates technical artifacts into business-oriented deliverables. Do not reproduce implementation details; focus on what the client receives, not how it is built. The output Markdown must match the standard $COMPANY_NAME template structure and writing voice and is suitable for export to Google Docs.

## Writing Style

The SoW must match the $COMPANY_NAME house style — first person plural, direct prose, business language, lettered items for exclusions/dependencies/assumptions, and section-specific prose patterns. See [`template.md`](template.md) for the full voice-and-tone rules and per-section formatting patterns. Follow it consistently when drafting any SoW section.

## Derived Arguments

```text
$SLICE_DIR  = $ARGUMENTS[0]                           # Path to Specify slice directory
$OUTPUT_PATH = $ARGUMENTS[1] OR derive_from_change_dir # Output SoW path
$CLIENT_NAME  = $ARGUMENTS[2] OR "unknown — to be confirmed" # Client organisation name
$COMPANY_NAME = $ARGUMENTS[3] OR "Propellerhead"             # Company name (default: Propellerhead)
$PDF_FLAG     = "pdf" present in $ARGUMENTS                # Optional: also generate PDF
```

Path derivation:

```text
IF $OUTPUT_PATH not provided:
  $OUTPUT_DIR  = dirname($SLICE_DIR)/../
  $CRATE_NAME  = basename($SLICE_DIR)
  $OUTPUT_PATH = $OUTPUT_DIR/SOW-$CRATE_NAME.md

# Extracted from artifacts at runtime (Step 2):
$PROJECT_NAME     = design.md ## Context → Purpose summary
$SOURCE_REFERENCE = design.md header → Source field (if present)
```

## Process

Each step composes against the verbatim Markdown blocks in [section-templates.md](references/section-templates.md). Cite that file once and treat it as the source of truth for boilerplate.

### Step 1: Read and Validate Artifacts

Read the Specify artifacts from `$SLICE_DIR` (specs/ and design.md) and validate the minimum sections required for SoW generation.

**Required**: Context (Source, Purpose); at least one Business Logic Block.
**Optional but valuable**: API Contracts, External Service Dependencies, Constants & Configuration, Domain Model, Implementation Requirements, Publication & Timing Patterns.

If required sections are missing, fail with a clear error listing them. Determine artifact origin (`code-analysis` = migration, `requirements` = greenfield); the choice frames Background and Scope narrative.

### Step 2: Extract Project Metadata

Pull from the artifacts:

- **Project Name** — design.md `## Context` → Purpose summary (extract a name where possible, otherwise reuse the Purpose).
- **Project Purpose** — design.md `## Context` → Purpose summary.
- **Artifact Origin** — design.md header → migration vs greenfield framing.
- **Source Reference** — design.md header → Source field (repo URL or design document path).

### Step 3: Generate Cover Page

Drop in the cover-page template from [section-templates.md](references/section-templates.md#cover-page) with `$CLIENT_NAME`, `$PROJECT_NAME`, `$COMPANY_NAME`, `$TODAY` (DD MMMM YYYY), and `$VERSION_DATE` (YYYYMMDD_1) substituted.

### Step 4: Generate Introduction

Compose:

- **Background** — 2-3 paragraphs framed by artifact origin per [section-templates.md → Background framing](references/section-templates.md#background-framing). Greenfield framing references `$SOURCE_REFERENCE`.
- **Objectives** — single prose paragraph using the opener in [section-templates.md → Objectives prose](references/section-templates.md#objectives-prose).
- **Reference Agreement** — verbatim conflict-resolution clause from [section-templates.md → Reference Agreement](references/section-templates.md#reference-agreement).

### Step 5: Generate Services

Compose Scope (with In-Scope bullets), Design Inputs table, Deliverables table, and Exclusions block per the matching sections in [section-templates.md](references/section-templates.md). Decision points:

- Always include an **Automated Test Suite** deliverable.
- Never invent cost figures — every cost cell stays as `$X,XXX (N d)` for the Client Strategist.
- Order Exclusions: domain-specific first (derived from artifacts and `[unknown]` tokens), then the standard exclusions list.

### Step 6: Generate Fees

Drop in the Fees + Payment Schedule block from [section-templates.md → Fees + Payment Schedule](references/section-templates.md#fees--payment-schedule). Leave all monetary values as placeholders for the Client Strategist.

### Step 7: Generate Other Issues

Compose Dependencies ("We need…"), Assumptions ("It is assumed that…"), Change Requests/Processes, and Warranty/Liability per the matching sections in [section-templates.md](references/section-templates.md). Domain-specific entries first (derived from External Service Dependencies, Constants & Configuration, Implementation Requirements, Notes), then the standard entries listed in the reference.

### Step 8: Generate Acceptance

Drop in the signature blocks from [section-templates.md → Acceptance signature blocks](references/section-templates.md#acceptance-signature-blocks).

### Step 9: Generate Appendix A

Read [sow-template.md](references/sow-template.md) and include the Standard Services and Deliverables appendix verbatim. See [section-templates.md → Appendix A](references/section-templates.md#appendix-a--standard-services-and-deliverables) for the section coverage list.

### Step 10: Generate Appendix B (Optional)

Generate only when the artifacts contain test-related content (crate-writer integration test output, acceptance criteria with BDD scenarios). See [section-templates.md → Appendix B](references/section-templates.md#appendix-b--testing-guidelines-optional) for the coverage list. Otherwise omit.

### Step 11: Write Output

Write the complete SoW to `$OUTPUT_PATH` and emit the summary report from [section-templates.md → Output summary](references/section-templates.md#output-summary).

### Step 12: Generate PDF (Optional)

If `$PDF_FLAG` is set, follow the [section-templates.md → PDF rendering](references/section-templates.md#pdf-rendering-optional) procedure. Requires Python with `reportlab` and `pypdf` packages installed.

## Reference Documentation

- [section-templates.md](references/section-templates.md) — Verbatim Markdown templates and standard clauses for every step above.
- [sow-template.md](references/sow-template.md) — Complete SoW document template with standard sections and boilerplate text.
- [specify-to-sow-mapping.md](references/specify-to-sow-mapping.md) — Detailed mapping from artifact sections to SoW sections with examples.

## Examples

1. [migration-sow.md](examples/migration-sow.md) — SoW generated from `code-analysis` artifacts (TypeScript to Rust migration).
2. [greenfield-sow.md](examples/greenfield-sow.md) — SoW generated from `requirements` artifacts (design document).

## Error Handling

| Issue | Cause | Resolution |
| ----- | ----- | ---------- |
| Artifacts not found | Invalid `$SLICE_DIR` | Verify path and re-run |
| Missing Context section | Artifacts are incomplete or malformed | Run the appropriate analyzer skill first |
| No Business Logic or Requirements | Artifacts lack actionable content | Re-run the appropriate analyzer skill to enrich artifacts before generating SoW |
| Artifacts have many `[unknown]` tokens | Analysis was incomplete | Note unknowns as assumptions in the SoW; flag for Client Strategist review |
| Cannot determine project type | Artifact origin header missing | Default to `requirements` framing; note in SoW |

## Verification Checklist

- [ ] **Cover page**: Client name, project name, metadata table (Author, Date, Project Manager, Version)
- [ ] **Background**: Accurately frames the project context (migration vs greenfield)
- [ ] **Objectives**: Single prose paragraph opening with "The objective of this Statement of Work..."
- [ ] **Reference Agreement**: Includes conflict resolution clause with agreement number placeholder
- [ ] **Scope**: Includes "In Scope" bullet list of deliverable names
- [ ] **Design Inputs**: Referenced documents table present
- [ ] **Deliverables**: Table with DESCRIPTION | COST columns; each deliverable has bold title, narrative, and "Specifically:" items
- [ ] **Deliverables**: Cost placeholders present alongside each deliverable (no invented figures)
- [ ] **Exclusions**: Terse, lettered items with domain-specific and standard exclusions
- [ ] **Fees**: Placeholder values for all monetary amounts
- [ ] **Dependencies**: "We need..." voice, lettered items
- [ ] **Assumptions**: "It is assumed that..." pattern, lettered items
- [ ] **Change Requests/Processes**: Present
- [ ] **Warranty/Liability**: Present (N/A)
- [ ] **Acceptance**: Authorised Person / Signature / Date format for both parties
- [ ] **Appendix A**: Standard Services and Deliverables included
- [ ] **No technical jargon**: SoW uses business language throughout
- [ ] **No cost invention**: All costs are placeholders — never generate monetary values
- [ ] **Traceability**: Each deliverable can be traced back to artifact content

## Guardrails

- **Never invent costs.** All monetary values are placeholders. Cost estimation is the Client Strategist's responsibility.
- **Business language.** Translate technical artifact content into client-facing language. "Implement Handler<P> trait with HttpRequest provider" becomes "Integration with external API".
- **$COMPANY_NAME voice.** Use "we" for $COMPANY_NAME, direct and concise sentences, no filler.
- **Standard boilerplate is canonical.** The Reference Agreement, Fees terms, Change Requests, Warranty, and Acceptance sections are consistent across all SoWs and must come from the templates verbatim.
- **Client Strategist review.** The generated SoW is a draft. It must be reviewed and completed by the Client Strategist before being sent to the client.
- **Appendix A is fixed.** Include the Standard Services and Deliverables appendix verbatim from [sow-template.md](references/sow-template.md).
- **Exclusions protect scope.** Be thorough; better to explicitly exclude something than to leave scope ambiguous. Keep descriptions terse.
- **`[unknown]` tokens become assumptions.** Each unknown represents a decision that needs client confirmation.
