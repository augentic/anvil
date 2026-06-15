---
id: contracts-design
owner: contracts
kind: adapter
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - files-exist
  - contract-validator-clean
expected-artifacts:
  - contracts/http/returns-api.yaml
  - contracts/schemas/return-request.yaml
  - contracts/schemas/return-item.yaml
  - contracts/schemas/return-request-accepted.yaml
  - contracts/schemas/return-status.yaml
  - contracts/schemas/error-response.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
  - implementation-shapes-authored-inline
---

# Generate From A Design Document Passed To `/spec:refine`

Scenario ID: `contracts-design`

Use this test to verify that `/spec:refine` can turn a named prose design
document into Specify artifacts detailed enough for contract generation.

Pipeline note:

- In the `contracts` schema, `/spec:refine` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts as
  context. New or changed interface shapes should be introduced through a
  separate `contracts@1.0.0` change before implementation depends on them.

## Intent

Prove that the `contracts@1.0.0` slice loop can author HTTP and JSON Schema
artifacts when the requirements live in a separate prose design document
referenced by the `/spec:refine` prompt as `Source Material:`. The scenario
exercises the source-document path of the prose authorship mode end-to-end.

## Workspace

- **Adapter:** `contracts@1.0.0`.
- **Project shape:** a single project initialised with the `contracts@1.0.0`
  schema (run `/spec:init` first if the workspace is fresh).
- **Registry shape:** not applicable.
- **Isolation:** `fresh-project`. Start from an empty `contracts/` baseline.
- **Backend:** `manual` — a human or agent runs the prompts in **Invocation**
  and records results in the [run summary](run-summary-template.md).

## Inputs

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

## Invocation

Invoke `/spec:refine` with the document named as source material:

```text
/spec:refine returns-api-contract

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

After `/spec:refine` succeeds, drive `/spec:build returns-api-contract` to
produce the contract YAML, then optionally `/spec:merge returns-api-contract`
to promote the deltas into the baseline.

## Expected Artifacts

During `/spec:build`, the slice should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/returns-api.yaml`
- `contracts/schemas/return-request.yaml`
- `contracts/schemas/return-item.yaml`
- `contracts/schemas/return-request-accepted.yaml`
- `contracts/schemas/return-status.yaml`
- `contracts/schemas/error-response.yaml`

## Assertions

- `files-exist`: every path in **Expected Artifacts** exists in the slice
  working tree after `/spec:build`, and in the baseline `contracts/` tree
  after `/spec:merge` if merge is run.
- `contract-validator-clean`: the build's contract verifier exits `0` with no
  findings and no manual-review warnings on the produced artifacts.

## Negative Expectations

- `artifacts-outside-contracts-directory`: no contract YAML is written outside
  `contracts/http/` or `contracts/schemas/`. The slice must not author
  implementation files (Omnia crates, Vectis Crux modules, etc.).
- `implementation-shapes-authored-inline`: the slice must not pre-author
  Omnia/Vectis interface shapes; only the `contracts@1.0.0` artifacts above are
  produced.

## Cleanup

Drop or archive the slice before moving to the next scenario unless you
explicitly want the new baseline contracts to persist. Remove the seeded
`docs/returns-api-design.md` if your run-all sequence requires a clean
working tree.
