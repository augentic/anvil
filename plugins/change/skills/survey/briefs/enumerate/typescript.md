---
id: enumerate
description: Per-language enumeration prompt for /change:survey covering TypeScript and JavaScript sources.
language: typescript
---

# TypeScript / JavaScript surface enumeration for `/change:survey`

This brief drives the LLM that produces a candidate `surfaces.json` for a single `legacy-code` source written in TypeScript or JavaScript. The skill stages your output at `<staged-dir>/<source-key>.json`; `specify change survey` then validates, canonicalizes, and writes the canonical sidecar. Schema or invariant failures come back through the bounded repair loop with a structured validator detail; you re-emit the candidate with the fault corrected.

JavaScript sources (`.js`, `.mjs`, `.cjs`, `.jsx`) are folded into this brief: the framework idioms (Express, NestJS decorators via Babel, BullMQ, Fastify, Next.js, yargs, `ws`, `fetch`, `axios`) are the same. Detect the file extension purely to widen the import-graph walk; the brief content does not branch on it.

## Scope

Frameworks covered in v1, with the signature that qualifies a call site as a surface:

- **Express** — an `import` of `express` whose default export (or `Router`) has `.get` / `.post` / `.put` / `.patch` / `.delete` / `.all` / `.use` (mount only when it attaches a handler, not when it composes middleware) invoked with a path string. Each route registration is one `http-route` surface.
- **NestJS** — a class decorated with `@Controller(...)` from `@nestjs/common`; each method on that class decorated with `@Get` / `@Post` / `@Put` / `@Patch` / `@Delete` / `@Options` / `@Head` is one `http-route` surface. Methods decorated with `@MessagePattern` / `@EventPattern` from `@nestjs/microservices` are `message-sub`. Classes decorated with `@WebSocketGateway` from `@nestjs/websockets` contribute one `ws-handler` per `@SubscribeMessage` method.
- **BullMQ** — `import` of `bullmq`. `new Queue(name, …)` followed by `queue.add(jobName, …)` is a `message-pub` (one surface per `jobName`). `queue.add(name, payload, { repeat: … })` is additionally a `scheduled-job` keyed on the repeat expression. `new Worker(name, handler, …)` is one `message-sub` per queue name.
- **Fastify** — `import` of `fastify`. `app.get` / `.post` / `.put` / `.patch` / `.delete` / `.route({ method, url, handler })` invocations with a path string are `http-route` surfaces.
- **Next.js App Router** — files matching `app/**/route.{ts,js,tsx,jsx}` that export a function named after an HTTP verb (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD`). Each exported verb is one `http-route` surface; the route path is derived from the file path relative to `app/` with `[seg]` → `:seg` and `(group)` segments stripped.
- **Next.js Pages Router** — files under `pages/api/**.{ts,js}` whose default export is a request handler. Each file is one `http-route` surface (method defaults to `ANY` unless the handler narrows it via `req.method`); the route path is derived from the file path relative to `pages/api/`.

Out of scope for this brief: tRPC, GraphQL resolvers, gRPC services, AWS Lambda handlers, and Cloudflare Workers. If the source uses one of those exclusively, emit `surfaces: []` and let the operator review.

## Schema

The candidate document MUST match the closed schema verbatim. This is the contract that `specify change survey` validates; do not paraphrase fields, rename keys, or invent properties.

Top-level object — required fields, no additional properties:

- `version` — integer, must equal `1`.
- `source-key` — kebab-case string matching `^[a-z][a-z0-9-]*$`.
- `language` — string; emit `typescript` when any `.ts` / `.tsx` file participates, otherwise `javascript`.
- `surfaces` — array of `Surface` objects, sorted by `id` after canonicalization (the CLI sorts on write; you may emit in any order).

Each `Surface` object — required fields, no additional properties:

- `id` — stable string unique within this file. Compose it from `<kind>-<verb-or-action>-<slug>` so reruns diff cleanly.
- `kind` — one of the closed enum, exactly:
  - `http-route`
  - `message-pub`
  - `message-sub`
  - `ws-handler`
  - `scheduled-job`
  - `cli-command`
  - `ui-route`
  - `external-call-out`
- `identifier` — the legacy spelling of the surface (e.g. `POST /users`, `user.created`, `cleanup-temp-files`, `GET https://api.example.com/v1/widgets`). Preserve the source's case and punctuation; do not normalize.
- `handler` — `<file>:<symbol>` where `<file>` is a relative path under the source root and `<symbol>` is the named export, class method, or synthetic suffix. See [`handler` resolution](#handler-resolution).
- `touches` — array of relative paths to source files reached from the handler. Sorted alphabetically by the CLI on write. See [`touches[]` resolution](#touches-resolution).
- `declared-at` — non-empty array of `<file>` or `<file>:<line>` entries (relative paths) pointing at the registration site, sorted alphabetically by the CLI on write.

Path rules — every entry of `touches[]` and `declared-at[]` MUST:

- Be a relative path. No leading `/`, no Windows drive letter (`C:\…`).
- Contain no `..` path segments.
- Resolve to a file under the source root.
- Live outside the skip-root set: `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`.

Violations exit `surfaces-touches-out-of-tree`. Duplicate `id` values exit `surfaces-id-collision`. Missing required fields or unknown `kind` values exit `surfaces-validation-failed`.

## Worked examples

Each example shows the relevant input snippet, the framework signature that fired, and the `Surface` JSON block you should emit. Paths are fictitious — match the shape, not the literal files.

### `http-route` — Express

Input (`src/server.ts`):

```ts
import express from "express";
import { registerUser } from "./auth/register";

const app = express();
app.post("/users", registerUser);
```

Signature: `import "express"` + `app.post("/users", …)`. Handler resolves through the named import to `src/auth/register.ts:registerUser`.

```json
{
  "id": "http-post-users",
  "kind": "http-route",
  "identifier": "POST /users",
  "handler": "src/auth/register.ts:registerUser",
  "touches": [
    "src/auth/register.ts",
    "src/notifications/email.ts",
    "src/users/repository.ts"
  ],
  "declared-at": ["src/server.ts:5"]
}
```

### `http-route` — NestJS controller

Input (`src/users/users.controller.ts`):

```ts
import { Controller, Post, Body } from "@nestjs/common";
import { UsersService } from "./users.service";
import { CreateUserDto } from "./create-user.dto";

@Controller("users")
export class UsersController {
  constructor(private readonly users: UsersService) {}

  @Post()
  create(@Body() dto: CreateUserDto) {
    return this.users.create(dto);
  }
}
```

Signature: `@Controller("users")` + `@Post()` from `@nestjs/common`. Handler is `UsersController.create`; identifier composes the controller prefix with the verb.

```json
{
  "id": "http-post-users",
  "kind": "http-route",
  "identifier": "POST /users",
  "handler": "src/users/users.controller.ts:UsersController.create",
  "touches": [
    "src/users/create-user.dto.ts",
    "src/users/users.controller.ts",
    "src/users/users.service.ts"
  ],
  "declared-at": ["src/users/users.controller.ts:9"]
}
```

### `http-route` — Fastify

Input (`src/widgets/routes.ts`):

```ts
import { FastifyInstance } from "fastify";
import { listWidgets } from "./list";

export default async function widgetRoutes(app: FastifyInstance) {
  app.get("/widgets", listWidgets);
}
```

Signature: `FastifyInstance` parameter + `app.get("/widgets", …)`. Handler resolves to `src/widgets/list.ts:listWidgets`.

```json
{
  "id": "http-get-widgets",
  "kind": "http-route",
  "identifier": "GET /widgets",
  "handler": "src/widgets/list.ts:listWidgets",
  "touches": [
    "src/widgets/list.ts",
    "src/widgets/repository.ts",
    "src/widgets/routes.ts"
  ],
  "declared-at": ["src/widgets/routes.ts:5"]
}
```

### `http-route` — Next.js App Router

Input (`app/api/orders/route.ts`):

```ts
import { NextResponse } from "next/server";
import { createOrder } from "@/lib/orders/create";

export async function POST(req: Request) {
  const body = await req.json();
  const order = await createOrder(body);
  return NextResponse.json(order, { status: 201 });
}
```

Signature: file path under `app/**/route.ts` + exported HTTP-verb function. Identifier is `POST /api/orders`. The handler symbol is the verb export; the `@/lib/...` import is resolved through the project's `tsconfig.json` `paths` map back to a relative path under the source root.

```json
{
  "id": "http-post-api-orders",
  "kind": "http-route",
  "identifier": "POST /api/orders",
  "handler": "app/api/orders/route.ts:POST",
  "touches": [
    "app/api/orders/route.ts",
    "lib/orders/create.ts",
    "lib/orders/repository.ts"
  ],
  "declared-at": ["app/api/orders/route.ts:4"]
}
```

### `http-route` — Next.js Pages Router

Input (`pages/api/users/[id].ts`):

```ts
import type { NextApiRequest, NextApiResponse } from "next";
import { getUser } from "../../../lib/users/get";

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  const user = await getUser(String(req.query.id));
  res.status(200).json(user);
}
```

Signature: file path under `pages/api/**` + default-exported function. Identifier resolves the `[id]` segment as `:id`; the file does not narrow on `req.method`, so the method stays `ANY`.

```json
{
  "id": "http-any-api-users-id",
  "kind": "http-route",
  "identifier": "ANY /api/users/:id",
  "handler": "pages/api/users/[id].ts:default",
  "touches": [
    "lib/users/get.ts",
    "lib/users/repository.ts",
    "pages/api/users/[id].ts"
  ],
  "declared-at": ["pages/api/users/[id].ts:4"]
}
```

### `message-pub` — BullMQ producer

Input (`src/users/events.ts`):

```ts
import { Queue } from "bullmq";

const queue = new Queue("user-events");

export async function emitUserCreated(userId: string) {
  await queue.add("user.created", { userId });
}
```

Signature: `new Queue("user-events", …)` + `queue.add("user.created", …)`. Identifier is the job name; the queue name is preserved in `id` for stability.

```json
{
  "id": "message-pub-user-events-user-created",
  "kind": "message-pub",
  "identifier": "user.created",
  "handler": "src/users/events.ts:emitUserCreated",
  "touches": ["src/users/events.ts"],
  "declared-at": ["src/users/events.ts:7"]
}
```

### `message-sub` — BullMQ worker

Input (`src/workers/email.ts`):

```ts
import { Worker } from "bullmq";
import { sendVerificationEmail } from "../notifications/email";

new Worker("user-events", async (job) => {
  if (job.name === "user.created") {
    await sendVerificationEmail(job.data.userId);
  }
});
```

Signature: `new Worker("user-events", <arrow>, …)`. The handler is an inline arrow at line 4; emit a synthetic suffix per [`handler` resolution](#handler-resolution).

```json
{
  "id": "message-sub-user-events",
  "kind": "message-sub",
  "identifier": "user-events",
  "handler": "src/workers/email.ts:bullmq-handler-1",
  "touches": [
    "src/notifications/email.ts",
    "src/workers/email.ts"
  ],
  "declared-at": ["src/workers/email.ts:4"]
}
```

### `message-sub` — NestJS `@EventPattern`

Input (`src/orders/orders.controller.ts`):

```ts
import { Controller } from "@nestjs/common";
import { EventPattern, Payload } from "@nestjs/microservices";
import { OrdersService } from "./orders.service";

@Controller()
export class OrdersController {
  constructor(private readonly orders: OrdersService) {}

  @EventPattern("order.placed")
  handleOrderPlaced(@Payload() event: { orderId: string }) {
    return this.orders.fulfil(event.orderId);
  }
}
```

Signature: `@EventPattern("order.placed")` from `@nestjs/microservices`. `@EventPattern` is the more idiomatic subscriber decorator (fire-and-forget); use `@MessagePattern` only when the source uses it.

```json
{
  "id": "message-sub-order-placed",
  "kind": "message-sub",
  "identifier": "order.placed",
  "handler": "src/orders/orders.controller.ts:OrdersController.handleOrderPlaced",
  "touches": [
    "src/orders/orders.controller.ts",
    "src/orders/orders.service.ts"
  ],
  "declared-at": ["src/orders/orders.controller.ts:9"]
}
```

### `scheduled-job` — BullMQ repeatable job

Input (`src/jobs/cleanup.ts`):

```ts
import { Queue } from "bullmq";

const queue = new Queue("maintenance");

export async function scheduleCleanup() {
  await queue.add(
    "cleanup-temp-files",
    {},
    { repeat: { pattern: "0 3 * * *" } },
  );
}
```

Signature: `queue.add(..., { repeat: … })`. Identifier is the job name; the cron expression is captured in the `id` suffix only when needed to disambiguate multiple repeats on the same job name.

```json
{
  "id": "scheduled-job-cleanup-temp-files",
  "kind": "scheduled-job",
  "identifier": "cleanup-temp-files",
  "handler": "src/jobs/cleanup.ts:scheduleCleanup",
  "touches": ["src/jobs/cleanup.ts"],
  "declared-at": ["src/jobs/cleanup.ts:7"]
}
```

### `scheduled-job` — node-cron

Input (`src/jobs/digest.ts`):

```ts
import cron from "node-cron";
import { sendDailyDigest } from "../notifications/digest";

cron.schedule("0 8 * * *", () => {
  void sendDailyDigest();
});
```

Signature: `import "node-cron"` + `cron.schedule(<cron>, <arrow>)`. Identifier is the cron expression; the arrow handler gets a synthetic suffix.

```json
{
  "id": "scheduled-job-daily-digest",
  "kind": "scheduled-job",
  "identifier": "0 8 * * *",
  "handler": "src/jobs/digest.ts:node-cron-handler-1",
  "touches": [
    "src/jobs/digest.ts",
    "src/notifications/digest.ts"
  ],
  "declared-at": ["src/jobs/digest.ts:4"]
}
```

### `ws-handler` — NestJS gateway

Input (`src/chat/chat.gateway.ts`):

```ts
import { WebSocketGateway, SubscribeMessage, MessageBody } from "@nestjs/websockets";
import { ChatService } from "./chat.service";

@WebSocketGateway({ namespace: "/chat" })
export class ChatGateway {
  constructor(private readonly chat: ChatService) {}

  @SubscribeMessage("message")
  onMessage(@MessageBody() payload: { text: string }) {
    return this.chat.broadcast(payload.text);
  }
}
```

Signature: `@WebSocketGateway` + `@SubscribeMessage("message")`. Identifier composes the gateway namespace with the event name.

```json
{
  "id": "ws-handler-chat-message",
  "kind": "ws-handler",
  "identifier": "/chat#message",
  "handler": "src/chat/chat.gateway.ts:ChatGateway.onMessage",
  "touches": [
    "src/chat/chat.gateway.ts",
    "src/chat/chat.service.ts"
  ],
  "declared-at": ["src/chat/chat.gateway.ts:8"]
}
```

### `ws-handler` — plain `ws`

Input (`src/realtime/server.ts`):

```ts
import { WebSocketServer } from "ws";
import { handlePresence } from "./presence";

const wss = new WebSocketServer({ port: 8080 });

wss.on("connection", (socket) => {
  socket.on("message", (raw) => handlePresence(socket, raw));
});
```

Signature: `new WebSocketServer(...)` + `wss.on("connection", …)`. The `connection` callback is the entry handler; emit one surface per server and let `touches[]` capture downstream message routing.

```json
{
  "id": "ws-handler-realtime-connection",
  "kind": "ws-handler",
  "identifier": "ws://:8080#connection",
  "handler": "src/realtime/server.ts:ws-handler-1",
  "touches": [
    "src/realtime/presence.ts",
    "src/realtime/server.ts"
  ],
  "declared-at": ["src/realtime/server.ts:6"]
}
```

### `cli-command` — yargs

Input (`src/cli/index.ts`):

```ts
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import { runMigrate } from "./commands/migrate";

void yargs(hideBin(process.argv))
  .command("migrate", "Run pending migrations", {}, (argv) => runMigrate(argv))
  .strict()
  .parse();
```

Signature: `import "yargs"` + `.command(<name>, <desc>, <builder>, <handler>)`. Identifier is the command name. Commander (`program.command("migrate").action(handler)`) is the equivalent signature; emit one `cli-command` surface per registered command in either case.

```json
{
  "id": "cli-command-migrate",
  "kind": "cli-command",
  "identifier": "migrate",
  "handler": "src/cli/commands/migrate.ts:runMigrate",
  "touches": [
    "src/cli/commands/migrate.ts",
    "src/cli/index.ts",
    "src/db/migrations.ts"
  ],
  "declared-at": ["src/cli/index.ts:6"]
}
```

### `external-call-out` — `fetch` / `axios` / typed SDK

Input (`src/billing/stripe.ts`):

```ts
import axios from "axios";

export async function chargeCustomer(customerId: string, amountCents: number) {
  return axios.post(
    "https://api.stripe.com/v1/charges",
    { customer: customerId, amount: amountCents },
    { headers: { Authorization: `Bearer ${process.env.STRIPE_KEY}` } },
  );
}
```

Signature: a `fetch(...)` / `axios.<verb>(...)` / typed SDK method (e.g. `stripe.charges.create(...)`) whose target is an external URL or service. Identifier is the verb + URL; when the URL is parameterized, keep the source spelling. Emit one surface per distinct external call site.

```json
{
  "id": "external-call-out-stripe-charges",
  "kind": "external-call-out",
  "identifier": "POST https://api.stripe.com/v1/charges",
  "handler": "src/billing/stripe.ts:chargeCustomer",
  "touches": ["src/billing/stripe.ts"],
  "declared-at": ["src/billing/stripe.ts:4"]
}
```

## `handler` resolution

`handler` is always `<file>:<symbol>`. `<file>` is a relative path under the source root; `<symbol>` is one of:

- **Named export.** The exported function or constant invoked as the handler. For Express, Fastify, yargs, commander, and `fetch` / `axios` / SDK call sites where the registration passes a name, resolve through the import graph: if `app.post("/users", registerUser)` and `registerUser` is imported from `./auth/register`, the handler is `src/auth/register.ts:registerUser`.
- **Class method.** For NestJS controllers, gateways, and microservice pattern handlers, the symbol is `<ClassName>.<methodName>` (e.g. `UsersController.create`). The file is the controller / gateway file.
- **Verb export.** For Next.js App Router, the symbol is the HTTP-verb export name (`GET`, `POST`, …). For Pages Router default exports, the symbol is the literal string `default`.
- **Inline arrow with line.** When the registration passes an inline arrow or anonymous function and the file otherwise has no obvious name (Express middleware mounted inline, `app.get("/x", (req, res) => …)`), use the declaring file plus `<file>:<line>` — e.g. `src/server.ts:42`. The line number is stable across reruns because the file content is the only input.
- **Inline arrow with framework suffix.** When the framework provides no name (BullMQ `new Worker(name, async (job) => …)`, plain `ws` `wss.on("connection", (socket) => …)`, node-cron `cron.schedule(expr, () => …)`), use `<file>:<framework>-handler-<n>` where `<framework>` is the lower-case framework token (`bullmq`, `ws`, `node-cron`) and `<n>` is the 1-based occurrence index within the file. This keeps reruns stable even when line numbers shift slightly.

Pick the most specific form available: prefer a named export over a line number; prefer a line number over a framework suffix only when the file genuinely names the handler somewhere reachable.

## `touches[]` resolution

`touches[]` is a static, file-level reach analysis. It is not a runtime call graph and it is not a dependency-injection trace.

Algorithm:

1. Seed the queue with the handler file.
2. Pop a file. Parse its `import` / `export` / `require` / dynamic `import()` statements.
3. For each module specifier:
   - **Relative** (`./foo`, `../bar/baz`) — resolve against the source root using the project's `tsconfig.json` `paths` map and `moduleResolution` settings; common extensions are `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, plus `index.*` for directory imports. Enqueue the resolved file.
   - **Path-aliased** (e.g. `@/lib/foo` under a `tsconfig.json` `paths` map that points back into the source root) — resolve to the file the alias targets and enqueue it.
   - **Bare module** (`express`, `@nestjs/common`, `lodash`, any `@scope/*` import that does not resolve under the source root) — stop. This is a module boundary; do not follow.
4. Continue until the queue drains.

Exclusions applied at every step:

- Skip `*.d.ts` files (type-only).
- Skip any file under a skip-root directory: `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`.
- Skip files outside the source root (the resolver may produce them via misconfigured `paths`; treat as module boundary).

Include the handler file itself in `touches[]` so the candidate algorithm sizes it.

## Anti-patterns

The brief MUST NOT emit any of the following. Validator errors here come back as `surfaces-validation-failed` or `surfaces-touches-out-of-tree`; logical errors (dead code, hallucinations) survive validation but corrupt downstream candidate sizing.

- **Dead code.** A handler function that is defined but never wired to its framework (no `app.post(...)`, no `@Get()`, no `new Worker(...)`) is not a surface. Enumerate from registration sites, not from likely-looking functions.
- **Unreachable handlers.** A registration guarded by a feature flag the source unambiguously disables in production (`if (process.env.ENABLE_LEGACY === "1") app.post(...)`) is not a surface. When the guard is ambiguous, emit the surface and let the operator decide.
- **Type-only files.** Do not list `*.d.ts` files in `touches[]`. They contribute zero production LOC and inflate the candidate size.
- **Skip-root paths.** Never reference `node_modules`, `vendor`, `target`, `.venv`, `dist`, or `build` in `touches[]` or `declared-at[]`.
- **Traversal or absolute paths.** No `..` segments, no leading `/`, no Windows drive letters. Every entry must resolve under the source root.
- **Hallucinated framework signatures.** If the source has no `import "express"`, do not emit Express `http-route` surfaces. If `package.json` does not depend on `bullmq`, do not emit BullMQ surfaces. Framework absence is dispositive.
- **Test files.** Skip `*.test.*`, `*.spec.*`, anything under `tests/` or `__tests__/`. These are not production surfaces; they validate them.
- **Inline arrow handlers without a synthetic suffix.** Every inline arrow handler MUST carry either `<file>:<line>` or `<file>:<framework>-handler-<n>`. A bare `<file>` symbol is not stable across reruns and will not be accepted.
