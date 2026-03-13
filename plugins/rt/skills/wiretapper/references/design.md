# Wiretap Design Reference

This document summarizes detection and generated structure. Full adapter code is in [adapters/](adapters/). Generate TypeScript that matches these references exactly.

## Detection (Patterns A–H)

| Pattern | Detection |
|---------|-----------|
| A: Fastify HTTP | `fastify` in deps, no `@nestjs/core` |
| B: Express HTTP | `express` in deps, no `@nestjs/core` |
| C: NestJS HTTP | `@nestjs/core` in deps |
| D: Kafka consumer | `KafkaConsumer` / `onMessage` in source |
| E: HttpClient | `HttpClient` or `httpClient.get` in source |
| F: TypeORM | `typeorm` in deps + `DataSource` or `@InjectRepository` in source |
| G: Nest Kafka publisher | `@nestjs/microservices` + `ClientKafka` in source |
| H: Standalone Kafka publisher | `KafkaProducer` or kafkajs in source, no NestJS |

At most one of A, B, C. Use `runWithSession()` for Kafka consumer (D); wrap `createQueryRunner()` only for TypeORM (F); pass `...rest` in TypeORM `query()` wrapper.

## Generated Layout

```
src/wiretap/
├── session.ts      # WiretapSession, WiretapEntry, interfaces, extractError
├── wiretap.ts      # Wiretap singleton (AsyncLocalStorage + flush to {appName}.wiretap.json)
└── adapters/
    ├── fastify-hooks.ts            # A — references/adapters/fastify-hooks.md
    ├── express-middleware.ts        # B — references/adapters/express-middleware.md
    ├── nest-interceptor.ts         # C — references/adapters/nest-interceptor.md
    ├── kafka-consumer-wrapper.ts   # D — references/adapters/kafka-consumer-wrapper.md
    ├── http-client-wrapper.ts      # E — references/adapters/http-client-wrapper.md
    ├── typeorm-wrapper.ts          # F — references/adapters/typeorm-wrapper.md
    ├── kafka-nestclient-wrapper.ts # G — references/adapters/kafka-nestclient-wrapper.md
    └── kafka-producer-wrapper.ts   # H — references/adapters/kafka-producer-wrapper.md (dual-API)
```

Only generate adapters for detected patterns. Skip `/status` and `/swap` in HTTP adapters. All capture in E/F/G/H must be inside try/catch. **Full implementation for each adapter** is in the skill’s `references/adapters/` directory (e.g. `fastify-hooks.md`, `kafka-producer-wrapper.md`).

## Pattern H: Kafka producer (dual-API)

The kafka-producer-wrapper must support both APIs so it works with at-realtime-common and at-connector-common/kafkajs:

- **at-realtime-common**: `KafkaProducer` has `.publish(topic, incomingMsg)` with `incomingMsg = { key?, value, ... }`. Wrap `publish` and capture topic, `incomingMsg.value` (or incomingMsg), and `incomingMsg.key`.
- **at-connector-common / kafkajs**: producer has `.send(topic, key, message, ...rest)`. Wrap `send` as in the main design.

If producer is null/undefined or has neither method, return without wrapping (no-op). Never call `.bind()` on undefined. Full code: [adapters/kafka-producer-wrapper.md](adapters/kafka-producer-wrapper.md).
