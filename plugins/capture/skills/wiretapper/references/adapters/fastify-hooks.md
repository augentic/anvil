# Pattern A: fastify-hooks.ts

Generate when **Fastify (standalone)** is detected: `fastify` in deps, no `@nestjs/core`.

## Implementation

```typescript
// src/wiretap/adapters/fastify-hooks.ts

import { FastifyInstance } from "fastify";
import { Wiretap } from "../wiretap";
import { WiretapSession } from "../session";

const SKIP_PATHS = ["/status", "/swap"];

export function registerFastifyHooks(
  fastify: FastifyInstance,
  wiretap: Wiretap,
): void {
  fastify.addHook("onRequest", (req, reply, done) => {
    if (SKIP_PATHS.some((p) => req.url.startsWith(p))) return done();

    const session = new WiretapSession();
    session.setHandler(`${req.method} ${req.routerPath}`);
    const input: Record<string, unknown> = {
      ...((req.params as object) || {}),
      ...((req.query as object) || {}),
    };
    delete input.callback;
    if (
      req.body &&
      typeof req.body === "object" &&
      Object.keys(req.body as object).length
    ) {
      Object.assign(input, { body: req.body });
    }
    session.captureInput(input);
    wiretap.enterSession(session);
    done();
  });

  fastify.addHook("onSend", (req, reply, payload, done) => {
    const session = wiretap.getCurrentSession();
    if (session) {
      try {
        const parsed =
          typeof payload === "string" ? JSON.parse(payload) : payload;
        wiretap
          .flush(session.handler, session.toEntry(parsed, reply.statusCode))
          .catch(() => {});
      } catch {
        wiretap
          .flush(session.handler, session.toEntry(payload, reply.statusCode))
          .catch(() => {});
      }
    }
    done(null, payload);
  });
}
```

## Bootstrap

```typescript
registerFastifyHooks(webAppWrapper.getInstance(), wiretap);
```
