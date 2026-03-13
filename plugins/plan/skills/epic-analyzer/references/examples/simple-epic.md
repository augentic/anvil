# Example 1: Simple Epic with User Stories

## Scenario

Analyze a simple JIRA epic with 2 user stories that have acceptance criteria and BDD scenarios. The epic represents a feature for creating and retrieving orders in an e-commerce system.

## Input

**Arguments**:

```text
$ARGUMENTS[0]: ATR-7102
$ARGUMENTS[1]: ./.specify/changes/order-feature
```

**JIRA Data Structure** (Epic ATR-7102):

```json
{
  "key": "ATR-7102",
  "fields": {
    "summary": "Order Management System",
    "description": "Implement basic order creation and retrieval functionality for the e-commerce platform",
    "status": { "name": "In Progress" },
    "issuetype": { "name": "Epic" },
    "labels": ["backend", "api", "orders"],
    "components": [{ "name": "Order Service" }],
    "created": "2026-01-15T09:00:00.000Z",
    "updated": "2026-02-01T14:30:00.000Z"
  }
}
```

**User Story ATR-7103** (Linked to Epic):

````json
{
  "key": "ATR-7103",
  "fields": {
    "summary": "Create Order API Endpoint",
    "description": "As a customer, I want to create an order with multiple items so that I can purchase products.\n\n### Acceptance Criteria\n\n1. User can submit order with customer ID and list of items\n2. System validates that customer exists\n3. System validates that all products are in stock\n4. Order is created with status 'Pending'\n5. Order total is calculated from item prices\n6. System returns created order with order ID\n7. Error returned if customer not found (404)\n8. Error returned if products out of stock (400)\n\n### Other Information\n\nBDD Scenarios:\n\n```gherkin\nScenario: Successfully create order with valid items\n  Given customer \"CUST-001\" exists in the system\n  And product \"PROD-123\" has 10 units in stock\n  And product \"PROD-456\" has 5 units in stock\n  When customer submits order with items:\n    | product_id | quantity | price |\n    | PROD-123   | 2        | 29.99 |\n    | PROD-456   | 1        | 49.99 |\n  Then order is created successfully\n  And order status is 'Pending'\n  And order total is 109.97\n  And order ID is returned\n\nScenario: Reject order when customer not found\n  Given customer \"CUST-999\" does not exist\n  When customer submits order with valid items\n  Then request fails with status 404\n  And error message is \"Customer not found\"\n\nScenario: Reject order when product out of stock\n  Given customer \"CUST-001\" exists\n  And product \"PROD-123\" has 0 units in stock\n  When customer submits order with product \"PROD-123\"\n  Then request fails with status 400\n  And error message is \"Product out of stock: PROD-123\"\n```",
    "status": { "name": "To Do" },
    "issuetype": { "name": "Story" },
    "priority": { "name": "High" },
    "customfield_10016": 5
  }
}
````

**User Story ATR-7104** (Linked to Epic):

````json
{
  "key": "ATR-7104",
  "fields": {
    "summary": "Get Order by ID API Endpoint",
    "description": "As a customer, I want to retrieve my order details by order ID so that I can check the status and items.\n\n### Acceptance Criteria\n\n1. User can request order details with order ID\n2. System returns order with all details (ID, customer, items, total, status, timestamps)\n3. System returns 404 if order not found\n4. Only the order owner can view their order (authorization check)\n\n### Other Information\n\nBDD Scenarios:\n\n```gherkin\nScenario: Successfully retrieve order\n  Given order \"ORD-001\" exists for customer \"CUST-001\"\n  And the user is authenticated as customer \"CUST-001\"\n  When user requests order \"ORD-001\"\n  Then order details are returned\n  And response includes order ID, customer ID, items, total, status, created_at\n\nScenario: Return 404 for non-existent order\n  Given order \"ORD-999\" does not exist\n  When user requests order \"ORD-999\"\n  Then request fails with status 404\n  And error message is \"Order not found\"\n\nScenario: Reject unauthorized access\n  Given order \"ORD-001\" exists for customer \"CUST-001\"\n  And the user is authenticated as customer \"CUST-002\"\n  When user requests order \"ORD-001\"\n  Then request fails with status 403\n  And error message is \"Access denied\"\n```",
    "status": { "name": "To Do" },
    "issuetype": { "name": "Story" },
    "priority": { "name": "Medium" },
    "customfield_10016": 3
  }
}
````

## Execution

