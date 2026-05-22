---
id: enumerate
description: Identify slice-sized candidates in a TypeScript / JavaScript source tree bound at $SOURCE_DIR and emit one `## Candidate inventory` block per candidate.
authority: behaviour
---

# TypeScript / JavaScript source enumeration

`/spec:plan` invokes this brief once per binding under `plan.yaml.sources.<key>` whose adapter is `code-typescript`. Your job: walk the read-only source tree at `$SOURCE_DIR`, identify slice-sized units of work using the framework grammar below, and return one candidate block per unit. The CLI appends your blocks under `## Candidate inventory` in `discovery.md`; you never write `discovery.md` directly.

JavaScript sources (`.js`, `.mjs`, `.cjs`, `.jsx`) fold into this brief: the framework idioms are the same. Detect the file extension purely to widen the import-graph walk; the brief content does not branch on it.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound source root (the `path:` from `plan.yaml.sources.<key>`). Walk this tree; resolve `tsconfig.json` `paths` mappings relative to it.
- **Source key** — kebab-case identifier passed in via the runner (the `<key>` from `plan.yaml.sources.<key>`). Echoed into every candidate's `sources:` list.

The bound directory is the only filesystem grant; `$PROJECT_DIR` is unreachable. Treat the tree as read-only — no writes back into `$SOURCE_DIR`.

## Output: candidate blocks

Emit one fenced block per identified unit, in the shape the CLI appends under `## Candidate inventory`:

```markdown
### <id>

- id: <id>
- sources: [<source-key>]
- summary: <one-line description>
```

