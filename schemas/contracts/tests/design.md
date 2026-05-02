# Generate From A Design Document Passed To `/spec:define`

Use this test to verify that `/spec:define` can turn a named prose design
document into Specify artifacts detailed enough for contract generation.

Pipeline note:

- In the `contracts` schema, `/spec:define` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts as
  context. New or changed interface shapes should be introduced through a
  separate `contracts@v1` change before implementation depends on them.

## Source Document

Create a source design document such as `docs/returns-api-design.md`:

```markdown
# Returns API Design

The returns service lets customers request a return authorization for shipped
orders.

Producer: returns-service
Consumers: storefront, customer-support-console

## HTTP Interface

POST /returns
Creates a return request.

Request ReturnRequest:
- order_id: string, required
- customer_id: string, required
- reason: string, required, enum: damaged, wrong_item, no_longer_needed, other
- items: array of ReturnItem, required, minItems 1

ReturnItem:
- sku: string, required
- quantity: integer, required, minimum 1

Responses:
- 202 ReturnRequestAccepted with return_id: string, status: string enum
  pending_review|approved|rejected, created_at: date-time
- 400 ErrorResponse for invalid input
- 404 ErrorResponse when order_id is unknown
- 409 ErrorResponse when the order is not returnable

GET /returns/{return_id}
Returns current return status.

Responses:
- 200 ReturnStatus with return_id, status, updated_at
- 404 ErrorResponse when return_id is unknown
```

## Prompt

Invoke `/spec:define` with the document named as source material:

```text
/spec:define returns-api-contract

Generate API contracts from the design document at docs/returns-api-design.md.

Authorship Mode: Generate from prose
Source Material: docs/returns-api-design.md
Participants:
- returns-service: producer
- storefront: consumer
- customer-support-console: consumer

The change should define the Returns HTTP API and produce JSON Schema payloads
plus an OpenAPI 3.1 binding.
```

## Expected Contract Files

During `/spec:build`, the change should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/returns-api.yaml`
- `contracts/schemas/return-request.yaml`
- `contracts/schemas/return-item.yaml`
- `contracts/schemas/return-request-accepted.yaml`
- `contracts/schemas/return-status.yaml`
- `contracts/schemas/error-response.yaml`
