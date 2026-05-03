# Reverse-Engineer A Contract From A Legacy TypeScript Codebase

Use this test to verify that `/spec:define` can reverse-engineer Specify
contract artifacts from a legacy TypeScript codebase whose API surface a
prior `/spec:analyze --kind legacy-code` run has already identified.

Pipeline note:

- In the `contracts` schema, `/spec:define` creates `proposal.md`,
  `specs/**/*.md`, and `tasks.md`; contract YAML is produced during
  `/spec:build`.
- Omnia and Vectis implementation changes consume existing baseline contracts
  as context. Reverse-engineered interface shapes should be introduced through
  a separate `contracts@v1` change before implementation depends on them.
- Extract-from-source changes assume `/spec:analyze --kind legacy-code` has
  already produced a `discovery.md` capability summary identifying the API
  surface; this test stipulates that precondition rather than exercising it.

## Prerequisite

This test assumes a prior `/spec:analyze --kind legacy-code` run against
`vendor/orders-service/` has appended this capability block to the plan's
`discovery.md` (shape pinned by `plugins/spec/skills/analyze/SKILL.md`):

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

## Source Code

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

## Prompt

Invoke `/spec:define` in extract-from-source mode:

```text
/spec:define orders-api-contract

Reverse-engineer API contracts from an existing TypeScript service.

Authorship Mode: Extract from source code
Source Material:
- vendor/orders-service/src/index.ts
- vendor/orders-service/src/orders/handlers.ts
- vendor/orders-service/src/orders/types.ts
Analysis Context:
- discovery.md capability: orders
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

## Expected Contract Files

During `/spec:build`, the change should produce these change-local contract
deltas. After merge, the same paths become root `contracts/` baseline files.

- `contracts/http/orders-api.yaml`
- `contracts/schemas/create-order-request.yaml`
- `contracts/schemas/order-item.yaml`
- `contracts/schemas/order.yaml`
- `contracts/schemas/error-response.yaml`

The resulting specs should mark Content-Type, authentication, pagination,
and rate-limit fields as `[unknown]` because the TypeScript source does not
encode them. Endpoints or payloads outside the `orders` capability listed in
Analysis Context must surface as `[manual review required]` rather than be
silently included.
