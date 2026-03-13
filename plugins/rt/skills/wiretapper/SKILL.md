---
name: wiretapper
description: Add wiretap code to a cloned legacy TypeScript repo to capture request/response and side-effect data as fixture JSON; detect patterns, generate adapters, wire entrypoint, verify compile.
argument-hint: [legacy-dir] [app-name?]
allowed-tools: Read, Write, StrReplace, Shell, Grep
---

# Wiretap Skill

## Overview

Analyze a cloned legacy TypeScript/Node.js repository, detect which of eight patterns (A–H) apply (Fastify, Express, NestJS, Kafka consumer/producer, HttpClient, TypeORM), and generate a `src/wiretap/` folder with core session/singleton and only the relevant adapters. Then patch the app entrypoint to wire up wiretap (conditional on `WIRETAP_ENABLED=true`) and run the project build to verify the code compiles. Output at runtime is `{app-name}.wiretap.json` when wiretap is enabled.

This skill operates **autonomously**: it never prompts for input. Invalid input or build failure results in a clear error and step failure.

## Derived Arguments

```text
$LEGACY    = $ARGUMENTS[0]                    # Path to cloned legacy repo (required)
$APP_NAME  = $ARGUMENTS[1] OR from package.json "name" OR basename($LEGACY)   # App name for wiretap file
```

If `$LEGACY` is missing or not a directory, fail with: `"Error: legacy-dir is required and must be an existing directory."`

## Process

### Step 1: Validate

1. **Path**: Ensure `$LEGACY` exists and is a directory.
2. **Node project**: Ensure `$LEGACY/package.json` exists and is valid JSON.
3. If validation fails, exit with a clear error message.

### Step 2: Detect Patterns

Read `$LEGACY/package.json` (dependencies and devDependencies) and scan source under `$LEGACY` (e.g. `src/`, `lib/`, or root `*.ts`/`*.js`) for the signals below. Set of patterns is the union of all detected.

| Pattern | Description | Detection signal |
|---------|--------------|------------------|
| **A** | HTTP entry — Fastify (standalone) | `fastify` in deps **and** no `@nestjs/core` |
| **B** | HTTP entry — Express | `express` in deps **and** no `@nestjs/core` |
| **C** | HTTP entry — NestJS | `@nestjs/core` in deps |
| **D** | Kafka consumer entry | Source contains `KafkaConsumer` and `onMessage` (or equivalent consumer callback pattern) |
| **E** | Outbound HTTP — HttpClient | Source contains `HttpClient` or `httpClient.get` (e.g. from `at-realtime-common` or similar) |
| **F** | TypeORM | `typeorm` in deps **and** source contains `DataSource` or `@InjectRepository` or `createQueryRunner` |
| **G** | Kafka publisher — NestJS ClientKafka | `@nestjs/microservices` in deps **and** source contains `ClientKafka` |
| **H** | Kafka publisher — standalone | Source contains `KafkaProducer` or kafkajs producer usage, **and** no NestJS (no `@nestjs/core`) |

- **Conservative**: If in doubt, do **not** add a pattern; generate only adapters for patterns that are clearly present.
- **Mutually exclusive for HTTP entry**: At most one of A, B, C (NestJS takes precedence over Fastify/Express if present).

### Step 3: Generate Core and Adapters

Create `$LEGACY/src/wiretap/` and generate only the files below. Use the **exact** adapter code from [references/adapters/](references/adapters/) for each detected pattern; do not invent alternate implementations.

**Always generated:**

- `$LEGACY/src/wiretap/session.ts` — `WiretapSession`, `WiretapEntry`, `WiretapHttpCall`, `WiretapDbQuery`, `WiretapKafkaPublish`, `extractError`. AsyncLocalStorage-based session; `toEntry(output, statusCode)`. (See [references/design.md](references/design.md) for session/wiretap core.)
- `$LEGACY/src/wiretap/wiretap.ts` — `Wiretap` singleton: `AsyncLocalStorage<WiretapSession>`, `init(appName)`, `getInstance()`, `getCurrentSession()`, `enterSession()`, `runWithSession()`, `flush(handler, entry)` writing to `{appName}.wiretap.json`.

**Generated only when the corresponding pattern is detected** (full code in `references/adapters/<name>.md`):

