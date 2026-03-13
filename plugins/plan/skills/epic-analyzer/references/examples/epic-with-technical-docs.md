# Example 3: Epic with Technical Documentation

## Scenario

Analyze a JIRA epic that includes structured technical documentation in markdown sections within the Description field, plus attached technical specification files. This demonstrates how the epic-analyzer extracts and preserves technical context for downstream code generation.

## Input

**Arguments**:

```text
$ARGUMENTS[0]: ATR-7300
$ARGUMENTS[1]: ./.specify/changes/payment-gateway
```

**JIRA Data Structure** (Epic ATR-7300):

````json
{
  "key": "ATR-7300",
  "fields": {
    "summary": "Payment Gateway Integration",
    "description": "Implement payment processing with third-party gateway supporting credit cards and digital wallets.\n\n## Technical Specifications\n\nThis feature integrates with the Stripe payment gateway API for processing transactions.\n\n### Integration Requirements\n\n- Stripe API Version: 2023-10-16\n- Required Scopes: `payment_intents`, `customers`, `refunds`\n- Webhook Support: Required for async payment confirmations\n- PCI Compliance: Level 1 (handled by Stripe)\n\n## API Contract\n\n### Process Payment Endpoint\n\n**Endpoint**: `POST /api/payments/process`\n\n**Request**:\n```json\n{\n  \"order_id\": \"string\",\n  \"amount\": \"number\",\n  \"currency\": \"string (USD, EUR, GBP)\",\n  \"payment_method\": {\n    \"type\": \"card | wallet\",\n    \"token\": \"string\"\n  },\n  \"customer_id\": \"string\"\n}\n```\n\n**Response** (Success - 201):\n```json\n{\n  \"transaction_id\": \"string\",\n  \"status\": \"pending | completed | failed\",\n  \"amount\": \"number\",\n  \"currency\": \"string\",\n  \"created_at\": \"ISO 8601 timestamp\"\n}\n```\n\n**Error Responses**:\n- 400: Invalid payment method or amount\n- 402: Payment declined by gateway\n- 500: Gateway unavailable\n\n## Domain Model\n\n### Payment Transaction Entity\n\n```json\n{\n  \"id\": \"TransactionId (UUID)\",\n  \"order_id\": \"OrderId (UUID)\",\n  \"customer_id\": \"CustomerId (UUID)\",\n  \"amount\": \"Decimal\",\n  \"currency\": \"CurrencyCode (enum: USD, EUR, GBP)\",\n  \"status\": \"TransactionStatus (enum: Pending, Completed, Failed, Refunded)\",\n  \"payment_method_type\": \"PaymentMethodType (enum: Card, Wallet)\",\n  \"gateway_transaction_id\": \"string\",\n  \"created_at\": \"DateTime<Utc>\",\n  \"completed_at\": \"Option<DateTime<Utc>>\"\n}\n```\n\n### Business Rules\n\n- Minimum transaction amount: $0.50 USD equivalent\n- Maximum transaction amount: $10,000 USD equivalent\n- Transactions expire after 30 minutes if not completed\n- Refunds allowed within 90 days of transaction\n\n## External Services\n\n### Stripe Payment Gateway\n\n- Base URL: `https://api.stripe.com/v1`\n- Authentication: Bearer token via `STRIPE_SECRET_KEY` environment variable\n- Endpoints used:\n  - `POST /payment_intents` - Create payment intent\n  - `GET /payment_intents/{id}` - Retrieve payment status\n  - `POST /refunds` - Process refund\n\n### Order Service\n\n- Internal service for order validation\n- Endpoint: `GET /internal/orders/{order_id}`\n- Authentication: Service-to-service JWT\n\n## Configuration\n\n**Required Environment Variables**:\n\n```bash\nSTRIPE_SECRET_KEY=sk_live_xxxxx\nSTRIPE_WEBHOOK_SECRET=whsec_xxxxx\nORDER_SERVICE_URL=https://orders.internal/api\nORDER_SERVICE_JWT_SECRET=xxxxx\nPAYMENT_TIMEOUT_SECONDS=1800\nMAX_RETRY_ATTEMPTS=3\n```\n\n**Required Providers** (Omnia SDK):\n\n- HttpRequest - For Stripe API calls\n- StateStore - For caching payment status\n- Publish - For payment event notifications\n- Config - For reading environment variables\n- Identity - For service authentication",
    "status": { "name": "In Progress" },
    "issuetype": { "name": "Epic" },
    "labels": ["backend", "payments", "integration"],
    "components": [{ "name": "Payment Service" }],
    "created": "2026-01-20T10:00:00.000Z",
    "updated": "2026-02-03T16:45:00.000Z",
    "attachment": [
      {
        "id": "10001",
        "filename": "stripe-api-spec.json",
        "size": 45678,
        "mimeType": "application/json",
        "created": "2026-01-21T09:00:00.000Z"
      },
      {
        "filename": "payment-schema.sql",
        "size": 2340,
        "mimeType": "text/plain",
        "created": "2026-01-21T09:15:00.000Z"
      }
    ]
  }
}
````