`id` is kebab-case, derived from the dominant surface identifier or handler path (e.g. `POST /users` → `user-registration`, `email.send` queue → `email-send`). It is the stable handle re-enumeration writes against. The block validates against `schemas/discovery/candidate.schema.json` (kebab-case `id`, non-empty `sources[]`, one-line `summary`). One block per candidate; no `tentative:` field at enumerate time (set later by `/spec:plan`'s `propose` sub-step).

## Internal staging

Enumeration grammar is **adapter-internal** — there is no `surfaces.json` sibling artifact in 2.0 and no published schema for the intermediate shape. You MAY stage a working JSON document under `$SCRATCH_DIR/staged.json` to keep the framework walk auditable during the run; treat its shape as adapter-private (see [Working JSON shape](#working-json-shape)). Only candidate blocks are visible to downstream synthesis; the staged JSON does not survive the run.

## Framework grammar

Each row describes the import + call-site signature that qualifies one surface, the surface `kind` token, and how to compose the candidate `id`.

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

Out of scope for v1: tRPC, GraphQL resolvers, gRPC services, AWS Lambda handlers, Cloudflare Workers. If the source uses one of those exclusively, return zero candidates and let the operator review.

## Candidate algorithm

1. **Walk the tree.** Enumerate framework call sites per the grammar above. Skip `node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`, `*.d.ts`, and test directories (`test`, `tests`, `__tests__`, `spec`, `specs`, `*.test.*`, `*.spec.*`).
2. **Size check.** Compute the union-of-`touches` LOC across every identified surface. If the union is `< 1000` production LOC, emit **one source-level candidate** named after the source key (or its dominant subject) covering every surface, and stop for that source.
3. **Surface candidates.** Otherwise, treat each surface as the default candidate.
4. **Minimal same-source clustering.** Merge surface candidates only when ALL of these hold:
    - One signal fires: shared `touches` overlap ≥ 50% (computed as `|intersection| / |smaller set|`), **or** shared `handler` / call site, **or** an explicit grouping the operator already wrote in `discovery.md`'s `## Candidate inventory`.
    - The merged LOC stays `< 1000`. If merging pushes the candidate over, do not merge.
5. **`too-large` after clustering.** A candidate whose LOC stays `≥ 1000` is still emitted; flag the staged JSON entry with an internal `unresolved: true` marker so `/spec:plan`'s `propose` sub-step can call it out. Enumerate exits 0 either way — `propose` is the gate, not `enumerate`.

Production LOC counts non-blank, non-comment-only lines in source files, excluding `*.d.ts`, generated code (`*.gen.*`, `*.generated.*`, `*.pb.*`, `*_pb.*`), tests, and the skip-root directories above.

## Path rules

Every internal staged reference to a file under `$SOURCE_DIR` MUST be a relative path:

- No leading `/`, no Windows drive letter (`C:\…`).
- No `..` segments.
- Resolves to a file under `$SOURCE_DIR`.
- Not under a skip-root (`node_modules`, `vendor`, `target`, `.venv`, `dist`, `build`).

A symlink inside `$SOURCE_DIR` pointing outside the bound root is denied at canonicalization; the host runner returns `source-enumerate-path-denied` and the slice stays `refining` per RFC-25 §Extraction reliability.

## Working JSON shape

For internal staging only (not an artifact). Top-level: `{ version: 1, source-key, language, surfaces[] }`. Each surface: `{ id, kind, identifier, handler, touches[], declared-at[] }`. `kind` is one of `http-route | message-pub | message-sub | ws-handler | scheduled-job | cli-command | ui-route | external-call-out`. `handler` is `<file>:<symbol>` (named export, `<ClassName>.<method>`, verb export, `<file>:<line>` for inline arrows, `<file>:<framework>-handler-<n>` when the framework provides no name). `touches[]` is a static, file-level reach analysis: import-graph walk from the handler file through relative + `tsconfig.json` `paths`-aliased imports, stopping at bare module specifiers; include the handler file itself. `declared-at[]` carries the registration site (`<file>` or `<file>:<line>`).

You never publish this shape. The candidate algorithm reads from it; only the candidate blocks reach `discovery.md`.

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

Union LOC stays well below 1000 → Decision 2 (size check) emits one source-level candidate covering the single surface. Resulting candidate block:

```markdown
### user-registration

- id: user-registration
- sources: [legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

When a larger source decomposes into multiple candidates, emit one block per surface (or per merged cluster) in source order (alphabetical by handler path within the source) so re-enumeration produces stable diffs.

## Anti-patterns

- **Dead code.** A handler defined but never wired to a framework (no `app.post(...)`, no `@Get()`, no `new Worker(...)`) is not a surface. Enumerate from registration sites, not from likely-looking functions.
- **Feature-flag-disabled handlers.** A registration unambiguously disabled in production (`if (process.env.ENABLE_LEGACY === "1") app.post(...)`) is not a surface. When the guard is ambiguous, emit it and let the operator decide at Gate 1.
- **Hallucinated framework signatures.** If `package.json` does not depend on `bullmq`, do not emit BullMQ surfaces. Framework absence is dispositive.
- **Test files.** Skip `*.test.*`, `*.spec.*`, and anything under `tests/` or `__tests__/`. Tests validate production surfaces, they are not production surfaces.
- **Type-only `.d.ts` files in `touches`.** They contribute zero production LOC and inflate candidate sizing.
- **Cross-source coalescing.** This brief only sees one source's tree. Cross-source merges happen later in `/spec:plan`'s `propose` sub-step.
- **Writing `discovery.md` or `plan.yaml`.** Only candidate blocks. The CLI owns every lifecycle file.

## Failure modes

| Condition                                              | Action                                                                                                                          |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `$SOURCE_DIR` empty / no recognised framework imports  | Return zero candidates. Operator reviews in `discovery.md`.                                                                     |
| Read denied outside `$SOURCE_DIR`                      | Host runner returns `source-enumerate-path-denied`; the slice stays `refining`.                                                 |
| Internal staged JSON malformed                         | Repair within the run; the candidate algorithm is the final consumer, not an external schema check.                             |
| Surface uses an out-of-scope framework (tRPC, gRPC, …) | Skip it. Return whatever in-scope candidates the tree has; document the gap in the summary of the relevant source-level candidate. |
