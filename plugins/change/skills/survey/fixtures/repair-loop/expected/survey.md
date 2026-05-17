# demo-repair survey

## Summary

Sources: 1 | Surfaces: 1 | Candidates: 1 | Unresolved: 0

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-svc | ./legacy-svc-source | typescript | 200 | 1 |

## Candidate inventory

### legacy-svc [acceptable, 200 LOC]

```yaml
kind: candidate
sources: [legacy-svc]
touches:
  - src/handler.ts
surfaces:
  - legacy-svc:http-get-health
declared-at:
  - legacy-svc:src/server.ts:4
```
