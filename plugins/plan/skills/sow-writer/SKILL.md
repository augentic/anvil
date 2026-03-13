---
name: sow-writer
description: Generate a Statement of Work (SoW) document from Specify artifacts and project context.
argument-hint: [change-dir] [output-path?] [client-name?] [company-name?] [--pdf?]
allowed-tools: Read, Write, StrReplace, Shell, Grep
---

# SoW Generator Skill

## Overview

Generate a professional Statement of Work (SoW) document from Specify artifacts and project context. The SoW follows the standard $COMPANY_NAME template structure and writing voice, suitable for export to Google Docs.

This skill reads the Specify artifacts (specs and design.md) produced by `code-analyzer` or `epic-analyzer` and translates technical content into client-facing deliverables. The output is a Markdown document structured to match the standard $COMPANY_NAME SoW format.

**Key principle**: Translate technical artifacts into business-oriented deliverables. Do not reproduce implementation details; focus on what the client receives, not how it is built.

## Writing Style

The SoW must match the $COMPANY_NAME house style. Follow these rules consistently:

### Voice and Tone

- **First person plural**: Use "we" when referring to $COMPANY_NAME. "We need access to..." not "$COMPANY_NAME requires access to..."
- **Direct and concise**: Short sentences. No filler or padding. Get to the point.
- **Business language**: No technical jargon. Translate all technical terms from the artifacts into business language.
- **Confident but not presumptuous**: State facts and requirements clearly. Avoid hedging language.

### Section-specific Style

- **Objectives**: Write as a **single prose paragraph**, not bullet points. Open with "The objective of this Statement of Work is to deliver..."
- **Deliverables**: Each deliverable gets its **own heading** with a narrative paragraph describing what it does, followed by a "Sub-components" list using the pattern: **Bold Title** - description.
- **Exclusions**: Terse. Lettered items with a **bold title** and 1-2 sentence description. No lengthy explanations.
- **Dependencies**: Use "We need..." phrasing. Direct, personal voice. Brief.
- **Assumptions**: Use "It is assumed that..." pattern. Concise. Do NOT append "If this assumption proves incorrect, we will raise a change request" to every item.

### Formatting Patterns

- **Lettered items** (A, B, C...) for Exclusions, Dependencies, and Assumptions — each with a bold title on the first line and description below.
- **Sub-components** use bullet list with **Bold Title** - description pattern.
- **Costs** are listed alongside each deliverable in the deliverables table (DESCRIPTION | COST format).
- **Design Inputs** table lists referenced documents with links.

## Derived Arguments

```text
$CHANGE_DIR  = $ARGUMENTS[0]                           # Path to Specify change directory
$OUTPUT_PATH = $ARGUMENTS[1] OR derive_from_change_dir # Output SoW path
$CLIENT_NAME  = $ARGUMENTS[2] OR "unknown — to be confirmed" # Client organisation name
$COMPANY_NAME = $ARGUMENTS[3] OR "Propellerhead"             # Company name (default: Propellerhead)
$PDF_FLAG     = "--pdf" present in $ARGUMENTS                # Optional: also generate PDF
```

Path derivation:

```text
IF $OUTPUT_PATH not provided:
  $OUTPUT_DIR  = dirname($CHANGE_DIR)/../
  $CRATE_NAME  = basename($CHANGE_DIR)
  $OUTPUT_PATH = $OUTPUT_DIR/SOW-$CRATE_NAME.md
```

## Process

### Step 1: Read and Validate Artifacts

Read the Specify artifacts from `$CHANGE_DIR` (specs/ and design.md) and validate that they contain the minimum sections required for SoW generation:

**Required sections**:

- Context (Source, Purpose)
- Business Logic Blocks OR User Stories (at least one)

**Optional but valuable sections**:

- API Contracts
- External Service Dependencies
- Constants & Configuration
- Domain Model
- Implementation Requirements
- Publication & Timing Patterns

**If required sections are missing**: Fail with a clear error listing the missing sections.

**Determine artifact origin**:

- `code-analysis` — Migration project (TypeScript to Rust WASM)
- `requirements` — Greenfield development (from JIRA epic or design document)

The artifact origin informs the Background and Scope narrative.

### Step 2: Extract Project Metadata

From the artifacts, extract:

- **Project Name**: From design.md `## Context` → Purpose summary (extract name if possible, otherwise use Purpose).
- **Project Purpose**: From design.md `## Context` → Purpose summary.
- **Artifact Origin**: From design.md header → determines migration vs greenfield framing.
- **Source Reference**: From design.md header → Source field (repo URL, JIRA epic key, or design document path).

### Step 3: Generate Cover Page

Write the SoW cover page with metadata table:

```markdown
# Statement of Work

**Client**: $CLIENT_NAME
**Project**: $PROJECT_NAME

| | |
| --- | --- |
| Author: | [Author], $COMPANY_NAME |
| Date: | $TODAY |
| Project Manager: | [Project Manager] |
| Version: | $VERSION_DATE |
```

Use today's date in `DD MMMM YYYY` format. Version uses `YYYYMMDD_1` format.

### Step 4: Generate Introduction

#### Background

Compose a 2-3 paragraph background based on artifact origin:

- **Migration** (`code-analysis` origin): Frame as modernisation of an existing system. Reference the source system, what it does, and the target platform (Rust WASM / Omnia). Close with "This Statement of Work defines the scope of the $PROJECT_NAME migration in accordance with the specifications provided by $CLIENT_NAME."
- **Greenfield** (`requirements` origin): Frame as new capability delivery. Reference the business need from User Stories or Component purpose. Close with "This Statement of Work defines the scope of the $PROJECT_NAME delivery in accordance with the requirements specified in $SOURCE_REFERENCE."

Use the Component purpose summary and any context from the design.md Notes section.

#### Objectives

Write objectives as a **single prose paragraph** (not bullet points). Open with:

> "The objective of this Statement of Work is to deliver/migrate..."

Summarise the key outcomes in flowing prose. Reference the main deliverables and what they achieve for the client.

#### Reference Agreement

Write with conflict resolution clause:

```markdown
### Reference Agreement

This Statement of Work is subject to the terms and conditions set out in the Master Services Agreement [AGREEMENT NUMBER]. Where there is any conflict between this Statement of Work (including every Schedule or Appendix to this Statement of Work) and the Master Services Agreement (including every Schedule and Appendix of the Master Services Agreement), this Statement of Work shall prevail.
```

### Step 5: Generate Services

#### Scope

Write a 1-paragraph scope statement that summarises what the engagement covers. Close with "The work is limited to the functionality and deliverables defined below and will be carried out in accordance with the specifications provided by $CLIENT_NAME."

Then add an **In Scope** sub-section listing the deliverable names as bullets:

```markdown
#### In Scope

- $DELIVERABLE_1
- $DELIVERABLE_2
- $DELIVERABLE_3
```

#### Design Inputs

Generate a table of referenced documents that inform the deliverables:

```markdown
### Design Inputs

| # | Referenced Document | Required By |
| --- | --- | --- |
| 1 | $DOCUMENT_1 | $DELIVERABLE(S) |
| 2 | $DOCUMENT_2 | $DELIVERABLE(S) |
```

For `code-analysis` artifacts, reference the source TypeScript files. For `requirements` artifacts, reference the JIRA epic and stories. Also include any external API documentation.

#### Deliverables

Create a deliverables table with cost alongside each deliverable. Each deliverable row contains a bold title with cost, a narrative description, then "Specifically:" with bullet items:

```markdown
### Deliverables

| DESCRIPTION | COST |
| --- | --- |
| **$DELIVERABLE_TITLE** | $X,XXX (N d) |
| $NARRATIVE_DESCRIPTION — 2-4 sentences describing what this capability does. Focus on what the client gets, not how it is built. | |
| Specifically: | |
| - $SPECIFIC_ITEM_1 | |
| - $SPECIFIC_ITEM_2 | |
| | |
| **$NEXT_DELIVERABLE** | $X,XXX (N d) |
| ... | |
```

Map artifact sections to deliverables:

| Artifact Source | Deliverable |
| --------------- | ----------- |
| Spec requirements / Business Logic | Core component capabilities |
| API Contracts (design.md) | API endpoints |
| External Service Dependencies (design.md) | Integration capabilities |
| Publication & Timing Patterns (design.md) | Event processing |
| Domain Model (design.md) | Domain types and validation |
| User Stories / BDD Scenarios (specs) | Feature capabilities |

Always include an **Automated Test Suite** deliverable.

**Never invent cost figures.** All values must be placeholders (`$X,XXX (N d)`) for the Client Strategist.

