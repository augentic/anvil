# Example: Migration SoW from Code-Analysis Artifacts

## Scenario

A TypeScript notification processing service is being migrated to Rust WASM. The `code-analyzer` skill has produced Specify artifacts (specs + design.md), and we need to generate a SoW for the client.

## Input

**Specify artifacts**: `./.specify/changes/notification_hub/`
**Client**: Auckland Transport
**Origin**: code-analysis (TypeScript source)

**Key artifact content** (summarised):

- **Component**: `notification_hub` — Processes disruption notifications and publishes to mobile channels
- **Business Logic Blocks**: 3 blocks (process notification, enrich with route data, format for mobile)
- **API Contracts**: 2 endpoints (POST /notifications, GET /notifications/{id}/status)
- **External Service Dependencies**: Route API (HTTP), Message Broker (Publish)
- **Constants & Configuration**: 4 environment variables (ROUTE_API_URL, BROKER_TOPIC, etc.)
- **Publication & Timing Patterns**: 1 topic (notification.mobile.v1)
- **`[unknown]` tokens**: 2 (retry policy for Route API, notification TTL)

## Command

```prompt
/sow-writer ./.specify/changes/notification_hub ./SOW-notification-hub.md "Auckland Transport"
```

## Output

The generated SoW at `./SOW-notification-hub.md`:

```markdown
# Statement of Work

**Client**: Auckland Transport
**Project**: Notification Hub

| Version | Date        | Author | Changes       |
| ------- | ----------- | ------ | ------------- |
| 0.1     | 27 Feb 2026 |        | Initial draft |

## Introduction

### Background

Auckland Transport operates a notification processing system that delivers
disruption alerts to mobile users. As part of the ongoing platform
modernisation programme, this service will be migrated from TypeScript to
Rust WASM on the Omnia platform, delivering improved performance, type
safety, and alignment with the target architecture.

### Objectives

- Migrate the notification processing capability to Rust WASM with full
  functional parity
- Preserve all existing integrations with the Route API and message broker
- Deliver a comprehensive automated test suite covering all business logic
- Ensure seamless deployment via the existing CI/CD pipeline
- Improve system reliability through stronger type safety and error handling

### Reference Agreement

This Statement of Work is issued under the Master Services Agreement between
Propellerhead Limited and Auckland Transport dated [DATE].

## Services

### Scope

This Statement of Work covers the migration of the Notification Hub from
TypeScript to Rust WASM, targeting the Omnia platform. The implementation
will preserve existing business logic for notification processing, route
data enrichment, and mobile delivery, while integrating with the Route API
and message broker.

### Deliverables

| DESCRIPTION | COST |
| ----------- | ---- |
| **Notification Processing** | $X,XXX (N d) |
| We will implement the core notification processing component, handling | |
| inbound disruption events and preparing them for mobile delivery. This | |
| involves: | |
| - Processing inbound notification payloads | |
| - Enriching notifications with route and stop information via the Route API | |
| - Formatting notifications for mobile consumption | |
| | |
| **Mobile Notification Delivery** | $X,XXX (N d) |
| We will implement the event publishing integration, delivering formatted | |
| notifications to the mobile channel via the message broker. This involves: | |
| - Publishing to the notification.mobile.v1 topic | |
| - Notification status tracking via the status API endpoint | |
| | |
| **Automated Test Suite** | $X,XXX (N d) |
| We will deliver a comprehensive test suite covering all business logic, | |
| API contracts, and integration points. This involves: | |
| - Unit tests for notification processing and enrichment logic | |
| - Integration tests for Route API and message broker interactions | |
| - Contract tests for API endpoints | |

### Exclusions from Scope

The following items are explicitly excluded from this Statement of Work:

A. **User Acceptance Testing**
   User Acceptance Testing (UAT) confirms that the system will help users
   achieve their goals. We exclude this testing from scope as we expect
   Auckland Transport to undertake this activity, though we will provide
   necessary support.

B. **Design Review Board (DRB)**
   DRB activities and approvals are excluded from this scope of work.

C. **Project Management**
   Day-to-day project management activities are excluded unless separately
   agreed as a deliverable.

D. **Dependency and Environment Management**
   Resolution of issues relating to Auckland Transport environments,
   third-party configuration, authentication services, networking, firewall
   rules, or vendor infrastructure is excluded unless separately agreed.

E. **Performance and Load Testing**
   Performance benchmarking and load testing of the migrated component are
   excluded from this engagement.

F. **Data Migration**
   Migration of any existing notification data or history is excluded from
   this scope.

G. **Additional Features**
   Features or enhancements beyond the current TypeScript implementation are
   excluded from this Statement of Work, which we can address separately.

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

## Other Issues

### Dependencies

The following dependencies have been identified as important to the success
of this engagement:

A. **Route API Availability**
   Auckland Transport must provide stable access to the Route API, including
   API documentation, credentials, and a test environment with representative
   data.

B. **Message Broker Access**
   Auckland Transport must provide access to the message broker environment
   for the notification.mobile.v1 topic, including publish permissions and
   a test environment.

C. **Access to Environments**
   Auckland Transport must provide access to development, staging, and
   production environments with appropriate credentials and network access.

D. **Auckland Transport Project Manager**
   Auckland Transport will nominate a project manager to coordinate
   activities, provide timely feedback on deliverables, and facilitate access
   to stakeholders and resources.

E. **Auckland Transport Product Owner**
   Auckland Transport will nominate a product owner to clarify requirements,
   prioritise deliverables, and accept completed work.

### Assumptions

A. **Route API Retry Policy**
   The Route API integration will use a standard retry policy with
   exponential backoff (3 retries, 1s/2s/4s delays). If Auckland Transport
   requires a different retry strategy, we will raise a change request.

B. **Notification TTL**
   Notifications will not have a time-to-live (TTL) constraint. If a TTL is
   required, we will raise a change request to add expiry handling.

C. **Deployment to Production**
   Deployment to production will follow the existing CI/CD pipeline and
   release process. No new deployment infrastructure is required.

D. **No Material Architectural Changes**
   The surrounding system architecture will not undergo material changes
   during the engagement period.

E. **Security and Compliance**
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

1. **Cost placeholders**: All monetary values are `$X,XXX` — never generated by the skill.
2. **Business language**: "Handler<P> trait with HttpRequest provider" became "integration with the Route API".
3. **`[unknown]` → Assumptions**: The 2 unknown tokens (retry policy, notification TTL) became assumptions A and B.
4. **External deps → Dependencies**: Route API and Message Broker became client-provided dependencies.
5. **Standard exclusions**: UAT, DRB, project management, and environment management are always excluded.
