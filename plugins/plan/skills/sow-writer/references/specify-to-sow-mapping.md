# Specify-to-SoW Section Mapping

This document defines how each Specify section maps to SoW sections and the transformation rules for converting technical content into business-facing language.

## Mapping Table

| Specify Section | SoW Section | Transformation |
| ---------- | ----------- | -------------- |
| Component (Name, Purpose) | Cover Page, Background, Scope | Title Case name; purpose becomes background narrative |
| Specify Type header | Background | `code-analysis` = migration framing; `requirements` = greenfield framing |
| User Stories | Objectives, Deliverables | Acceptance criteria summarised as objectives; stories grouped into deliverables |
| Business Logic Blocks | Deliverables | Each major block becomes a deliverable item; algorithm details omitted |
| API Contracts | Deliverables, Scope | Endpoints grouped as "API implementation" deliverable; integration points in scope |
| External Service Dependencies | Scope, Dependencies | Integration points in scope narrative; access requirements as dependencies |
| Constants & Configuration | Dependencies | Environment-specific config as client-provided dependencies |
| Domain Model | Deliverables | Complex domain models noted as "domain types and validation" deliverable |
| Publication & Timing Patterns | Deliverables | Event publishing as "event integration" deliverable |
| Implementation Requirements | Scope, Assumptions | Technology stack informs scope; constraints become assumptions |
| Notes | Assumptions | Analysis notes become SoW assumptions |
| `[unknown]` tokens | Assumptions | Each unknown becomes an assumption requiring client confirmation |

## Language Transformation Rules

### Principle: Technical to Business

The SoW audience is a client stakeholder, not a developer. Every technical concept must be translated.

| Technical (Specify) | Business (SoW) |
| -------------- | --------------- |
| Handler<P> trait implementation | Component implementation |
| HttpRequest provider integration | External API integration |
| Publish provider with topic routing | Event notification delivery |
| StateStore key-value operations | Data caching and retrieval |
| Config provider lookups | Environment configuration |
| serde deserialization | Data format handling |
| WASM32 compilation target | Cloud-native deployment |
| Omnia SDK provider pattern | Platform integration |
| MockProvider test harness | Automated test suite |
| `cargo check` / `cargo test` | Build verification and testing |
| Domain model with newtypes | Business data validation |

### Scope Framing by Specify Type

**Migration (`code-analysis` artifacts)**:

> This Statement of Work covers the migration of the $COMPONENT from $SOURCE_LANGUAGE to Rust WASM, targeting the Omnia platform. The migration will preserve existing business logic and external integrations while delivering improved performance, type safety, and platform alignment.

**Greenfield (`requirements` artifacts)**:

> This Statement of Work covers the delivery of the $COMPONENT, a new capability that $PURPOSE_SUMMARY. The implementation will be built on the Omnia platform using Rust WASM.

### Deliverable Description Pattern

Each deliverable should follow this pattern:

```
**$TITLE**

We will $ACTION $WHAT, enabling $BUSINESS_OUTCOME. This involves:

- $SPECIFIC_ITEM_1
- $SPECIFIC_ITEM_2
- $SPECIFIC_ITEM_3
```

Example:

```
**Notification Processing**

We will implement the notification processing component, enabling real-time
delivery of disruption alerts to mobile users. This involves:

- Processing inbound disruption events from the messaging platform
- Enriching notifications with route and stop information
- Publishing formatted notifications for mobile consumption
```

### Exclusion Patterns

Standard exclusions (include unless the artifacts explicitly contradicts):

```markdown
A. **User Acceptance Testing**
   User Acceptance Testing (UAT) confirms that the system will help users
   achieve their goals. We exclude this testing from scope as we expect the
   client to undertake this activity, though we will provide necessary support.

B. **Design Review Board (DRB)**
   DRB activities and approvals are excluded from this scope of work.

C. **Project Management**
   Day-to-day project management activities are excluded unless separately
   agreed as a deliverable.

D. **Dependency and Environment Management**
   Resolution of issues relating to client environments, third-party
   configuration, authentication services, networking, firewall rules, or
   vendor infrastructure is excluded unless separately agreed.

E. **Additional Minor Features**
   Features or enhancements not described in the Deliverables section above
   are excluded from this Statement of Work, which we can address separately.
```

### Dependency Patterns

From Specify External Service Dependencies:

```markdown
A. **$SERVICE_NAME API Availability**
   $CLIENT_NAME must provide stable access to the $SERVICE_NAME API, including
   documentation, credentials, and a test environment.
```

From Specify Constants & Configuration:

```markdown
B. **Access to Environments**
   $CLIENT_NAME must provide access to development, staging, and production
   environments with appropriate credentials and network access.
```

Standard dependencies:

```markdown
C. **$CLIENT_NAME Project Manager**
   $CLIENT_NAME will nominate a project manager to coordinate activities,
   provide timely feedback on deliverables, and facilitate access to
   stakeholders and resources.

D. **$CLIENT_NAME Product Owner**
   $CLIENT_NAME will nominate a product owner to clarify requirements,
   prioritise deliverables, and accept completed work.
```

### Assumption Patterns

From Specify `[unknown]` tokens:

```markdown
A. **$ASSUMPTION_TITLE**
   $DESCRIPTION_DERIVED_FROM_UNKNOWN_TOKEN. If this assumption proves
   incorrect, we will raise a change request.
```

Standard assumptions:

```markdown
B. **Deployment to Production**
   Deployment to production will follow the existing CI/CD pipeline and
   release process. No new deployment infrastructure is required.

C. **No Material Architectural Changes**
   The surrounding system architecture will not undergo material changes
   during the engagement period.

D. **Security and Compliance**
   Security and compliance requirements are met by the existing platform
   capabilities. No additional security certifications are required.
```
