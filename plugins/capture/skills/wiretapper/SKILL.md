---
name: capture-wiretapper
description: Add wiretap code to a cloned legacy TypeScript repo to capture request/response and side-effect data as replay-ready JSON; detect patterns, generate adapters, wire entrypoint, verify compile. Use when a legacy TypeScript service must be wiretapped for replay-ready captures before migration; not for consuming captures in the Specify workflow (bind `captures` at plan time and run Omnia `build/replay.md`) or for non-TypeScript sources.
argument-hint: <legacy-dir> [app-name]
---

# Wiretapper Skill

Analyze a cloned legacy TypeScript/Node.js repository, detect which of eight patterns (A–H) apply (Fastify, Express, NestJS, Kafka consumer/producer, HttpClient, TypeORM), and generate a `src/wiretap/` folder with core session/singleton and only the relevant adapters. Then patch the app entrypoint to wire up wiretap (conditional on `WIRETAP_ENABLED=true`) and run the project build to verify the code compiles. Output at runtime is `{app-name}.wiretap.json` when wiretap is enabled.

This skill operates **autonomously**: it never prompts for input. Invalid input or build failure results in a clear error and step failure.

If `<legacy-dir>` is missing or not a directory, fail with: `"Error: legacy-dir is required and must be an existing directory."` `<app-name>` defaults to `package.json` `"name"` or `basename(<legacy-dir>)` when omitted; it labels the runtime wiretap file `{app-name}.wiretap.json`.

## Critical Path

### 0. Bootstrap the legacy tree (if remote)

`/capture:wiretapper` only operates on a local directory. When the legacy source lives on a remote, materialise it first into a fresh temporary directory and pass the resulting local path as `$LEGACY_DIR`:

```bash
git clone "$url" "$dest"
```

### 1. Validate

1. **Path**: Ensure `$LEGACY_DIR` exists and is a directory.
2. **Node project**: Ensure `$LEGACY_DIR/package.json` exists and is valid JSON.
3. If validation fails, exit with a clear error message.

### 2. Detect Patterns

Read `$LEGACY_DIR/package.json` (dependencies and devDependencies) and scan source under `$LEGACY_DIR` (e.g. `src/`, `lib/`, or root `*.ts`/`*.js`). The eight-pattern detection table (A–H), the per-pattern signals, and the HTTP-entry mutual-exclusion rule (NestJS > Fastify/Express) live in [references/design.md](references/design.md) §Detection. Be conservative — when in doubt, do not add a pattern.

### 3. Generate Core and Adapters

Create `$LEGACY_DIR/src/wiretap/` and generate only the files below. Use the **exact** adapter code from [references/adapters/](references/adapters/) for each detected pattern; do not invent alternate implementations.

**Always generated:**

- `$LEGACY_DIR/src/wiretap/session.ts` — `WiretapSession`, `WiretapEntry`, `WiretapHttpCall`, `WiretapDbQuery`, `WiretapKafkaPublish`, `extractError`. AsyncLocalStorage-based session; `toEntry(output, statusCode)`. (See [references/design.md](references/design.md) for session/wiretap core.)
- `$LEGACY_DIR/src/wiretap/wiretap.ts` — `Wiretap` singleton: `AsyncLocalStorage<WiretapSession>`, `init(appName)`, `getInstance()`, `getCurrentSession()`, `enterSession()`, `runWithSession()`, `flush(handler, entry)` writing to `{appName}.wiretap.json`.

**Generated only when the corresponding pattern is detected.** The pattern → adapter file → reference document mapping (A–H) lives in [references/design.md](references/design.md) §Generated Layout. Generate one adapter file under `$LEGACY_DIR/src/wiretap/adapters/` per detected pattern, copying verbatim from the listed reference.

### 4. Wire Up the Start

1. **Locate entrypoint**: Prefer `src/main.ts`, `src/start.ts`, `main.ts`, `start.ts`, or the file referenced by `package.json` `main`/`scripts.start`.
2. **Insert wiretap bootstrap** so it runs only when `process.env.WIRETAP_ENABLED === "true"`:
   - Call `Wiretap.init(appName)` (use `$APP_NAME`).
   - For each detected pattern, call the corresponding register/wrap function with the appropriate app instance (e.g. Fastify instance, Express app, NestJS `app.get(DataSource)`, etc.). Match composition order to the design (e.g. HTTP entry last so session is set before any outbound/DB/Kafka wrappers run).
   - For NestJS + Kafka: register interceptor and TypeORM/Kafka wrappers **before** `startAllMicroservices()`; wrap Kafka consumer before `startAllMicroservices()`.
3. If the entrypoint cannot be determined or patched safely (e.g. non-standard layout), fail with a clear message describing what was tried.

### 5. Verify Compile

1. From `$LEGACY_DIR`, run the project build (e.g. `npm run build` or `npx tsc --noEmit`). Use the script the project defines; if both exist, prefer `npm run build`.
2. If the build fails, report the compiler errors and **fail the step**. Do not leave the repo in a broken state without failing.

## Reference Documentation
- **[references/design.md](references/design.md)** — Detection table, file structure, and constraints. The authoritative source for generated code structure.
- **[references/adapters/](references/adapters/)** — Full TypeScript code for each adapter; generate code that matches these references exactly.
- **[Capture output format](https://github.com/augentic/specify-adapters/blob/main/sources/captures/prose/references/capture-format.md)** — directory layout wiretap output must satisfy for `captures` source binding (`tests/data/replays/<handler>/<scenario>.json`).

## Verification Checklist

- [ ] `$LEGACY_DIR/src/wiretap/session.ts` and `wiretap.ts` exist.
- [ ] Only adapter files for detected patterns exist under `src/wiretap/adapters/`.
- [ ] App entrypoint contains conditional wiretap init and adapter registration (when `WIRETAP_ENABLED=true`).
- [ ] `npm run build` (or equivalent) succeeds from `$LEGACY_DIR`.

## Guardrails

- **No cron/background**: Do not capture scheduled or long-running loops; only request-scoped and message-scoped handlers.
- **Handler keys**: HTTP uses `METHOD path` (e.g. `GET /api/v1/...`); Kafka uses `topic:TopicName`.
- **Safety**: Wiretap must never break the application; all recording in adapters is try/catch wrapped.
- **Single output file**: `{app-name}.wiretap.json` in the process cwd when the app runs; no file locking required.
- **Skip paths `/status` and `/swap` in HTTP entry adapters (A, B, C).**
- **Kafka consumer**: use `runWithSession()` not `enterWith()`; flush in the commit callback.