**User Story ATR-7301** (Linked to Epic):

````json
{
  "key": "ATR-7301",
  "fields": {
    "summary": "Process Credit Card Payment",
    "description": "As a customer, I want to pay for my order using a credit card so that I can complete my purchase.\n\n### Acceptance Criteria\n\n1. Customer can submit payment with order ID and credit card token\n2. System validates order exists and amount matches\n3. System creates payment intent with Stripe\n4. System handles 3D Secure authentication if required\n5. Transaction status is updated based on gateway response\n6. Customer receives confirmation on successful payment\n7. Error handling for declined cards (402)\n8. Error handling for invalid amounts (400)\n\n## Technical Specifications\n\n### Stripe Integration Details\n\n**Payment Intent Flow**:\n\n1. Create PaymentIntent with amount and currency\n2. Attach payment method to intent\n3. Confirm payment intent\n4. Handle webhook for status updates\n\n**3D Secure Handling**:\n\n- If `requires_action` status returned, redirect to authentication URL\n- Poll for completion or receive webhook\n- Update transaction status accordingly\n\n### Error Mapping\n\n| Stripe Error Code | HTTP Status | User Message |\n| ---------------- | ----------- | ------------- |\n| card_declined | 402 | \"Your card was declined\" |\n| insufficient_funds | 402 | \"Insufficient funds\" |\n| invalid_amount | 400 | \"Invalid payment amount\" |\n| api_error | 500 | \"Payment service unavailable\" |\n\n### Other Information\n\nBDD Scenarios:\n\n```gherkin\nScenario: Successfully process credit card payment\n  Given order \"ORD-001\" exists with amount 99.99 USD\n  And customer \"CUST-001\" has valid card token \"tok_visa\"\n  And Stripe API is available\n  When customer submits payment for order \"ORD-001\"\n  Then payment intent is created with Stripe\n  And payment is confirmed successfully\n  And transaction status is \"Completed\"\n  And order status is updated to \"Paid\"\n  And confirmation event is published\n\nScenario: Handle declined card\n  Given order \"ORD-002\" exists with amount 50.00 USD\n  And customer submits payment with declined card token\n  When payment is processed\n  Then Stripe returns card_declined error\n  And response status is 402\n  And error message is \"Your card was declined\"\n  And transaction status is \"Failed\"\n\nScenario: Handle 3D Secure authentication\n  Given order \"ORD-003\" exists with amount 150.00 USD\n  And card requires 3D Secure verification\n  When customer submits payment\n  Then payment intent returns requires_action status\n  And authentication URL is returned to customer\n  And transaction status is \"Pending\"\n  When customer completes authentication\n  And webhook confirms payment\n  Then transaction status is updated to \"Completed\"\n```",
    "status": { "name": "In Progress" },
    "issuetype": { "name": "Story" },
    "priority": { "name": "High" },
    "customfield_10016": 8
  }
}
````

