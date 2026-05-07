# Import A Contract Passed To `/spec:define`

Use this test to verify that an externally supplied OpenAPI document is imported,
upgraded if needed, decomposed into shared schemas, and verified.

Pipeline note:

- In the `contracts` schema, `/spec:define` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; import normalization is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts as
  context. Imported interface shapes should be introduced through a separate
  `contracts@v1` change before implementation depends on them.

## Source Contract

Create an external OpenAPI document, for example
`vendor/ticket-api.openapi.yaml`:

```yaml
openapi: "3.0.3"
info:
  title: Ticket API
  version: "1.0.0"
paths:
  /tickets:
    post:
      operationId: createTicket
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/CreateTicketRequest"
      responses:
        "201":
          description: Ticket created.
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Ticket"
components:
  schemas:
    CreateTicketRequest:
      type: object
      required: [subject, requester_email]
      properties:
        subject:
          type: string
        requester_email:
          type: string
          format: email
    Ticket:
      type: object
      required: [id, subject, status]
      properties:
        id:
          type: string
        subject:
          type: string
        status:
          type: string
          enum: [open, pending, closed]
```

## Prompt

Invoke `/spec:define` in import mode:

```text
/spec:define import-ticket-api-contract

Import existing contracts.

Authorship Mode: Import existing contracts
Source Material: vendor/ticket-api.openapi.yaml
Participants:
- ticket-service: producer
- support-console: consumer

Normalize the supplied OpenAPI document into Specify contract conventions.
Preserve the endpoint behavior from the source contract, upgrade to OpenAPI 3.1
if needed, decompose inline schemas into contracts/schemas, and verify the
resulting contract artifacts.
```

## Expected Contract Files

During `/spec:build`, the import should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/ticket-api.yaml`, upgraded to OpenAPI 3.1
- `contracts/schemas/create-ticket-request.yaml`
- `contracts/schemas/ticket.yaml`

The import report should identify the source format, any lossless upgrades, any
manual-review warnings, and the verifier result.
