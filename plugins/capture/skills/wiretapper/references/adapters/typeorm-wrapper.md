# Pattern F: typeorm-wrapper.ts

Generate when **TypeORM** is detected. Wrap `createQueryRunner()` only (do not also wrap `connection.query()` — double-capture). Pass `...rest` in the `query()` wrapper for TypeORM 0.3.x `useStructuredResult`. Capture with `result?.records ?? result`.

## Implementation

```typescript
// src/wiretap/adapters/typeorm-wrapper.ts

import { Wiretap } from "../wiretap";

type TypeOrmConnection = {
  createQueryRunner: (...args: unknown[]) => unknown;
};

export function wrapTypeOrmConnection(
  connection: TypeOrmConnection,
  wiretap: Wiretap,
): void {
  const originalCreateQR = connection.createQueryRunner.bind(connection);

  connection.createQueryRunner = (...args: unknown[]) => {
    const runner = originalCreateQR(...args) as { query: (query: string, parameters?: unknown[], ...rest: unknown[]) => Promise<unknown> };
    const originalRunnerQuery = runner.query.bind(runner);

    runner.query = async (
      query: string,
      parameters?: unknown[],
      ...rest: unknown[]
    ) => {
      const start = Date.now();
      const result = await originalRunnerQuery(query, parameters, ...rest);
      try {
        const session = wiretap.getCurrentSession();
        if (session) {
          const rawResult = (result as { records?: unknown })?.records ?? result;
          session.captureDbQuery({
            type: detectQueryType(query),
            query: query.trim(),
            parameters,
            result: summarizeResult(rawResult),
            duration_ms: Date.now() - start,
          });
        }
      } catch {
        /* never break the actual DB query */
      }
      return result;
    };

    return runner;
  };
}

function detectQueryType(
  query: string,
): "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "RAW" {
  const t = query.trim().toUpperCase();
  if (t.startsWith("SELECT")) return "SELECT";
  if (t.startsWith("INSERT")) return "INSERT";
  if (t.startsWith("UPDATE")) return "UPDATE";
  if (t.startsWith("DELETE")) return "DELETE";
  return "RAW";
}

function summarizeResult(result: unknown): unknown {
  if (Array.isArray(result) && result.length > 10) {
    return { rowCount: result.length, sample: result.slice(0, 3) };
  }
  return result;
}
```

## Bootstrap

```typescript
wrapTypeOrmConnection(app.get(DataSource), wiretap);
```
