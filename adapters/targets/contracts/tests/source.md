---
id: contracts-source
owner: contracts
kind: adapter
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
isolation: fresh-project
authorship-mode: extract
assertions:
  - files-exist
  - contract-validator-clean
  - unknown-fields-marked
  - out-of-scope-flagged-manual-review
expected-artifacts:
  - contracts/http/orders-api.yaml
  - contracts/schemas/create-order-request.yaml
  - contracts/schemas/order-item.yaml
  - contracts/schemas/order.yaml
  - contracts/schemas/error-response.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
  - silently-expanded-beyond-discovery-scope
  - wire-level-fields-guessed-not-marked-unknown
---

# Reverse-Engineer A Contract From A Legacy TypeScript Codebase

Scenario ID: `contracts-source`

Use this test to verify that `/spec:refine` can reverse-engineer Specify
contract artifacts from a legacy TypeScript codebase whose API surface a
prior `/spec:plan legacy-code` run has already identified.

Pipeline note:

- In the `contracts` schema, `/spec:refine` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts
  as context. Reverse-engineered interface shapes should be introduced through
  a separate `contracts@1.0.0` change before implementation depends on them.
- Extract-from-source changes assume `/spec:plan legacy-code` has
  already produced a `discovery.md` adapter summary identifying the API
  surface; this test stipulates that precondition rather than exercising it.

## Intent

Prove that the `contracts@1.0.0` slice loop can extract HTTP and JSON Schema
artifacts from a legacy TypeScript service when a prior `/spec:plan`
run has constrained the scope to one adapter. The scenario covers
analysis-bounded extraction: the contract change must stay inside the scoped
entry points and must mark wire-level fields as `[unknown]` rather than
guessing.

## Workspace

- **Adapter:** `contracts@1.0.0`.
- **Project shape:** a single project initialised with the `contracts@1.0.0`
  schema (run `/spec:init` first if the workspace is fresh).
- **Registry shape:** not applicable.
- **Isolation:** `fresh-project`. Start from an empty `contracts/` baseline.
- **Backend:** `manual` — a human or agent runs the prompts in **Invocation**
  and records results in the [run summary](run-summary-template.md).
- **Precondition:** the discovery block in **Inputs** below must already be
  present in the plan's `discovery.md`. The scenario stipulates this, it does
  not exercise `/spec:plan`.

## Inputs

### Discovery precondition

This test assumes a prior `/spec:plan legacy-code` run against
`vendor/orders-service/` has appended this adapter block to the plan's
`discovery.md` (shape pinned by `plugins/spec/skills/plan/SKILL.md`):

````markdown
### orders

```yaml
summary: Create and read customer orders.
sources:
  - src/index.ts
  - src/orders/handlers.ts
  - src/orders/types.ts
depends-on: []
hints:
  entry_points: [GET /orders/:orderId, POST /orders]
  external_deps: []
confidence: high
```
````

The `entry_points` list is the analysis-identified scope boundary for the
contract change; surface beyond `POST /orders` and `GET /orders/:orderId`
must be flagged `[manual review required]` rather than silently transcribed.

### Source code

Create a small legacy TypeScript service under `vendor/orders-service/`.

`vendor/orders-service/src/orders/types.ts`:

```ts
export type OrderStatus = "pending" | "shipped" | "delivered" | "cancelled";

export interface OrderItem {
  sku: string;
  quantity: number;
}

export interface CreateOrderRequest {
  customer_id: string;
  items: OrderItem[];
}

export interface Order {
  id: string;
  customer_id: string;
  status: OrderStatus;
  items: OrderItem[];
  created_at: string;
}

export interface ErrorResponse {
  code: string;
  message: string;
}
```

`vendor/orders-service/src/orders/handlers.ts`:

