# TypeScript / JavaScript source survey

`/spec:plan` invokes this brief once per binding under `plan.yaml.sources.<key>` whose adapter is `typescript`. Your job: walk the read-only source tree at `$SOURCE_DIR`, identify slice-sized units of work using the framework grammar below, and return one lead block per unit. The CLI appends your blocks under `## Lead inventory` in `discovery.md`; you never write `discovery.md` directly.

JavaScript sources (`.js`, `.mjs`, `.cjs`, `.jsx`) fold into this brief: the framework idioms are the same. Detect the file extension purely to widen the import-graph walk; the brief content does not branch on it.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound source root (the `path:` from `plan.yaml.sources.<key>`). Walk this tree; resolve `tsconfig.json` `paths` mappings relative to it.
- **Source key** — kebab-case identifier passed in via the runner (the `<key>` from `plan.yaml.sources.<key>`). The CLI stamps each lead's `source` from it; this brief does not emit it.

The bound directory is the only filesystem grant; `$PROJECT_DIR` is unreachable. Treat the tree as read-only — no writes back into `$SOURCE_DIR`.

## Output: lead blocks

Emit one fenced block per identified unit, in the shape the CLI appends under `## Lead inventory`:

```markdown
### <lead>

- lead: <lead>
- synopsis: <reconciliation-grade headline>
- topics: [<optional-kebab-slugs>]
```

`lead` is kebab-case, derived from the dominant surface identifier or handler path (e.g. `POST /users` → `user-registration`, `email.send` queue → `email-send`). It is the stable handle re-survey writes against (keyed by `(source, lead)`). After the CLI stamps `source`, the block validates against `schemas/discovery/lead.schema.json` (kebab-case `lead`, scalar `source`, content-bearing `synopsis`, optional kebab-case `topics`). One block per lead.

`synopsis` SHOULD name the handler's surface (route + method, queue + job, topic) and its salient behaviour/constraint so a same-slug lead from another source can be matched or distinguished on content, not just the shared slug. Prefer one line; it MAY run to a few lines when one is too thin. It stays plan-time headline material — slice-time behaviour belongs in `typescript.extract` claims, not here.

`topics` (optional) is an inline list of kebab-case domain slugs drawn from the surface (e.g. `[identity, http-route]`); author them only when the code clearly supports the classification, omit the bullet otherwise. They are extra grouping context for `propose` and the join key for the decision-contradiction warning — never a grouping the CLI computes.

## Internal staging

