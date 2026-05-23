# Pattern D: kafka-consumer-wrapper.ts

Generate when **Kafka consumer** is detected. Use `runWithSession()` (not `enterWith()`). Flush in the commit callback. If the listener returns a Promise, catch rejections and flush failure.

## Implementation

```typescript
// src/wiretap/adapters/kafka-consumer-wrapper.ts

import { Wiretap } from "../wiretap";
import { WiretapSession } from "../session";

type KafkaConsumerLike = {
  onMessage: (
    listener: (event: unknown, next: () => void, ...rest: unknown[]) => void,
  ) => void;
};

export function wrapKafkaConsumer(
  consumer: KafkaConsumerLike,
  wiretap: Wiretap,
): void {
  const originalOnMessage = consumer.onMessage.bind(consumer);
  consumer.onMessage = (
    listener: (event: unknown, next: () => void, ...rest: unknown[]) => void,
  ) => {
    originalOnMessage((event: unknown, next: () => void, ...rest: unknown[]) => {
      const session = new WiretapSession();
      const ev = event as { topic?: string; value?: unknown };
      session.setHandler(`topic:${ev.topic || "unknown"}`);

      let parsed: unknown = ev.value;
      try {
        parsed =
          typeof ev.value === "string"
            ? JSON.parse(ev.value)
            : ev.value;
      } catch {
        /* keep as-is */
      }
      session.captureInput(parsed as Record<string, unknown>);

      let flushed = false;
      const flushFailure = (error: unknown) => {
        if (flushed) return;
        flushed = true;
        const message = error instanceof Error ? error.message : String(error);
        wiretap
          .flush(session.handler, session.toEntry({ error: message }, 500))
          .catch(() => {});
      };

      wiretap.runWithSession(session, () => {
        try {
          const result = listener(
            event,
            () => {
              if (flushed) return next();
              flushed = true;
              wiretap
                .flush(session.handler, session.toEntry(null, 200))
                .catch(() => {});
              next();
            },
            ...rest,
          );
          const maybePromise = result as Promise<unknown> | undefined;
          if (maybePromise != null && typeof maybePromise.catch === "function") {
            maybePromise.catch(flushFailure);
          }
        } catch (error) {
          flushFailure(error);
          throw error;
        }
      });
    });
  };
}
```

**Note:** Listener may return `void` or `Promise`. Only call `.catch(flushFailure)` when the return value is a thenable, to avoid TypeScript errors (e.g. "An expression of type 'void' cannot be tested for truthiness").

## Bootstrap

Before `startAllMicroservices()`:

```typescript
wrapKafkaConsumer(app.get(KafkaConsumer), wiretap);
```
