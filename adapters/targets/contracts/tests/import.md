---
id: contracts-import
owner: contracts
kind: adapter
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
isolation: fresh-project
authorship-mode: import
assertions:
  - files-exist
  - contract-validator-clean
  - import-report-present
expected-artifacts:
  - contracts/http/ticket-api.yaml
  - contracts/schemas/create-ticket-request.yaml
  - contracts/schemas/ticket.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
  - source-format-not-upgraded-to-openapi-31
  - inline-schemas-not-decomposed
---

# Import A Contract Passed To `/spec:refine`

Scenario ID: `contracts-import`

Use this test to verify that an externally supplied OpenAPI document is imported,
upgraded if needed, decomposed into shared schemas, and verified.

Pipeline note:

- In the `contracts` schema, `/spec:refine` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; import normalization is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts as
  context. Imported interface shapes should be introduced through a separate
  `contracts@1.0.0` change before implementation depends on them.

## Intent

Prove that the `contracts@1.0.0` slice loop can normalise an externally supplied
OpenAPI document: it preserves the source endpoint behavior, upgrades the
document to OpenAPI 3.1, decomposes inline schemas into `contracts/schemas/`,
and runs the verifier on the resulting artifacts.

## Workspace

- **Adapter:** `contracts@1.0.0`.
- **Project shape:** a single project initialised with the `contracts@1.0.0`
  schema (run `/spec:init` first if the workspace is fresh).
- **Registry shape:** not applicable.
- **Isolation:** `fresh-project`. Start from an empty `contracts/` baseline.
- **Backend:** `manual` — a human or agent runs the prompts in **Invocation**
  and records results in the [run summary](run-summary-template.md).

## Inputs

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

## Invocation

Invoke `/spec:refine` in import mode:

```text
/spec:refine import-ticket-api-contract

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

After `/spec:refine` succeeds, drive `/spec:build import-ticket-api-contract`
to produce the normalised contract YAML, then optionally
`/spec:merge import-ticket-api-contract` to promote the deltas into the
baseline.

## Expected Artifacts

During `/spec:build`, the import should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/ticket-api.yaml`, upgraded to OpenAPI 3.1
- `contracts/schemas/create-ticket-request.yaml`
- `contracts/schemas/ticket.yaml`

The import report should identify the source format, any lossless upgrades, any
manual-review warnings, and the verifier result.

## Assertions

- `files-exist`: every path in **Expected Artifacts** exists in the slice
  working tree after `/spec:build`.
- `contract-validator-clean`: the build's contract verifier exits `0` with no
  unresolved `$ref` failures, missing schema metadata, or binding coverage
  failures on the imported artifacts. Manual-review warnings are surfaced in
  the run summary but do not by themselves fail this assertion.
- `import-report-present`: the build phase produces an import report (in build
  output or proposal annotations) identifying the source format, any lossless
  upgrades performed, manual-review warnings, and the verifier result.

## Negative Expectations

- `artifacts-outside-contracts-directory`: no contract YAML is written outside
  `contracts/http/` or `contracts/schemas/`.
- `source-format-not-upgraded-to-openapi-31`: the resulting `contracts/http/ticket-api.yaml`
  must declare `openapi: "3.1.x"`. Leaving the document at OpenAPI 3.0.3 (the
  source version) is a failure of this scenario.
- `inline-schemas-not-decomposed`: `CreateTicketRequest` and `Ticket` must
  appear as standalone `contracts/schemas/*.yaml` files; leaving them inline
  under `components.schemas` in the HTTP document is a failure.

## Cleanup

Drop or archive the slice before moving to the next scenario. Remove
`vendor/ticket-api.openapi.yaml` if your run-all sequence requires a clean
working tree.