Survey grammar is **adapter-internal** — there is no `surfaces.json` sibling artifact and no published schema for the intermediate shape. You MAY stage a working JSON document under `$SCRATCH_DIR/staged.json` to keep the framework walk auditable during the run; treat its shape as adapter-private (see [Working JSON shape](#working-json-shape)). Only lead blocks are visible to downstream synthesis; the staged JSON is retained in the scratch lane purely as an audit trail and is never read by any later phase.

## Framework grammar

Each row describes the import + call-site signature that qualifies one surface, the surface `kind` token, and how to compose the lead `id`.

- **Express** — `import` of `express` whose default export (or `Router`) has `.get` / `.post` / `.put` / `.patch` / `.delete` / `.all` / `.use` (mount only when it attaches a handler) called with a path string. Each route registration is one `http-route` surface.
- **Fastify** — `import` of `fastify`. `app.get` / `.post` / `.put` / `.patch` / `.delete` / `.route({ method, url, handler })` with a path string is `http-route`.
- **NestJS** — class decorated with `@Controller(...)` from `@nestjs/common`; each method decorated with `@Get` / `@Post` / `@Put` / `@Patch` / `@Delete` / `@Options` / `@Head` is `http-route`. Methods with `@MessagePattern` / `@EventPattern` from `@nestjs/microservices` are `message-sub`. Classes with `@WebSocketGateway` contribute one `ws-handler` per `@SubscribeMessage` method.
- **Next.js App Router** — files matching `app/**/route.{ts,js,tsx,jsx}` exporting an HTTP-verb function (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD`). The route path derives from the file path relative to `app/` with `[seg]` → `:seg` and `(group)` segments stripped. One `http-route` per verb.
- **Next.js Pages Router** — files under `pages/api/**.{ts,js}` whose default export is a request handler. Each file is one `http-route` (method defaults to `ANY` unless the handler narrows on `req.method`).
- **BullMQ** — `import` of `bullmq`. `new Queue(name, …)` + `queue.add(jobName, …)` is `message-pub` (one surface per `jobName`); the same call with `{ repeat: … }` additionally emits a `scheduled-job` keyed on the repeat expression. `new Worker(name, handler, …)` is `message-sub` per queue.
- **node-cron** — `cron.schedule(<cron>, <handler>)` is `scheduled-job`. Identifier is the cron expression.
- **`ws`** — `new WebSocketServer(...)` + `wss.on("connection", …)` is one `ws-handler` per server.
- **yargs / commander** — `.command(<name>, <desc>, <builder>, <handler>)` or `program.command(...).action(handler)` is one `cli-command` per registered command.
- **`fetch` / `axios` / typed SDK** — outbound HTTP / SDK call sites (`axios.<verb>(url, ...)`, `fetch(url, ...)`, `stripe.charges.create(...)`) whose target is an external URL or service is `external-call-out`. One surface per distinct call site.

Out of scope for v1: tRPC, GraphQL resolvers, gRPC services, AWS Lambda handlers, Cloudflare Workers. If the source uses one of those exclusively, return zero leads and let the operator review.

## Lead algorithm

1. **Walk the tree.** Survey framework call sites per the grammar above. Skip `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`, `*.d.ts`, and test directories (`test`, `tests`, `__tests__`, `spec`, `specs`, `*.test.*`, `*.spec.*`).
2. **Size check.** Compute the union-of-`touches` LOC across every identified surface. If the union is `< 1000` production LOC, emit **one source-level lead** named after the source key (or its dominant subject) covering every surface, and stop for that source.
3. **Surface leads.** Otherwise, treat each surface as the default lead.
4. **Minimal same-source clustering.** Merge surface leads only when ALL of these hold:
    - One signal fires: shared `touches` overlap ≥ 50% (computed as `|intersection| / |smaller set|`), **or** shared `handler` / call site, **or** an explicit grouping the operator already wrote in `discovery.md`'s `## Lead inventory`.
    - The merged LOC stays `< 1000`. If merging pushes the lead over, do not merge.
5. **`too-large` after clustering.** A lead whose LOC stays `≥ 1000` is still emitted; flag the staged JSON entry with an internal `unresolved: true` marker so `/spec:plan`'s `propose` sub-step can call it out. Survey exits 0 either way — `propose` is the gate, not `survey`.

Production LOC counts non-blank, non-comment-only lines in source files, excluding `*.d.ts`, generated code (`*.gen.*`, `*.generated.*`, `*.pb.*`, `*_pb.*`), tests, and the skip-root directories above.

## Path rules

Every internal staged reference to a file under `$SOURCE_DIR` MUST be a relative path:

- No leading `/`, no Windows drive letter (`C:\…`).
- No `..` segments.
- Resolves to a file under `$SOURCE_DIR`.
- Not under a skip-root (`node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`).

A symlink inside `$SOURCE_DIR` pointing outside the bound root is denied at canonicalization; the host runner returns `source-survey-path-denied` and the slice stays `refining` per workflow §Extraction reliability.

## Working JSON shape

For internal staging only (not an artifact). Top-level: `{ version: 1, source, language, surfaces[] }`. Each surface: `{ id, kind, identifier, handler, touches[], declared-at[] }`. `kind` is one of `http-route | message-pub | message-sub | ws-handler | scheduled-job | cli-command | ui-route | external-call-out`. `handler` is `<file>:<symbol>` (named export, `<ClassName>.<method>`, verb export, `<file>:<line>` for inline arrows, `<file>:<framework>-handler-<n>` when the framework provides no name). `touches[]` is a static, file-level reach analysis: import-graph walk from the handler file through relative + `tsconfig.json` `paths`-aliased imports, stopping at bare module specifiers; include the handler file itself. `declared-at[]` carries the registration site (`<file>` or `<file>:<line>`).

You never publish this shape. The lead algorithm reads from it; only the lead blocks reach `discovery.md`.

## Worked example

Tiny Express service rooted at `$SOURCE_DIR`:

```
src/
├── server.ts          # Express setup; app.post("/users", registerUser)
├── users/
│   ├── register.ts    # registerUser handler; email validation
│   └── repository.ts  # insertUser
```

Framework signatures fired:

- `import express from "express"` + `app.post("/users", registerUser)` in `src/server.ts` → `http-route` `POST /users`, handler resolves through the named import to `src/users/register.ts:registerUser`, `touches` is `[src/server.ts, src/users/register.ts, src/users/repository.ts]`.

Union LOC stays well below 1000 → Decision 2 (size check) emits one source-level lead covering the single surface. Resulting lead block:

```markdown
### user-registration

- lead: user-registration
- synopsis: Registration endpoint accepting email + password with RFC-5322 validation.
```

When a larger source decomposes into multiple leads, emit one block per surface (or per merged cluster) in source order (alphabetical by handler path within the source) so re-survey produces stable diffs.

## Anti-patterns

- **Dead code.** A handler defined but never wired to a framework (no `app.post(...)`, no `@Get()`, no `new Worker(...)`) is not a surface. Survey from registration sites, not from likely-looking functions.
- **Feature-flag-disabled handlers.** A registration unambiguously disabled in production (`if (process.env.ENABLE_LEGACY === "1") app.post(...)`) is not a surface. When the guard is ambiguous, emit it and let the operator decide at Gate 1.
- **Hallucinated framework signatures.** If `package.json` does not depend on `bullmq`, do not emit BullMQ surfaces. Framework absence is dispositive.
- **Test files.** Skip `*.test.*`, `*.spec.*`, and anything under `tests/` or `__tests__/`. Tests validate production surfaces, they are not production surfaces.
- **Type-only `.d.ts` files in `touches`.** They contribute zero production LOC and inflate lead sizing.
- **Cross-source coalescing.** This brief only sees one source's tree. Cross-source merges happen later in `/spec:plan`'s `propose` sub-step — see [From sources to slices](../references/spec-runtime/reconciliation.md#plan-time-leads-become-slices) for how leads reconcile into slices.
- **Writing `discovery.md` or `plan.yaml`.** Only lead blocks. The CLI owns every lifecycle file.

## Failure modes

| Condition                                              | Action                                                                                                                          |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `$SOURCE_DIR` empty / no recognised framework imports  | Return zero leads. Operator reviews in `discovery.md`.                                                                     |
| Read denied outside `$SOURCE_DIR`                      | Host runner returns `source-survey-path-denied`; the slice stays `refining`.                                                 |
| Internal staged JSON malformed                         | Repair within the run; the lead algorithm is the final consumer, not an external schema check.                             |
| Surface uses an out-of-scope framework (tRPC, gRPC, …) | Skip it. Return whatever in-scope leads the tree has; document the gap in the synopsis of the relevant source-level lead. |