#### Exclusions from Scope

Generate exclusions by identifying what the artifacts explicitly do NOT cover. Write each exclusion as a **lettered item** with a **bold title** and a **terse 1-2 sentence description**:

```markdown
A. **$EXCLUSION_TITLE**
   $BRIEF_DESCRIPTION.

B. **$EXCLUSION_TITLE**
   $BRIEF_DESCRIPTION.
```

Domain-specific exclusions first (derived from artifacts), then standard exclusions:

- Items tagged `[unknown]` with significant scope implications
- Adjacent systems mentioned in External Service Dependencies that are not being modified
- Operational concerns (monitoring, alerting, on-call support)
- Data migration or backfill
- Performance and load testing

Standard exclusions (always include):

- User Acceptance Testing — "We expect $CLIENT_NAME to undertake User Acceptance Testing, though we will provide necessary support."
- Design Review Board (DRB) — "DRB activities and approvals are excluded."
- Project Management — "Day-to-day project management activities are excluded unless separately agreed."
- Dependency and Environment Management — "Resolution of issues relating to $CLIENT_NAME environments, third-party configuration, networking, or vendor infrastructure is excluded unless separately agreed."
- Additional Minor Features — "Features or enhancements not described in the Deliverables section above are excluded, which we can address separately."

### Step 6: Generate Fees

Write the Fees section with placeholder values:

```markdown
## Fees

The fees (Fees) payable by the Customer for the Services will be charged on a **Fixed Price** basis.

| Fee | $TOTAL + GST |
| --- | --- |

Notes:
- All amounts specified above exclude GST.
- Unless otherwise agreed, no amounts will be withheld by $CLIENT_NAME as project retention amounts.

### Payment Schedule

Fees will be invoiced on a monthly basis for the services rendered within that month to a maximum amount specified as the Fee (see above), and payments will be due on the 20th of the following month of the invoice date.

| Month | Payment |
| --- | --- |
| $MONTH_1 | $AMOUNT_1 |
```

Leave all monetary values as placeholders for the Client Strategist.

### Step 7: Generate Other Issues

#### Dependencies

Extract dependencies from the artifacts. Use **"We need..."** phrasing. Write each as a lettered item with a bold title:

```markdown
A. **$DEPENDENCY_TITLE**
   We need $WHAT_IS_NEEDED.
```

Sources:
- **From External Service Dependencies (design.md)**: Access to APIs, environments, credentials
- **From Constants & Configuration (design.md)**: Environment-specific configuration that the client must provide
- **From Implementation Requirements (design.md)**: Access to test environments, staging, production

Standard dependencies (always include):

- Access to Environments — "We need reliable environments for all components, and the necessary services need to be readily available and accessible by $COMPANY_NAME in order to deliver this work in a timely manner and within the budget of this Statement of Work."
- $CLIENT_NAME Project Manager — "$CLIENT_NAME provides a Project Manager who can coordinate various project resources and help resolve any impediments to delivery."
- $CLIENT_NAME Product Owner — "$CLIENT_NAME will nominate a product owner to clarify requirements, prioritise deliverables, and accept completed work."

#### Assumptions

Extract assumptions using **"It is assumed that..."** pattern. Write each as a lettered item with a bold title:

```markdown
A. **$ASSUMPTION_TITLE**
   It is assumed that $ASSUMPTION_DETAIL.
```

Sources:
- **Design.md Notes section**: Any assumptions noted during analysis
- **`[unknown]` tokens in artifacts**: Items where behaviour was assumed rather than confirmed

Standard assumptions (always include):
- Deployment to Production — "It is assumed that deployment to production will follow the existing CI/CD pipeline. No new deployment infrastructure is required."
- No Material Architectural Changes — "It is assumed that the surrounding system architecture will not undergo material changes during the engagement."
- User Acceptance Testing — "While the delivery team will undertake unit and system testing, key business stakeholders are assumed to be available to assist with Functional Testing and final User Acceptance Testing."

#### Change Requests/Processes

Always include:

```markdown
### Change Requests/Processes

Any new changes will result in a new Statement of Work for the new scope of work.
```

#### Warranty/Liability

Always include:

```markdown
### Warranty/Liability

N/A
```

### Step 8: Generate Acceptance

Write the acceptance section with signature blocks using Authorised Person format:

