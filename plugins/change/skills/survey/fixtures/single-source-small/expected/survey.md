# migrate-widget survey

## Summary

Sources: 1 | Surfaces: 3 | Candidates: 1 | Unresolved: 0

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-widget | ./legacy-widget-source | typescript | 850 | 3 |

## Candidate inventory

### legacy-widget [acceptable, 850 LOC]

```yaml
kind: candidate
sources: [legacy-widget]
touches:
  - src/handlers/create.ts
  - src/handlers/get.ts
  - src/handlers/list.ts
  - src/services/validate.ts
  - src/services/widget-service.ts
surfaces:
  - legacy-widget:http-get-widgets
  - legacy-widget:http-get-widgets-id
  - legacy-widget:http-post-widgets
declared-at:
  - legacy-widget:src/server.ts:10
  - legacy-widget:src/server.ts:11
  - legacy-widget:src/server.ts:12
```
