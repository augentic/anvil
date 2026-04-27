# Order Processing Specification

## Purpose

Process customer orders through the order API.

### Requirement: Place Order

ID: REQ-001

The system SHALL accept a `POST /orders` request to place a new order with a customer ID and at least one line item.

#### Scenario: Successful Order Placement

- **WHEN** a valid `POST /orders` request is received with a `customer_id` and a non-empty `items` array (each item with `product_id` and `quantity`)
- **THEN** the system responds with `201 Created` and an `OrderPlaced` payload containing `order_id`, `customer_id`, `items`, `total_amount`, and `placed_at`

#### Scenario: Empty Items List

- **WHEN** a `POST /orders` request is received with an empty `items` array
- **THEN** the system responds with `400 Bad Request` and an `ErrorResponse` with code `EMPTY_ORDER`

### Requirement: Retrieve Order

ID: REQ-002

The system SHALL accept a `GET /orders/{id}` request and return the order details.

#### Scenario: Order Found

- **WHEN** a `GET /orders/{id}` request is received with a valid order ID
- **THEN** the system responds with `200 OK` and an `OrderPlaced` payload

#### Scenario: Order Not Found

- **WHEN** a `GET /orders/{id}` request is received with a non-existent order ID
- **THEN** the system responds with `404 Not Found` and an `ErrorResponse` with code `ORDER_NOT_FOUND`

## Error Conditions

- `EMPTY_ORDER`: Order request has no line items
- `ORDER_NOT_FOUND`: Requested order ID does not exist
