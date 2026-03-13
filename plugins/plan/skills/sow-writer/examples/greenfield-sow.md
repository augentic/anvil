# Example: Greenfield SoW from Requirements Artifacts

## Scenario

A new GTFS generation component is being built from JIRA requirements. The `epic-analyzer` skill has produced Specify artifacts (specs + design.md), and we need to generate a SoW for the client.

## Input

**Specify artifacts**: `./.specify/changes/gtfs_generator/`
**Client**: Auckland Transport
**Origin**: requirements (JIRA epic)

**Key artifact content** (summarised):

- **Component**: `gtfs_generator` — Generates GTFS feed data from contract and timetable sources
- **User Stories**: 5 stories covering feed generation, validation, publishing, scheduling, and error reporting
- **Business Logic Blocks**: 4 blocks (extract contracts, transform to GTFS, validate feed, publish to CDN)
- **API Contracts**: 3 endpoints (POST /feeds/generate, GET /feeds/{id}, GET /feeds/{id}/validation)
- **External Service Dependencies**: Contract API (HTTP), Timetable API (HTTP), CDN (HTTP), Notification Service (Publish)
- **Constants & Configuration**: 6 environment variables
- **Implementation Requirements**: Config, HttpRequest, Publish, StateStore providers
- **`[unknown]` tokens**: 3 (feed generation schedule, CDN cache invalidation, historical feed retention)

## Command

```prompt
/sow-writer ./.specify/changes/gtfs_generator ./SOW-gtfs-generator.md "Auckland Transport"
```

## Output

The generated SoW at `./SOW-gtfs-generator.md`:

```markdown
# Statement of Work

**Client**: Auckland Transport
**Project**: GTFS Generator

| Version | Date        | Author | Changes       |
| ------- | ----------- | ------ | ------------- |
| 0.1     | 27 Feb 2026 |        | Initial draft |

## Introduction

### Background

Auckland Transport requires a new capability to generate GTFS (General
Transit Feed Specification) data from contract and timetable sources. This
feed data enables journey planning applications and third-party transit
information services to provide accurate, up-to-date public transport
information to commuters.

### Objectives

- Deliver an automated GTFS feed generation capability from contract and
  timetable data sources
- Ensure generated feeds pass GTFS specification validation
- Enable scheduled and on-demand feed generation
- Publish validated feeds to the CDN for consumption by downstream systems
- Provide feed generation status and validation reporting

### Reference Agreement

This Statement of Work is issued under the Master Services Agreement between
Propellerhead Limited and Auckland Transport dated [DATE].

## Services

### Scope

This Statement of Work covers the delivery of the GTFS Generator, a new
component that generates GTFS feed data from Auckland Transport's contract
and timetable systems. The implementation will integrate with the Contract
API, Timetable API, CDN, and Notification Service, built on the Omnia
platform using Rust WASM.

### Deliverables

| DESCRIPTION | COST |
| ----------- | ---- |
| **GTFS Feed Generation** | $X,XXX (N d) |
| We will implement the core feed generation capability, extracting data | |
| from contract and timetable sources and transforming it into valid GTFS | |
| format. This involves: | |
| - Extracting contract data from the Contract API | |
| - Extracting timetable data from the Timetable API | |
| - Transforming source data into GTFS-compliant feed format | |
| - Supporting both scheduled and on-demand generation | |
| | |
| **Feed Validation** | $X,XXX (N d) |
| We will implement GTFS specification validation, ensuring all generated | |
| feeds meet the required standard before publication. This involves: | |
| - Validating generated feeds against the GTFS specification | |
| - Producing detailed validation reports | |
| - Exposing validation status via the API | |
| | |
| **Feed Publishing** | $X,XXX (N d) |
| We will implement the feed publishing pipeline, delivering validated | |
| feeds to the CDN and notifying downstream systems. This involves: | |
| - Publishing validated feeds to the CDN | |
| - Sending generation status notifications | |
| - Providing feed retrieval via the API | |
| | |
| **Automated Test Suite** | $X,XXX (N d) |
| We will deliver a comprehensive test suite covering all business logic, | |
| API contracts, and integration points. This involves: | |
| - Unit tests for data extraction, transformation, and validation logic | |
| - Integration tests for external API interactions | |
| - Contract tests for API endpoints | |

### Exclusions from Scope

The following items are explicitly excluded from this Statement of Work:

A. **Systems Not Listed in Scope**
   Integration with systems not explicitly named in the Scope section above
   is excluded. Any additional integrations will be addressed separately.

B. **User Acceptance Testing**
   User Acceptance Testing (UAT) confirms that the system will help users
   achieve their goals. We exclude this testing from scope as we expect
   Auckland Transport to undertake this activity, though we will provide
   necessary support.

C. **Design Review Board (DRB)**
   DRB activities and approvals are excluded from this scope of work.

D. **Project Management**
   Day-to-day project management activities are excluded unless separately
   agreed as a deliverable.

E. **Dependency and Environment Management**
   Resolution of issues relating to Auckland Transport environments,
   third-party configuration, authentication services, networking, firewall
   rules, or vendor infrastructure is excluded unless separately agreed.

F. **Broader Data Model Changes**
   Changes to the underlying contract or timetable data models are excluded.
   The GTFS Generator will consume data in its current format.

G. **Analytics and Reporting**
   Feed usage analytics, download statistics, and reporting dashboards are
   excluded from this engagement.

H. **Additional Features**
   Features or enhancements not described in the Deliverables section above
   are excluded from this Statement of Work, which we can address separately.

## Fees

The fees (Fees) payable by the Customer for the Services will be charged on
a **Fixed Price** basis.

| Fee | $TOTAL + GST |
| --- | ------------ |

Notes:
- All amounts specified above exclude GST.
- Unless otherwise agreed, Auckland Transport will not withhold any amounts
  as project retention amounts.

### Payment Schedule

Fees will be invoiced monthly for services rendered within that month up to
a maximum amount specified as the Fee (see above). Payments are due on the
20th of the following month after the invoice date.

| Month | Payment |
| ----- | ------- |
| $MONTH_1 | $AMOUNT_1 |
| $MONTH_2 | $AMOUNT_2 |

## Other Issues

### Dependencies

The following dependencies have been identified as important to the success
of this engagement:

A. **Contract API Availability**
   Auckland Transport must provide stable access to the Contract API,
   including API documentation, credentials, and a test environment with
   representative contract data.

B. **Timetable API Availability**
   Auckland Transport must provide stable access to the Timetable API,
   including API documentation, credentials, and a test environment with
   representative timetable data.

C. **CDN Access**
   Auckland Transport must provide CDN publish credentials and configuration
   for feed distribution, including a test CDN endpoint.

D. **Access to Environments**
   Auckland Transport must provide access to development, staging, and
   production environments with appropriate credentials and network access.

E. **Auckland Transport Project Manager**
   Auckland Transport will nominate a project manager to coordinate
   activities, provide timely feedback on deliverables, and facilitate access
   to stakeholders and resources.

F. **Auckland Transport Product Owner**
   Auckland Transport will nominate a product owner to clarify requirements,
   prioritise deliverables, and accept completed work.

### Assumptions

A. **Feed Generation Schedule**
   Feed generation will be triggered on demand via the API. If Auckland
   Transport requires scheduled generation (e.g., daily at a fixed time),
   we will raise a change request to add scheduling configuration.

B. **CDN Cache Invalidation**
   The CDN will handle cache invalidation automatically when new feeds are
   published. If manual cache invalidation is required, we will raise a
   change request.

C. **Historical Feed Retention**
   Historical feeds will not be retained beyond the current active feed. If
   Auckland Transport requires a feed history or versioning mechanism, we
   will raise a change request.

D. **Deployment to Production**
   Deployment to production will follow the existing CI/CD pipeline and
   release process. No new deployment infrastructure is required.

E. **No Material Architectural Changes**
   The surrounding system architecture (Contract API, Timetable API, CDN)
   will not undergo material changes during the engagement period.

F. **Security and Compliance**
   Security and compliance requirements are met by the existing Omnia
   platform capabilities. No additional security certifications are required.

## Acceptance

### SIGNED by for and on behalf of Propellerhead Limited by:

| | |
| --- | --- |
| Name | |
| Title | |
| Date | |
| Signature | |

### SIGNED by for and on behalf of Auckland Transport by:

| | |
| --- | --- |
| Name | |
| Title | |
| Date | |
| Signature | |

## Appendix A

### Standard Services and Deliverables

[Standard boilerplate from sow-template.md]
```

## Key Observations

1. **Greenfield framing**: Background focuses on new capability delivery, not migration.
2. **User Stories → Deliverables**: The 5 user stories were grouped into 3 logical deliverables plus a test suite.
3. **Multiple external APIs → Multiple dependencies**: Each API from External Service Dependencies became a separate dependency.
4. **3 `[unknown]` tokens → 3 assumptions**: Schedule, CDN cache, and retention became assumptions A-C.
5. **Domain-specific exclusions**: "Broader Data Model Changes" and "Analytics and Reporting" are specific to this project, derived from what the artifacts do NOT cover.
