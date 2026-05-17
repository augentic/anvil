# Discovery — demo-repair

## Candidate inventory

<!-- source-key: legacy-svc -->
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
