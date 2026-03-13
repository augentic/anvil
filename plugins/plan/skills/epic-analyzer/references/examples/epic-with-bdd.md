# Example 2: Epic with Comprehensive BDD Scenarios

## Scenario

Analyze a JIRA epic where BDD scenarios are extensive and include edge cases, error handling, and complex business rules. This demonstrates how to parse detailed BDD syntax and extract comprehensive test scenarios.

## Input

**Arguments**:

```text
$ARGUMENTS[0]: ATR-7200
$ARGUMENTS[1]: ./.specify/changes/payment-processing
```

**JIRA Epic** (ATR-7200):

Summary: Payment Processing with Multiple Methods
Description: Implement payment processing supporting credit card and PayPal with fraud detection

**User Story** (ATR-7201):

Summary: Process Credit Card Payment

Acceptance Criteria:
1. Support Visa, Mastercard, and Amex
2. Validate card number using Luhn algorithm
3. Check expiration date is in future
4. Process payment through payment gateway
5. Return transaction ID on success
6. Implement retry logic for transient failures
7. Flag suspicious transactions for review

BDD Scenarios:

```gherkin
Scenario: Successfully process valid credit card payment
  Given customer has order with total 99.99
  And payment method is credit card
  And card number is "4532-1234-5678-9010" (valid Visa)
  And expiration date is "12/2027"
  And CVV is "123"
  When customer submits payment
  Then payment is processed successfully
  And transaction ID is returned
  And order status is updated to "Paid"

Scenario: Reject expired credit card
  Given customer has order with total 50.00
  And card expiration date is "01/2020"
  When customer submits payment
  Then payment fails with error "Card expired"
  And order status remains "Pending"

Scenario: Retry transient gateway failures
  Given payment gateway is experiencing intermittent errors
  And customer submits valid payment
  When gateway returns 503 Service Unavailable
  Then system retries payment up to 3 times
  And waits 2 seconds between retries
  And if all retries fail, returns "Payment gateway unavailable"

Scenario: Flag large transaction for fraud review
  Given customer has order with total 10000.00
  And customer's previous orders total less than 500.00
  When customer submits payment
  Then payment is held for review
  And order status is set to "Under Review"
  And fraud team is notified
```

## Expected Specify Artifacts (Excerpt)

```markdown
## Business Logic Blocks

### Block: process_credit_card_payment

- **Source**: User Story ATR-7201, All Acceptance Criteria
- **Purpose**: Process credit card payment with validation and retry logic

**Algorithm** (pseudocode):

1) [domain] Validate card number using Luhn algorithm
2) [domain] Check expiration date is in future
3) [domain] Validate CVV is 3-4 digits
4) [domain] Check card type (Visa/Mastercard/Amex) matches first digits
5) [domain] Check transaction amount against fraud threshold (> $5000)
6) [domain] If amount > $5000 and customer history < $500, flag for review
7) [infrastructure] If flagged, send notification to fraud team
8) [infrastructure] Call payment gateway API with card details
9) [infrastructure] If gateway returns 503, retry up to 3 times with 2s delay
10) [domain] If payment succeeds, update order status to "Paid"
11) [domain] If payment fails after retries, return error
12) [domain] Return transaction ID

**Edge Cases & Failure Modes**:

- [domain] Expired card → immediate rejection
- [domain] Invalid card number (fails Luhn) → immediate rejection
- [infrastructure] Gateway timeout → retry with exponential backoff
- [domain] Large transaction with new customer → fraud review
- [unknown] How to handle partial refunds

**Unknowns**:

- unknown — exact fraud detection threshold not specified (assumed $5000)
- unknown — definition of "customer history" (total past orders?)
- TODO: Implement delay/sleep mechanism
- Reason: WASI does not support sleep/delay operations
```

## Result

Demonstrates:

- Parsing complex BDD scenarios with multiple steps
- Extracting retry logic and timing requirements
- Identifying fraud detection rules from scenarios
- Marking unknowns for ambiguous thresholds
- TODO markers for WASI limitations (delays)