```markdown
## Acceptance

### SIGNED by for and on behalf of $COMPANY_NAME by:

| | |
| --- | --- |
| Authorised Person | |
| Signature | |
| Date | |

### SIGNED by for and on behalf of $CLIENT_NAME by:

| | |
| --- | --- |
| Authorised Person | |
| Signature | |
| Date | |
```

### Step 9: Generate Appendix A — Standard Services and Deliverables

Write Appendix A with the standard $COMPANY_NAME service descriptions. Read the template from [sow-template.md](references/sow-template.md) and include the Standard Services and Deliverables appendix verbatim.

This appendix covers:

- Requirements
- Architecture
- Source Artefacts
- Technical Contracts
- Build and Deployment
- Testing
- Release Notes
- System Documentation

### Step 10: Generate Appendix B — Testing Guidelines (Optional)

If the artifacts contain test-related information (e.g., from crate-writer's integration test output or User Story acceptance criteria with BDD scenarios), generate a Testing Guidelines appendix covering:

- Test approach (unit, integration, end-to-end)
- Test environment requirements
- Acceptance criteria mapping

If no test information is available, omit this appendix.

### Step 11: Write Output

Write the complete SoW to `$OUTPUT_PATH`.

Report a summary to the user:

```
SoW generated: $OUTPUT_PATH
Client: $CLIENT_NAME
Project: $PROJECT_NAME
Deliverables: $N items
Dependencies: $N items
Assumptions: $N items

Review required:
- [ ] Cost figures (all placeholders)
- [ ] Payment schedule
- [ ] Reference agreement number
- [ ] Author and Project Manager names
- [ ] Signature blocks
```

### Step 12: Generate PDF (Optional)

If `$PDF_FLAG` is set (`--pdf` was provided):

1. Use Python with `reportlab` to create a branded PDF from the generated Markdown:

   - Read the Markdown content from `$OUTPUT_PATH`
   - Create a PDF with branding (logo, headers/footers)
   - Include page numbers and confidentiality line in the footer
   - Render all tables, headings, and lists with professional styling
   - Write the PDF to `$OUTPUT_DIR/$BASENAME.pdf`

2. Append to the summary report:

```
PDF generated: $OUTPUT_DIR/$BASENAME.pdf
```

**Note**: Requires Python with `reportlab` and `pypdf` packages installed for PDF generation.

## Reference Documentation

- **[sow-template.md](references/sow-template.md)** — Complete SoW document template with standard sections and boilerplate text
- **[specify-to-sow-mapping.md](references/specify-to-sow-mapping.md)** — Detailed mapping from artifact sections to SoW sections with examples

## Examples

1. [migration-sow.md](examples/migration-sow.md) — SoW generated from `code-analysis` artifacts (TypeScript to Rust migration)
2. [greenfield-sow.md](examples/greenfield-sow.md) — SoW generated from `requirements` artifacts (JIRA epic)

## Error Handling

| Issue | Cause | Resolution |
| ----- | ----- | ---------- |
| Artifacts not found | Invalid `$CHANGE_DIR` | Verify path and re-run |
| Missing Context section | Artifacts are incomplete or malformed | Run the appropriate analyzer skill first |
| No Business Logic or User Stories | Artifacts lack actionable content | Re-run `epic-analyzer` to enrich artifacts before generating SoW |
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

## Important Notes

- **Never invent costs**: All monetary values must be placeholders. Cost estimation is the Client Strategist's responsibility.
- **Business language**: Translate technical artifact content into client-facing language. "Implement Handler<P> trait with HttpRequest provider" becomes "Integration with external API".
- **$COMPANY_NAME voice**: Use "we" for $COMPANY_NAME, direct and concise sentences, no filler.
- **Standard boilerplate**: The SoW includes standard $COMPANY_NAME sections (Reference Agreement, Fees terms, Change Requests, Warranty, Acceptance). These are consistent across all SoWs.
- **Client Strategist review**: The generated SoW is a draft. It must be reviewed and completed by the Client Strategist before being sent to the client.
- **Appendix A is standard**: The Standard Services and Deliverables appendix is the same across all SoWs and should be included verbatim from the template.
- **Exclusions protect scope**: Be thorough with exclusions. It is better to explicitly exclude something and later include it than to have scope ambiguity. Keep descriptions terse.
- **`[unknown]` tokens in artifacts**: Map these to assumptions in the SoW. Each unknown represents a decision that needs client confirmation.
