# Pattern G: kafka-nestclient-wrapper.ts

Generate when **NestJS ClientKafka** is detected: `@nestjs/microservices` + `ClientKafka` in source.

## Implementation

```typescript
// src/wiretap/adapters/kafka-nestclient-wrapper.ts

import { ClientKafka } from "@nestjs/microservices";
import { Wiretap } from "../wiretap";

export function wrapClientKafka(producer: ClientKafka, wiretap: Wiretap): void {
  const originalEmit = producer.emit.bind(producer);
  producer.emit = (pattern: string, data: unknown) => {
    try {
      const session = wiretap.getCurrentSession();
      if (session) session.captureKafkaPublish(pattern, data);
    } catch {
      /* never break the actual Kafka publish */
    }
    return originalEmit(pattern, data);
  };
}
```

## Bootstrap

```typescript
wrapClientKafka(app.get(ClientKafka), wiretap);
```
