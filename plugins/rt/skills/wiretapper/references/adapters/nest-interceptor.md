# Pattern C: nest-interceptor.ts

Generate when **NestJS** is detected. HTTP only — wrap `reply.send()` for output; Kafka is handled by Pattern D.

## Implementation

```typescript
// src/wiretap/adapters/nest-interceptor.ts

import {
  Injectable,
  NestInterceptor,
  ExecutionContext,
  CallHandler,
} from "@nestjs/common";
import { Observable } from "rxjs";
import { Wiretap } from "../wiretap";
import { WiretapSession } from "../session";

const SKIP_PATHS = ["/status", "/swap"];

@Injectable()
export class WiretapInterceptor implements NestInterceptor {
  constructor(private wiretap: Wiretap) {}

  intercept(context: ExecutionContext, next: CallHandler): Observable<unknown> {
    const session = new WiretapSession();
    const ctxType = context.getType();

    if (ctxType === "http") {
      const req = context.switchToHttp().getRequest();
      if (SKIP_PATHS.some((p: string) => req.url.startsWith(p))) return next.handle();

      const reply = context.switchToHttp().getResponse();
      session.setHandler(`${req.method} ${req.routerPath || req.url}`);
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

      const originalSend = reply.send.bind(reply);
      let flushed = false;
      reply.send = (payload: unknown) => {
        if (!flushed) {
          flushed = true;
          try {
            const parsed =
              typeof payload === "string" ? JSON.parse(payload) : payload;
            this.wiretap
              .flush(session.handler, session.toEntry(parsed, reply.statusCode))
              .catch(() => {});
          } catch {
            this.wiretap
              .flush(
                session.handler,
                session.toEntry(payload, reply.statusCode),
              )
              .catch(() => {});
          }
        }
        return originalSend(payload);
      };
    } else {
      return next.handle();
    }

    return new Observable((subscriber) => {
      this.wiretap.runWithSession(session, () => {
        next.handle().subscribe({
          next: (result) => subscriber.next(result),
          error: (err) => subscriber.error(err),
          complete: () => subscriber.complete(),
        });
      });
    });
  }
}
```

## Bootstrap

```typescript
app.useGlobalInterceptors(new WiretapInterceptor(wiretap));
```
