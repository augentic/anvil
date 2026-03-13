# Pattern H: kafka-producer-wrapper.ts

Generate this file when Pattern H (standalone Kafka publisher) is detected. The wrapper supports **two producer APIs** so it works with both at-realtime-common and at-connector-common/kafkajs.

## Behavior

- **at-realtime-common**: `KafkaProducer` exposes `.publish(topic, incomingMsg)` where `incomingMsg` is `{ key?, value, ... }`. Wrap `publish` and capture topic, message (from `incomingMsg.value` or `incomingMsg`), and optional key.
- **at-connector-common / kafkajs**: producer exposes `.send(topic, key, message, ...rest)`. Wrap `send` and capture topic, key, message.
- **Guard**: If `producer` is null/undefined, or has neither `publish` nor `send`, return without wrapping (no-op). This avoids `Cannot read properties of undefined (reading 'bind')` when the DI returns a different shape.

## Full implementation

```typescript
import { Wiretap } from "../wiretap";

/**
 * Wraps a Kafka producer that has either:
 * - .send(topic, key, message, ...rest) (at-connector-common / kafkajs style), or
 * - .publish(topic, { key, value, ... }) (at-realtime-common style).
 * If the producer has neither, returns without wrapping (no-op).
 */
export function wrapKafkaProducer(
  producer: unknown,
  wiretap: Wiretap,
): void {
  if (producer == null) return;

  const p = producer as Record<string, unknown>;

  // at-realtime-common: publish(topic, { key?, value, ... })
  if (typeof p.publish === "function") {
    const originalPublish = p.publish.bind(producer);
    p.publish = (topic: string, incomingMsg: { key?: string; value?: unknown; [k: string]: unknown }) => {
      try {
        const session = wiretap.getCurrentSession();
        if (session) {
          const msg = incomingMsg?.value ?? incomingMsg;
          session.captureKafkaPublish(topic, msg, incomingMsg?.key);
        }
      } catch {
        /* never break the actual Kafka publish */
      }
      return originalPublish(topic, incomingMsg);
    };
    return;
  }

  // at-connector-common / kafkajs style: send(topic, key, message, ...rest)
  if (typeof p.send === "function") {
    const originalSend = p.send.bind(producer);
    p.send = (topic: string, key: string, message: unknown, ...rest: unknown[]) => {
      try {
        const session = wiretap.getCurrentSession();
        if (session) session.captureKafkaPublish(topic, message, key);
      } catch {
        /* never break the actual Kafka publish */
      }
      return originalSend(topic, key, message, ...rest);
    };
  }
}
```

## Bootstrap

Called once per producer instance, e.g.:

```typescript
wrapKafkaProducer(app.get(KafkaProducer), wiretap);
```
