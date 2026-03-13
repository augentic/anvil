# SoW Document Template

This is the canonical Statement of Work template. The `sow-writer` skill populates this structure from Specify artifacts.

Sections marked `[standard]` are consistent across all SoWs. Sections marked `[generated]` are populated from Specify artifact content.

---

## Document Structure

```text
Cover Page                          [generated]
Introduction                        [generated]
  Background                        [generated]
  Objectives                        [generated]
  Reference Agreement               [standard]
Services                            [generated]
  Scope                             [generated]
  Deliverables                      [generated]
  Exclusions from Scope             [generated]
Fees                                [standard + placeholders]
  Payment Schedule                  [standard + placeholders]
Other Issues                        [generated]
  Dependencies                      [generated]
  Assumptions                       [generated]
Acceptance                          [standard]
Appendix A: Standard Services       [standard]
Appendix B: Testing Guidelines      [optional, generated]
```

---

## Standard Boilerplate Sections

### Reference Agreement

```markdown
### Reference Agreement

This Statement of Work is issued under the Master Services Agreement between Propellerhead Limited and $CLIENT_NAME dated [DATE].
```

### Fees

```markdown
## Fees

The fees (Fees) payable by the Customer for the Services will be charged on a **Fixed Price** basis.

| Fee | $TOTAL + GST |
| --- | ------------ |

Notes:
- All amounts specified above exclude GST.
- Unless otherwise agreed, $CLIENT_NAME will not withhold any amounts as project retention amounts.
```

### Payment Schedule

```markdown
### Payment Schedule

Fees will be invoiced monthly for services rendered within that month up to a maximum amount specified as the Fee (see above). Payments are due on the 20th of the following month after the invoice date.

| Month | Payment |
| ----- | ------- |
| $MONTH_1 | $AMOUNT_1 |
```

### Acceptance

```markdown
## Acceptance

### SIGNED by for and on behalf of Propellerhead Limited by:

| | |
| --- | --- |
| Name | |
| Title | |
| Date | |
| Signature | |

### SIGNED by for and on behalf of $CLIENT_NAME by:

| | |
| --- | --- |
| Name | |
| Title | |
| Date | |
| Signature | |
```

---

## Appendix A: Standard Services and Deliverables

This appendix describes key services and deliverables that underpin the engagement, improving the quality of software delivery and supportability.

Prior to delivering software, we need to outline the technical vision, validate our thinking, and create alignment in order to better coordinate effort. Aside from the delivery of working software, we ensure we are contributing to organisational memory around the software delivered.

### Requirements

Even when requirements have been created prior to our engagement, a large part of our success is dependent on our involvement in crafting outcome-focussed user stories, wireframes, and a simple usage narrative.

### Architecture

We document a high-level solution architecture as an aid to communicating key concepts and capturing key technical decisions. The straightforward approach is evolutionary rather than perfect and complete, allowing us to start small and evolve as the solution evolves.

We use Architectural Decision Records (ADRs) to capture key architectural decisions during the lifetime of the software, as it evolves.

### Source Artefacts

We deliver all application source artefacts including source code, test scripts, and build & deployment scripts.

### Technical Contracts

Technical contracts, or APIs, communicate system behaviour to external technical stakeholders. The documentation is typically generated either from the specification or from the code implementing the API.

### Build and Deployment

Our default approach to build and deployment automation uses GitHub Actions for continuous integration and continuous deployment (CI/CD). We create pipelines that build, test, and deploy the software automatically.

### Testing

We practice test-driven development (TDD) where practical. Our testing approach includes:

- **Unit tests**: Cover individual functions and business logic
- **Integration tests**: Verify component interactions and external service integrations
- **Contract tests**: Validate API contracts between services

We deliver all test scripts alongside the source code.

### Release Notes

We document changes in each release, including new features, bug fixes, and breaking changes. Release notes are maintained alongside the source code.

### System Documentation

We maintain system documentation that covers:

- **Architecture**: High-level system architecture and key decisions (ADRs)
- **API documentation**: Generated from code or specifications
- **Deployment**: Environment setup and deployment procedures
- **Operations**: Monitoring, alerting, and troubleshooting guides
