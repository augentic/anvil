# Spec — capability `a`

## Purpose

Classify an order into `small` / `medium` / `large` based on its `amount`, after validating
that the order has a non-empty `orderId` and a non-negative `amount`.

### Requirement: Order is classified by amount bucket

ID: REQ-001

Source: `src/a/handler.ts::classifyOrder`

The system SHALL classify an order into exactly one of `small`, `medium`, or `large`
based on its `amount` and the configured thresholds `SMALL_MAX` (50) and `MEDIUM_MAX` (500).

#### Scenario: Small order

Given an order with `amount = 25`
When `classifyOrder` is invoked
Then the result's `class` is `"small"`

#### Scenario: Medium order (boundary)

Given an order with `amount = 500`
When `classifyOrder` is invoked
Then the result's `class` is `"medium"`

#### Scenario: Large order

Given an order with `amount = 10000`
When `classifyOrder` is invoked
Then the result's `class` is `"large"`

### Requirement: Invalid orderId is rejected

ID: REQ-002

Source: `src/a/handler.ts::classifyOrder`

The system SHALL reject orders whose `orderId` fails the `nonEmpty` check.

#### Scenario: Empty orderId

Given an order with `orderId = ""`
When `classifyOrder` is invoked
Then an error is thrown with message `"orderId is required"`

### Requirement: Negative amount is rejected

ID: REQ-003

Source: `src/a/handler.ts::classifyOrder`

The system SHALL reject orders whose `amount` is negative.

#### Scenario: Negative amount

Given an order with `amount = -1`
When `classifyOrder` is invoked
Then an error is thrown with message `"amount must be non-negative"`

## Error Conditions

- `Error("orderId is required")` — raised when the `orderId` validation fails.
- `Error("amount must be non-negative")` — raised when `amount < 0`.

## Metrics

None observed in the scoped read set.