| Pattern | File | Reference |
|---------|------|-----------|
| A | `adapters/fastify-hooks.ts` | [fastify-hooks.md](references/adapters/fastify-hooks.md) |
| B | `adapters/express-middleware.ts` | [express-middleware.md](references/adapters/express-middleware.md) |
| C | `adapters/nest-interceptor.ts` | [nest-interceptor.md](references/adapters/nest-interceptor.md) |
| D | `adapters/kafka-consumer-wrapper.ts` | [kafka-consumer-wrapper.md](references/adapters/kafka-consumer-wrapper.md) |
| E | `adapters/http-client-wrapper.ts` | [http-client-wrapper.md](references/adapters/http-client-wrapper.md) |
| F | `adapters/typeorm-wrapper.ts` | [typeorm-wrapper.md](references/adapters/typeorm-wrapper.md) |
| G | `adapters/kafka-nestclient-wrapper.ts` | [kafka-nestclient-wrapper.md](references/adapters/kafka-nestclient-wrapper.md) |
| H | `adapters/kafka-producer-wrapper.ts` | [kafka-producer-wrapper.md](references/adapters/kafka-producer-wrapper.md) (dual-API: `.publish` and `.send`) |

**Guardrails (from design):**

- All capture code in adapters (E, F, G, H) must wrap recording in try/catch so wiretap never breaks the app.
- Skip paths `/status` and `/swap` in HTTP entry adapters (A, B, C).
- Kafka consumer: use `runWithSession()` not `enterWith()`; flush in the commit callback.

### Step 4: Wire Up the Start

1. **Locate entrypoint**: Prefer `src/main.ts`, `src/start.ts`, `main.ts`, `start.ts`, or the file referenced by `package.json` `main`/`scripts.start`.
2. **Insert wiretap bootstrap** so it runs only when `process.env.WIRETAP_ENABLED === "true"`:
   - Call `Wiretap.init(appName)` (use `$APP_NAME`).
   - For each detected pattern, call the corresponding register/wrap function with the appropriate app instance (e.g. Fastify instance, Express app, NestJS `app.get(DataSource)`, etc.). Match composition order to the design (e.g. HTTP entry last so session is set before any outbound/DB/Kafka wrappers run).
   - For NestJS + Kafka: register interceptor and TypeORM/Kafka wrappers **before** `startAllMicroservices()`; wrap Kafka consumer before `startAllMicroservices()`.
3. If the entrypoint cannot be determined or patched safely (e.g. non-standard layout), fail with a clear message describing what was tried.

### Step 5: Verify Compile

1. From `$LEGACY`, run the project build (e.g. `npm run build` or `npx tsc --noEmit`). Use the script the project defines; if both exist, prefer `npm run build`.
2. If the build fails, report the compiler errors and **fail the step**. Do not leave the repo in a broken state without failing.

### Step 6 (Optional): Integration Doc

Optionally add `$LEGACY/src/wiretap/README.md` documenting that wiretap is enabled with `WIRETAP_ENABLED=true` and listing which adapters were registered.

## Reference Documentation

- **[references/design.md](references/design.md)** — Detection table, file structure, and constraints. The authoritative source for generated code structure.
- **[references/adapters/](references/adapters/)** — Full TypeScript code for each adapter; generate code that matches these references exactly.

## Error Handling

| Issue | Cause | Resolution |
|-------|--------|------------|
| Invalid or missing legacy path | Bad argument or path not a directory | Fail with "Error: legacy-dir is required and must be an existing directory." (or similar) |
| No package.json | Not a Node project | Fail with clear message; do not generate. |
| Entrypoint not found | Unusual layout | Fail with message listing paths checked. |
| Build fails after wiring | Syntax/import errors in generated or patched code | Report compiler output and fail the step. |
| Wiretap capture throws | Bug in generated adapter | All adapters must wrap capture in try/catch (design guardrail). |

## Verification Checklist

- [ ] `$LEGACY/src/wiretap/session.ts` and `wiretap.ts` exist.
- [ ] Only adapter files for detected patterns exist under `src/wiretap/adapters/`.
- [ ] App entrypoint contains conditional wiretap init and adapter registration (when `WIRETAP_ENABLED=true`).
- [ ] `npm run build` (or equivalent) succeeds from `$LEGACY`.

## Important Notes

- **No cron/background**: Do not capture scheduled or long-running loops; only request-scoped and message-scoped handlers.
- **Handler keys**: HTTP uses `METHOD path` (e.g. `GET /api/v1/...`); Kafka uses `topic:TopicName`.
- **Safety**: Wiretap must never break the application; all recording in adapters is try/catch wrapped.
- **Single output file**: `{app-name}.wiretap.json` in the process cwd when the app runs; no file locking required.