## Execution

```text
epic-analyzer ATR-7300 ./.specify/changes/payment-gateway
```

## MCP Tool Calls

```
1. CallMcpTool:
     server: "atlassian"
     toolName: "get_issue"
     arguments: {"issueKey": "ATR-7300"}

   Response: Epic metadata with technical documentation sections (shown above)

2. CallMcpTool:
     server: "atlassian"
     toolName: "search_issues"
     arguments: {"jql": "\"Epic Link\" = ATR-7300"}

   Response: List of user stories [ATR-7301]

3. CallMcpTool:
     server: "atlassian"
     toolName: "get_issue"
     arguments: {"issueKey": "ATR-7301"}

   Response: User story details (shown above)

4. CallMcpTool:
     server: "atlassian"
     toolName: "jira_get_attachments"
     arguments: {"issueKey": "ATR-7300"}

   Response: List of attachments [stripe-api-spec.json, payment-schema.sql]

5. CallMcpTool:
     server: "atlassian"
     toolName: "jira_download_attachment"
     arguments: {"attachmentId": "10001"}

   Response: Content of stripe-api-spec.json

6. CallMcpTool:
     server: "atlassian"
     toolName: "jira_download_attachment"
     arguments: {"filename": "payment-schema.sql", "issueKey": "ATR-7300"}

   Response: Content of payment-schema.sql
```

**Note**: If MCP attachment tools are not available, steps 4-6 are skipped gracefully and logged in artifact notes.

## Expected Output

**Generated Specify Artifacts** (`.specify/changes/payment-gateway/` - Excerpt):

````markdown
# Specify Artifacts

Generated from: JIRA Epic ATR-7300
Generated at: 2026-02-05T14:30:00Z

## Epic Metadata

- **Key**: ATR-7300
- **Summary**: Payment Gateway Integration
- **Description**: Implement payment processing with third-party gateway supporting credit cards and digital wallets.
- **Status**: In Progress
- **Labels**: backend, payments, integration
- **Components**: Payment Service
- **Created**: 2026-01-20T10:00:00.000Z
- **Updated**: 2026-02-03T16:45:00.000Z

## User Stories

### User Story: ATR-7301

- **Summary**: Process Credit Card Payment
- **Description**: As a customer, I want to pay for my order using a credit card so that I can complete my purchase.
- **Status**: In Progress
- **Story Points**: 8
- **Priority**: High

#### Acceptance Criteria

1. Customer can submit payment with order ID and credit card token
2. System validates order exists and amount matches
3. System creates payment intent with Stripe
4. System handles 3D Secure authentication if required
5. Transaction status is updated based on gateway response
6. Customer receives confirmation on successful payment
7. Error handling for declined cards (402)
8. Error handling for invalid amounts (400)

#### BDD Scenarios

**Scenario: Successfully process credit card payment**

- Given: order "ORD-001" exists with amount 99.99 USD
- And: customer "CUST-001" has valid card token "tok_visa"
- And: Stripe API is available
- When: customer submits payment for order "ORD-001"
- Then: payment intent is created with Stripe
- And: payment is confirmed successfully
- And: transaction status is "Completed"
- And: order status is updated to "Paid"
- And: confirmation event is published

**Scenario: Handle declined card**

- Given: order "ORD-002" exists with amount 50.00 USD
- And: customer submits payment with declined card token
- When: payment is processed
- Then: Stripe returns card_declined error
- And: response status is 402
- And: error message is "Your card was declined"
- And: transaction status is "Failed"

**Scenario: Handle 3D Secure authentication**

- Given: order "ORD-003" exists with amount 150.00 USD
- And: card requires 3D Secure verification
- When: customer submits payment
- Then: payment intent returns requires_action status
- And: authentication URL is returned to customer
- And: transaction status is "Pending"
- When: customer completes authentication
- And: webhook confirms payment
- Then: transaction status is updated to "Completed"

## Technical Documentation