```text
epic-analyzer ATR-7102 ./.specify/changes/order-feature
```

## MCP Tool Calls

```
1. CallMcpTool:
     server: "jira"
     toolName: "get_issue"
     arguments: {"issueKey": "ATR-7102"}

   Response: Epic metadata (shown above)

2. CallMcpTool:
     server: "jira"
     toolName: "search_issues"
     arguments: {"jql": "\"Epic Link\" = ATR-7102"}

   Response: List of user stories [ATR-7103, ATR-7104]

3. CallMcpTool:
     server: "jira"
     toolName: "get_issue"
     arguments: {"issueKey": "ATR-7103"}

   Response: User story details (shown above)

4. CallMcpTool:
     server: "jira"
     toolName: "get_issue"
     arguments: {"issueKey": "ATR-7104"}

   Response: User story details (shown above)
```

## Expected Output

**Generated Specify Artifacts** (`.specify/changes/order-feature/` - Excerpt):

````markdown
# Specify Artifacts

Generated from: JIRA Epic ATR-7102
Generated at: 2026-02-04T10:30:00Z

## Epic Metadata

- **Key**: ATR-7102
- **Summary**: Order Management System
- **Description**: Implement basic order creation and retrieval functionality for the e-commerce platform
- **Status**: In Progress
- **Labels**: backend, api, orders
- **Components**: Order Service
- **Created**: 2026-01-15T09:00:00.000Z
- **Updated**: 2026-02-01T14:30:00.000Z

## User Stories

### User Story: ATR-7103

- **Summary**: Create Order API Endpoint
- **Description**: As a customer, I want to create an order with multiple items so that I can purchase products.
- **Status**: To Do
- **Story Points**: 5
- **Priority**: High

#### Acceptance Criteria

1. User can submit order with customer ID and list of items
2. System validates that customer exists
3. System validates that all products are in stock
4. Order is created with status 'Pending'
5. Order total is calculated from item prices
6. System returns created order with order ID
7. Error returned if customer not found (404)
8. Error returned if products out of stock (400)

#### BDD Scenarios

**Scenario: Successfully create order with valid items**

- Given: customer "CUST-001" exists in the system
- And: product "PROD-123" has 10 units in stock
- And: product "PROD-456" has 5 units in stock
- When: customer submits order with items (PROD-123 x2 @ 29.99, PROD-456 x1 @ 49.99)
- Then: order is created successfully
- And: order status is 'Pending'
- And: order total is 109.97
- And: order ID is returned

**Scenario: Reject order when customer not found**

- Given: customer "CUST-999" does not exist
- When: customer submits order with valid items
- Then: request fails with status 404
- And: error message is "Customer not found"

**Scenario: Reject order when product out of stock**

- Given: customer "CUST-001" exists
- And: product "PROD-123" has 0 units in stock
- When: customer submits order with product "PROD-123"
- Then: request fails with status 400
- And: error message is "Product out of stock: PROD-123"

### User Story: ATR-7104

- **Summary**: Get Order by ID API Endpoint
- **Description**: As a customer, I want to retrieve my order details by order ID so that I can check the status and items.
- **Status**: To Do
- **Story Points**: 3
- **Priority**: Medium

#### Acceptance Criteria

1. User can request order details with order ID
2. System returns order with all details (ID, customer, items, total, status, timestamps)
3. System returns 404 if order not found
4. Only the order owner can view their order (authorization check)

#### BDD Scenarios

**Scenario: Successfully retrieve order**

- Given: order "ORD-001" exists for customer "CUST-001"
- And: the user is authenticated as customer "CUST-001"
- When: user requests order "ORD-001"
- Then: order details are returned
- And: response includes order ID, customer ID, items, total, status, created_at

**Scenario: Return 404 for non-existent order**

- Given: order "ORD-999" does not exist
- When: user requests order "ORD-999"
- Then: request fails with status 404
- And: error message is "Order not found"

**Scenario: Reject unauthorized access**

- Given: order "ORD-001" exists for customer "CUST-001"
- And: the user is authenticated as customer "CUST-002"
- When: user requests order "ORD-001"
- Then: request fails with status 403
- And: error message is "Access denied"

## Domain Model

### Entities

#### Order

- **Attributes**:
  - `id` — OrderId (String) — Unique identifier for the order
  - `customer_id` — CustomerId (String) — Customer who placed the order
  - `items` — Vec<OrderItem> — List of items in the order
  - `total` — Decimal — Total price of all items
  - `status` — OrderStatus (Enum) — Current status of the order
  - `created_at` — DateTime<Utc> — When order was created
  - `updated_at` — DateTime<Utc> — When order was last updated

