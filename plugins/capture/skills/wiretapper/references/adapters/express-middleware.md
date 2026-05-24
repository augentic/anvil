# Pattern B: express-middleware.ts

Generate when **Express** is detected: `express` in deps, no `@nestjs/core`.

## Implementation

```typescript
// src/wiretap/adapters/express-middleware.ts

import * as express from "express";
import { Wiretap } from "../wiretap";
import { WiretapSession } from "../session";

const SKIP_PATHS = ["/status", "/swap"];

export function wiretapMiddleware(wiretap: Wiretap) {
  return (
    req: express.Request,
    res: express.Response,
    next: express.NextFunction,
  ) => {
    if (SKIP_PATHS.some((p) => req.url.startsWith(p))) return next();

    const session = new WiretapSession();
    session.setHandler(`${req.method} ${req.route?.path || req.path}`);
    const input: Record<string, unknown> = { ...req.params, ...req.query };
    if (req.body && Object.keys(req.body).length) input.body = req.body;
    session.captureInput(input);
    wiretap.enterSession(session);

    const originalJson = res.json.bind(res);
    res.json = (body: unknown) => {
      wiretap
        .flush(session.handler, session.toEntry(body, res.statusCode))
        .catch(() => {});
      return originalJson(body);
    };

    next();
  };
}
```

## Bootstrap

```typescript
webApp.use(wiretapMiddleware(wiretap));
```