### From Epic ATR-7300

#### Technical Specifications

This feature integrates with the Stripe payment gateway API for processing transactions.

**Integration Requirements**

- Stripe API Version: 2023-10-16
- Required Scopes: `payment_intents`, `customers`, `refunds`
- Webhook Support: Required for async payment confirmations
- PCI Compliance: Level 1 (handled by Stripe)

#### API Contracts (Specified)

**Process Payment Endpoint**

**Endpoint**: `POST /api/payments/process`

**Request**:

```json
{
  "order_id": "string",
  "amount": "number",
  "currency": "string (USD, EUR, GBP)",
  "payment_method": {
    "type": "card | wallet",
    "token": "string"
  },
  "customer_id": "string"
}
```
````

**Response** (Success - 201):

```json
{
  "transaction_id": "string",
  "status": "pending | completed | failed",
  "amount": "number",
  "currency": "string",
  "created_at": "ISO 8601 timestamp"
}
```

**Error Responses**:

- 400: Invalid payment method or amount
- 402: Payment declined by gateway
- 500: Gateway unavailable

#### Domain Model (Specified)

**Payment Transaction Entity**

```json
{
  "id": "TransactionId (UUID)",
  "order_id": "OrderId (UUID)",
  "customer_id": "CustomerId (UUID)",
  "amount": "Decimal",
  "currency": "CurrencyCode (enum: USD, EUR, GBP)",
  "status": "TransactionStatus (enum: Pending, Completed, Failed, Refunded)",
  "payment_method_type": "PaymentMethodType (enum: Card, Wallet)",
  "gateway_transaction_id": "string",
  "created_at": "DateTime<Utc>",
  "completed_at": "Option<DateTime<Utc>>"
}
```

**Business Rules**

- Minimum transaction amount: $0.50 USD equivalent
- Maximum transaction amount: $10,000 USD equivalent
- Transactions expire after 30 minutes if not completed
- Refunds allowed within 90 days of transaction

#### External Services

**Stripe Payment Gateway**

- Base URL: `https://api.stripe.com/v1`
- Authentication: Bearer token via `STRIPE_SECRET_KEY` environment variable
- Endpoints used:
  - `POST /payment_intents` - Create payment intent
  - `GET /payment_intents/{id}` - Retrieve payment status
  - `POST /refunds` - Process refund

**Order Service**

- Internal service for order validation
- Endpoint: `GET /internal/orders/{order_id}`
- Authentication: Service-to-service JWT

#### Database Schema

No database schema specified in epic description.

#### Configuration Requirements

**Required Environment Variables**:

```bash
STRIPE_SECRET_KEY=sk_live_xxxxx
STRIPE_WEBHOOK_SECRET=whsec_xxxxx
ORDER_SERVICE_URL=https://orders.internal/api
ORDER_SERVICE_JWT_SECRET=xxxxx
PAYMENT_TIMEOUT_SECONDS=1800
MAX_RETRY_ATTEMPTS=3
```

**Required Capabilities**:

- Outbound HTTP - For Stripe API calls
- Key-value state - For caching payment status
- Message publishing - For payment event notifications
- Configuration - For reading environment variables
- Authentication/Identity - For service authentication

### From User Story ATR-7301

#### Technical Specifications

**Stripe Integration Details**

**Payment Intent Flow**:

1. Create PaymentIntent with amount and currency
2. Attach payment method to intent
3. Confirm payment intent
4. Handle webhook for status updates

**3D Secure Handling**:

- If `requires_action` status returned, redirect to authentication URL
- Poll for completion or receive webhook
- Update transaction status accordingly

**Error Mapping**

| Stripe Error Code  | HTTP Status | User Message                  |
| ------------------ | ----------- | ----------------------------- |
| card_declined      | 402         | "Your card was declined"      |
| insufficient_funds | 402         | "Insufficient funds"          |
| invalid_amount     | 400         | "Invalid payment amount"      |
| api_error          | 500         | "Payment service unavailable" |