```ts
import { Request, Response } from "express";
import {
  CreateOrderRequest,
  ErrorResponse,
  Order,
} from "./types";
import { findOrder, persistOrder } from "./store";

export async function createOrder(req: Request, res: Response) {
  const body = req.body as CreateOrderRequest;
  if (!body.customer_id || !body.items?.length) {
    const err: ErrorResponse = {
      code: "INVALID_INPUT",
      message: "customer_id and items are required",
    };
    return res.status(400).json(err);
  }
  const order: Order = await persistOrder(body);
  return res.status(201).json(order);
}

export async function getOrder(req: Request, res: Response) {
  const order = await findOrder(req.params.orderId);
  if (!order) {
    const err: ErrorResponse = {
      code: "NOT_FOUND",
      message: "order not found",
    };
    return res.status(404).json(err);
  }
  return res.status(200).json(order);
}
```

`vendor/orders-service/src/index.ts`:

```ts
import express from "express";
import { createOrder, getOrder } from "./orders/handlers";

const app = express();
app.use(express.json());

app.post("/orders", createOrder);
app.get("/orders/:orderId", getOrder);

app.listen(3000);
```

## Invocation

Invoke `/spec:refine` in extract-from-source mode:

```text
/spec:refine orders-api-contract

Reverse-engineer API contracts from an existing TypeScript service.

Authorship Mode: Extract from source code
Source Material:
- vendor/orders-service/src/index.ts
- vendor/orders-service/src/orders/handlers.ts
- vendor/orders-service/src/orders/types.ts
Analysis Context:
- discovery.md adapter: orders
- entry_points: POST /orders, GET /orders/:orderId
Participants:
- orders-service: producer
- storefront: consumer
- fulfillment-console: consumer

Read the legacy TypeScript handlers, type declarations, and route
registrations to derive the interface that the service currently exposes.
Capture endpoint paths and methods from the express route registrations,
status codes from the handler return sites, and payload shapes from the
imported TypeScript interfaces and literal types. Mark wire-level details
that the source does not encode — Content-Type, auth headers, pagination
semantics, rate limits, and idempotency-key conventions — with [unknown]
rather than guessing. Stay within the analysis-identified scope; flag any
additional surface as [manual review required] rather than silently
expanding the contract change.
```

After `/spec:refine` succeeds, drive `/spec:build orders-api-contract` to
produce the contract YAML, then optionally `/spec:merge orders-api-contract`
to promote the deltas into the baseline.

## Expected Artifacts

During `/spec:build`, the slice should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/orders-api.yaml`
- `contracts/schemas/create-order-request.yaml`
- `contracts/schemas/order-item.yaml`
- `contracts/schemas/order.yaml`
- `contracts/schemas/error-response.yaml`

The resulting specs should mark Content-Type, authentication, pagination,
and rate-limit fields as `[unknown]` because the TypeScript source does not
encode them. Endpoints or payloads outside the `orders` adapter listed in
Analysis Context must surface as `[manual review required]` rather than be
silently included.

## Assertions

- `files-exist`: every path in **Expected Artifacts** exists in the slice
  working tree after `/spec:build`.
- `contract-validator-clean`: the build's contract verifier exits `0` with no
  unresolved `$ref` failures, missing schema metadata, or binding coverage
  failures on the extracted artifacts. Manual-review warnings are surfaced in
  the run summary but do not by themselves fail this assertion.
- `unknown-fields-marked`: Content-Type, authentication, pagination, and
  rate-limit fields appear in the slice specs marked `[unknown]` rather than
  filled in with guessed values.
- `out-of-scope-flagged-manual-review`: any surface beyond the entry points
  declared in the discovery block (`POST /orders`, `GET /orders/:orderId`)
  appears in the slice artifacts as `[manual review required]` and is not
  silently transcribed into the contract.

## Negative Expectations

- `artifacts-outside-contracts-directory`: no contract YAML is written outside
  `contracts/http/` or `contracts/schemas/`.
- `silently-expanded-beyond-discovery-scope`: the slice must not author
  contract entries for endpoints outside the analysis-declared `entry_points`.
- `wire-level-fields-guessed-not-marked-unknown`: Content-Type, auth headers,
  pagination semantics, rate limits, or idempotency-key conventions must not
  be filled in with values that the TypeScript source does not encode. Any
  guessed value is a failure of this scenario.

## Cleanup

Drop or archive the slice before moving to the next scenario. Remove the
seeded `vendor/orders-service/` tree if your run-all sequence requires a
clean working tree.