- **Relationships**:
  - Order belongs to one Customer
  - Order has many OrderItems

- **Business Rules**:
  - [domain] Order must contain at least one item
  - [domain] Order total must match sum of item prices
  - [domain] Initial status must be 'Pending'
  - [domain] Customer must exist before order creation

#### OrderItem

- **Attributes**:
  - `product_id` — ProductId (String) — Product identifier
  - `quantity` — u32 — Quantity ordered
  - `price` — Decimal — Price per unit at time of order

- **Relationships**:
  - OrderItem belongs to one Order

- **Business Rules**:
  - [domain] Quantity must be greater than 0
  - [domain] Price must be greater than 0

#### Customer

- **Attributes**:
  - `id` — CustomerId (String) — Unique identifier
  - [unknown — other attributes not specified in JIRA]

- **Unknowns**:
  - unknown — Customer entity details not fully specified in requirements

#### Product

- **Attributes**:
  - `id` — ProductId (String) — Unique identifier
  - `stock_quantity` — u32 — Available units in stock
  - [unknown — other attributes not specified in JIRA]

- **Unknowns**:
  - unknown — Product entity details not fully specified in requirements

## API Contracts

### Endpoint: Create Order

- **Path**: `/orders`
- **Method**: `POST`
- **Summary**: Create a new order with items
- **Source**: User Story ATR-7103

**Request Body** (required):

```json
{
  "customer_id": "string — Customer identifier",
  "items": [
    {
      "product_id": "string — Product identifier",
      "quantity": "number — Quantity to order",
      "price": "number — Price per unit"
    }
  ]
}
```
````

**Response** (Success - 201 Created):

```json
{
  "id": "string — Order identifier",
  "customer_id": "string — Customer identifier",
  "items": "array — List of order items",
  "total": "number — Total order amount",
  "status": "string — Order status (Pending)",
  "created_at": "string — ISO 8601 timestamp"
}
```

**Error Responses**:

| Status | Condition             | Response                                          |
| ------ | --------------------- | ------------------------------------------------- |
| 400    | Products out of stock | `{"error": "Product out of stock: {product_id}"}` |
| 404    | Customer not found    | `{"error": "Customer not found"}`                 |

**Authentication**: [unknown — not specified in JIRA]

**Unknowns**:

- unknown — authentication method not specified

### Endpoint: Get Order by ID

- **Path**: `/orders/{order_id}`
- **Method**: `GET`
- **Summary**: Retrieve order details by ID
- **Source**: User Story ATR-7104

**Request Parameters**:

| Name     | Location | Type   | Required | Description      |
| -------- | -------- | ------ | -------- | ---------------- |
| order_id | path     | string | Yes      | Order identifier |

**Response** (Success - 200 OK):

```json
{
  "id": "string — Order identifier",
  "customer_id": "string — Customer identifier",
  "items": [
    {
      "product_id": "string",
      "quantity": "number",
      "price": "number"
    }
  ],
  "total": "number — Total order amount",
  "status": "string — Order status",
  "created_at": "string — ISO 8601 timestamp",
  "updated_at": "string — ISO 8601 timestamp"
}
```

**Error Responses**:

| Status | Condition           | Response                       |
| ------ | ------------------- | ------------------------------ |
| 403    | Unauthorized access | `{"error": "Access denied"}`   |
| 404    | Order not found     | `{"error": "Order not found"}` |

**Authentication**: Required (user must be authenticated)

**Unknowns**:

- unknown — specific authentication mechanism not detailed

## Business Logic Blocks

### Block: validate_and_create_order

- **Source**: User Story ATR-7103, Acceptance Criteria 1-6
- **Purpose**: Validate order request and create new order

**Algorithm** (pseudocode):

1. [domain] Parse request body and extract customer_id and items
2. [infrastructure] Fetch customer from external service to verify exists
3. [domain] Return 404 error if customer not found
4. [domain] Validate items list is not empty
5. [mechanical] For each item in request:
   a) [infrastructure] Check product stock quantity via external service
   b) [domain] Return 400 error if stock insufficient
6. [mechanical] Calculate total = sum(item.quantity \* item.price for all items)
7. [domain] Create order object with status = "Pending"
8. [infrastructure] Persist order to database
9. [domain] Return created order with generated ID

**Preconditions**:

- [domain] Request contains valid customer_id
- [domain] Request contains at least one item
- [domain] All items have positive quantity and price

**Postconditions**:

- [domain] Order exists in system with status "Pending"
- [domain] Order has unique ID
- [domain] Order total equals sum of item prices

**Edge Cases & Failure Modes**:

- [domain] Customer does not exist → 404 error
- [domain] Product out of stock → 400 error
- [domain] Empty items list → 400 error
- [unknown] Multiple concurrent orders depleting stock

**Inputs**:

- `customer_id` — String — Customer identifier
- `items` — Vec<OrderItem> — List of items to order

**Outputs**:

- `order` — Order — Created order object

**Errors**:

- CustomerNotFound (404) → "Customer not found"
- ProductOutOfStock (400) → "Product out of stock: {product_id}"
- InvalidInput (400) → "Order must contain at least one item"

**Unknowns**:

- unknown — transaction handling for stock deduction not specified
- unknown — inventory reservation mechanism not detailed

### Block: retrieve_order_with_authorization

- **Source**: User Story ATR-7104, Acceptance Criteria 1-4
- **Purpose**: Retrieve order by ID with authorization check

**Algorithm** (pseudocode):

1. [domain] Extract order_id from path parameter
2. [infrastructure] Fetch order from database by ID
3. [domain] Return 404 error if order not found
4. [domain] Get authenticated user's customer_id
5. [domain] Compare order.customer_id with authenticated customer_id
6. [domain] Return 403 error if customer_ids don't match
7. [domain] Return order details

**Preconditions**:

- [domain] User is authenticated
- [domain] Order ID is valid format

**Postconditions**:

- [domain] Order details returned only to owner

**Edge Cases & Failure Modes**:

- [domain] Order not found → 404 error
- [domain] User not authenticated → [unknown — behavior not specified]
- [domain] User not order owner → 403 error

**Inputs**:

- `order_id` — OrderId — Order identifier
- `authenticated_customer_id` — CustomerId — Current user's customer ID

**Outputs**:

- `order` — Order — Order details

**Errors**:

- OrderNotFound (404) → "Order not found"
- AccessDenied (403) → "Access denied"

**Unknowns**:

- unknown — authentication mechanism not specified
- unknown — how to obtain authenticated customer_id

## Source Capabilities Summary

Based on external service dependencies and requirements:

- [x] Configuration — Environment configuration
- [x] Outbound HTTP — External calls to customer service and inventory service
- [ ] Message publishing — Not required for this feature
- [x] Key-value state — Persistent order storage
- [x] Authentication/Identity — Authentication for authorization checks
- [ ] Table/database access — Not required for this feature
- [ ] Real-time messaging — Not required for this feature

## Implementation Requirements

### External Dependencies

- Customer Service API — Validate customer exists — Source: ATR-7103
- Inventory Service API — Check product stock — Source: ATR-7103
- Order Database — Persist and retrieve orders — Source: ATR-7103, ATR-7104

### Constraints

- [infrastructure] All I/O through capability abstractions (no direct network/file access)
- [infrastructure] Stateless request handling (no long-lived process state)
- [unknown — performance requirements not specified]
- [unknown — database schema not specified]

## Dependencies

List JIRA issues referenced:

- ATR-7102 — Epic (Order Management System)
- ATR-7103 — User Story (Create Order API Endpoint)
- ATR-7104 — User Story (Get Order by ID API Endpoint)

## Notes

- JIRA custom fields accessed: customfield_10016 (Story Points)
- BDD scenarios found in: Description field under "Other Information" section
- Ambiguities requiring clarification:
  - Authentication mechanism not specified
  - External service endpoints not provided (customer service, inventory service)
  - Database schema for orders not defined
  - Stock reservation/deduction logic not detailed
- Assumptions made:
  - Customer and Inventory services are external HTTP APIs
  - Orders are stored in a state store (key-value or database)
  - Order IDs are generated by the system (not provided in request)

```

## Result

The analyzer successfully:

1. Fetched epic metadata from JIRA MCP server
2. Retrieved linked user stories using JQL search
3. Extracted acceptance criteria from description field
4. Parsed BDD scenarios from "Other Information" section
5. Inferred domain model with Order, OrderItem, Customer, Product entities
6. Inferred API contracts: POST /orders and GET /orders/{order_id}
7. Extracted business logic blocks with proper tags
8. Marked ambiguities and unknowns explicitly
9. Generated Specify artifacts ready for crate-writer skill
```