#### API Contracts (Specified)

No additional API contracts specified in user story.

#### Domain Model (Specified)

No additional domain model specified in user story.

#### External Services

No additional external services specified in user story.

#### Database Schema

No database schema specified in user story.

#### Configuration Requirements

No additional configuration specified in user story.

### From Attachments

#### Attachment: stripe-api-spec.json (Source: ATR-7300)

**File Type**: JSON
**Size**: 44.6 KB

**Content**:

```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "Stripe Payment Intent API",
    "version": "2023-10-16"
  },
  "paths": {
    "/v1/payment_intents": {
      "post": {
        "summary": "Create a PaymentIntent",
        "requestBody": {
          "content": {
            "application/x-www-form-urlencoded": {
              "schema": {
                "type": "object",
                "properties": {
                  "amount": {
                    "type": "integer",
                    "description": "Amount in cents"
                  },
                  "currency": {
                    "type": "string",
                    "enum": ["usd", "eur", "gbp"]
                  },
                  "payment_method": {
                    "type": "string"
                  },
                  "confirm": {
                    "type": "boolean"
                  }
                },
                "required": ["amount", "currency"]
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "PaymentIntent created",
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "id": { "type": "string" },
                    "status": {
                      "type": "string",
                      "enum": [
                        "requires_payment_method",
                        "requires_confirmation",
                        "requires_action",
                        "processing",
                        "succeeded",
                        "canceled"
                      ]
                    },
                    "amount": { "type": "integer" },
                    "currency": { "type": "string" }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

#### Attachment: payment-schema.sql (Source: ATR-7300)

**File Type**: SQL
**Size**: 2.3 KB

**Content**:

```sql
-- Payment Transactions Table
CREATE TABLE payment_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id),
    customer_id UUID NOT NULL REFERENCES customers(id),
    amount DECIMAL(10, 2) NOT NULL CHECK (amount >= 0.50),
    currency VARCHAR(3) NOT NULL CHECK (currency IN ('USD', 'EUR', 'GBP')),
    status VARCHAR(20) NOT NULL CHECK (status IN ('Pending', 'Completed', 'Failed', 'Refunded')),
    payment_method_type VARCHAR(20) NOT NULL CHECK (payment_method_type IN ('Card', 'Wallet')),
    gateway_transaction_id VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (NOW() + INTERVAL '30 minutes'),

    INDEX idx_order_id (order_id),
    INDEX idx_customer_id (customer_id),
    INDEX idx_gateway_transaction_id (gateway_transaction_id),
    INDEX idx_status (status),
    INDEX idx_created_at (created_at)
);

