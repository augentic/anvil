# SoW Section Templates

Verbatim Markdown templates and standard clauses that the SoW skill drops into the generated document. The skill body links here once and then composes against these blocks instead of repeating them inline.

## Cover Page

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

## Background framing

- **Migration** (`code-analysis` origin): Frame as modernisation of an existing system. Reference the source system, what it does, and the target platform (Rust WASM / Omnia). Close with "This Statement of Work defines the scope of the $PROJECT_NAME migration in accordance with the specifications provided by $CLIENT_NAME."
- **Greenfield** (`requirements` origin): Frame as new adapter delivery. Reference the business need from the Component purpose. Close with "This Statement of Work defines the scope of the $PROJECT_NAME delivery in accordance with the requirements specified in $SOURCE_REFERENCE."

Use the Component purpose summary and any context from the design.md Notes section.

## Objectives prose

Write objectives as a **single prose paragraph** (not bullet points). Open with:

> "The objective of this Statement of Work is to deliver/migrate..."

Summarise the key outcomes in flowing prose. Reference the main deliverables and what they achieve for the client.

## Reference Agreement

```markdown
### Reference Agreement

This Statement of Work is subject to the terms and conditions set out in the Master Services Agreement [AGREEMENT NUMBER]. Where there is any conflict between this Statement of Work (including every Schedule or Appendix to this Statement of Work) and the Master Services Agreement (including every Schedule and Appendix of the Master Services Agreement), this Statement of Work shall prevail.
```

## Scope statement

Write a 1-paragraph scope statement that summarises what the engagement covers. Close with "The work is limited to the functionality and deliverables defined below and will be carried out in accordance with the specifications provided by $CLIENT_NAME."

Then add an **In Scope** sub-section listing the deliverable names as bullets:

```markdown
#### In Scope

- $DELIVERABLE_1
- $DELIVERABLE_2
- $DELIVERABLE_3
```

## Design Inputs

```markdown
### Design Inputs

| # | Referenced Document | Required By |
| --- | --- | --- |
| 1 | $DOCUMENT_1 | $DELIVERABLE(S) |
| 2 | $DOCUMENT_2 | $DELIVERABLE(S) |
```

For `code-analysis` artifacts, reference the source TypeScript files. For `requirements` artifacts, reference the requirements specifications. Also include any external API documentation.

## Deliverables table

```markdown
### Deliverables

| DESCRIPTION | COST |
| --- | --- |
| **$DELIVERABLE_TITLE** | $X,XXX (N d) |
| $NARRATIVE_DESCRIPTION — 2-4 sentences describing what this adapter does. Focus on what the client gets, not how it is built. | |
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
| Spec requirements / Business Logic | Core component adapters |
| API Contracts (design.md) | API endpoints |
| External Service Dependencies (design.md) | Integration adapters |
| Publication & Timing Patterns (design.md) | Event processing |
| Domain Model (design.md) | Domain types and validation |
| Requirements / BDD Scenarios (specs) | Feature adapters |

Always include an **Automated Test Suite** deliverable. Never invent cost figures — every value is `$X,XXX (N d)` for the Client Strategist.

## Exclusions

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

## Fees + Payment Schedule

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

## Dependencies

Use **"We need..."** phrasing. Each entry is a lettered item with a bold title:

```markdown
A. **$DEPENDENCY_TITLE**
   We need $WHAT_IS_NEEDED.
```

Sources:

- **External Service Dependencies (design.md)**: Access to APIs, environments, credentials.
- **Constants & Configuration (design.md)**: Environment-specific configuration the client must provide.
- **Implementation Requirements (design.md)**: Access to test environments, staging, production.

Standard dependencies (always include):

- Access to Environments — "We need reliable environments for all components, and the necessary services need to be readily available and accessible by $COMPANY_NAME in order to deliver this work in a timely manner and within the budget of this Statement of Work."
- $CLIENT_NAME Project Manager — "$CLIENT_NAME provides a Project Manager who can coordinate various project resources and help resolve any impediments to delivery."
- $CLIENT_NAME Product Owner — "$CLIENT_NAME will nominate a product owner to clarify requirements, prioritise deliverables, and accept completed work."

## Assumptions

Use **"It is assumed that..."** pattern. Each entry is a lettered item with a bold title:

```markdown
A. **$ASSUMPTION_TITLE**
   It is assumed that $ASSUMPTION_DETAIL.
```

Sources:

- **Design.md Notes section**: Any assumptions noted during analysis.
- **`[unknown]` tokens in artifacts**: Items where behaviour was assumed rather than confirmed.

Standard assumptions (always include):

- Deployment to Production — "It is assumed that deployment to production will follow the existing CI/CD pipeline. No new deployment infrastructure is required."
- No Material Architectural Changes — "It is assumed that the surrounding system architecture will not undergo material changes during the engagement."
- User Acceptance Testing — "While the delivery team will undertake unit and system testing, key business stakeholders are assumed to be available to assist with Functional Testing and final User Acceptance Testing."

## Change Requests / Warranty

Always include verbatim:

```markdown
### Change Requests/Processes

Any new changes will result in a new Statement of Work for the new scope of work.

### Warranty/Liability

N/A
```

## Acceptance signature blocks

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

## Appendix A — Standard Services and Deliverables

Read [sow-template.md](sow-template.md) and include the Standard Services and Deliverables appendix verbatim. Covers: Requirements, Architecture, Source Artefacts, Technical Contracts, Build and Deployment, Testing, Release Notes, System Documentation.

## Appendix B — Testing Guidelines (optional)

Generate only when test-related artifact content exists (e.g. crate-writer integration test output, BDD scenarios). Cover: test approach (unit / integration / end-to-end), test environment requirements, acceptance criteria mapping. Otherwise omit the appendix.

## Output summary

```text
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

## PDF rendering (optional)

When `$PDF_FLAG` is set:

1. Use Python with `reportlab` to render a branded PDF from `$OUTPUT_PATH`:
   - Read the Markdown content.
   - Apply branding (logo, headers/footers).
   - Page numbers + confidentiality line in the footer.
   - Render tables, headings, and lists with professional styling.
   - Write the PDF to `$OUTPUT_DIR/$BASENAME.pdf`.
2. Append `PDF generated: $OUTPUT_DIR/$BASENAME.pdf` to the summary.

Requires `reportlab` and `pypdf`.