-- Payment Status History Table (for audit trail)
CREATE TABLE payment_status_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES payment_transactions(id),
    previous_status VARCHAR(20),
    new_status VARCHAR(20) NOT NULL,
    reason TEXT,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    INDEX idx_transaction_id (transaction_id)
);
```

### Technical Documentation Notes

- Technical documentation sources: Description sections and Attachments
- Attachment support: Available via MCP
- Skipped attachments: None

## Domain Model

### Entities

#### PaymentTransaction

- **Attributes**:
  - `id` — TransactionId (UUID) — Unique identifier for the transaction
  - `order_id` — OrderId (UUID) — Associated order
  - `customer_id` — CustomerId (UUID) — Customer who made payment
  - `amount` — Decimal — Payment amount (minimum 0.50)
  - `currency` — CurrencyCode (enum: USD, EUR, GBP) — Currency code
  - `status` — TransactionStatus (enum: Pending, Completed, Failed, Refunded) — Current status
  - `payment_method_type` — PaymentMethodType (enum: Card, Wallet) — Type of payment method
  - `gateway_transaction_id` — String — Stripe transaction ID
  - `created_at` — DateTime<Utc> — Creation timestamp
  - `completed_at` — Option<DateTime<Utc>> — Completion timestamp
  - `expires_at` — DateTime<Utc> — Expiration timestamp (30 minutes from creation)

- **Relationships**:
  - PaymentTransaction belongs to one Order
  - PaymentTransaction belongs to one Customer

- **Business Rules**:
  - [domain] Minimum transaction amount: $0.50 USD equivalent
  - [domain] Maximum transaction amount: $10,000 USD equivalent
  - [domain] Transactions expire after 30 minutes if not completed
  - [domain] Refunds allowed within 90 days of transaction
  - [domain] Amount must be positive
  - [domain] Currency must be one of USD, EUR, GBP

#### PaymentStatusHistory

- **Attributes**:
  - `id` — UUID — Unique identifier
  - `transaction_id` — TransactionId (UUID) — Related transaction
  - `previous_status` — Option<TransactionStatus> — Previous status
  - `new_status` — TransactionStatus — New status
  - `reason` — Option<String> — Reason for status change
  - `changed_at` — DateTime<Utc> — When status changed

- **Relationships**:
  - PaymentStatusHistory belongs to one PaymentTransaction

- **Business Rules**:
  - [domain] All status changes must be recorded
  - [domain] Audit trail is immutable

## Source Capabilities Summary

Based on external service dependencies and requirements:

- [x] Configuration — Environment configuration (STRIPE_SECRET_KEY, etc.)
- [x] Outbound HTTP — Stripe API calls and Order Service calls
- [x] Message publishing — Payment event notifications
- [x] Key-value state — Caching payment status
- [x] Authentication/Identity — Service-to-service JWT authentication
- [ ] Table/database access — Not required for this feature
- [ ] Real-time messaging — Not required for this feature

## Implementation Requirements

### External Dependencies

- Stripe Payment Gateway API (v2023-10-16) — Payment processing — Source: ATR-7300
- Order Service API — Order validation — Source: ATR-7300
- Payment Transactions Database — Transaction persistence — Source: ATR-7300 (attachment)

### Constraints

- [infrastructure] Must handle Stripe webhook events for async status updates
- [infrastructure] Must implement exponential backoff for retries
- [domain] PCI compliance handled by Stripe (no card data stored)
- [infrastructure] Stateless request handling (no long-lived process state)
- [domain] Transaction amounts in cents (integer) for Stripe API

## Dependencies

List JIRA issues referenced:

- ATR-7300 — Epic (Payment Gateway Integration)
- ATR-7301 — User Story (Process Credit Card Payment)

## Notes

- JIRA custom fields accessed: customfield_10016 (Story Points)
- BDD scenarios found in: Description field under "Other Information" section
- Technical documentation extracted from: Description sections and Attachments
- Markdown sections found: Technical Specifications, API Contract, Domain Model, External Services, Configuration
- Attachments processed: 2 files (stripe-api-spec.json, payment-schema.sql)
- Attachment support status: Available via MCP
- Ambiguities requiring clarification:
  - Webhook endpoint URL not specified
  - Retry backoff strategy details not provided
  - Concurrent payment handling strategy unclear
- Assumptions made:
  - Stripe API credentials provided via environment variables
  - Order Service is internal and uses JWT authentication
  - Database schema uses key-value state patterns
  - All amounts converted to cents for Stripe API

```

## Result

The analyzer successfully:

1. Fetched epic and user story with technical documentation sections
2. Parsed markdown sections: Technical Specifications, API Contract, Domain Model, External Services, Configuration
3. Fetched attachments via MCP (stripe-api-spec.json, payment-schema.sql)
4. Extracted and preserved JSON, SQL content from attachments
5. Generated enhanced Specify artifacts with comprehensive Technical Documentation section
6. Maintained traceability for all extracted content
7. Provided explicit technical requirements for code generation
8. Gracefully logged attachment support status

These enhanced Specify artifacts provide significantly more context for the crate-writer skill, resulting in higher quality code generation with proper:
- API endpoint implementations matching specifications
- Domain model types matching schema definitions
- Environment configuration matching requirements
- External service integration patterns
- Error handling based on specified error codes
```
