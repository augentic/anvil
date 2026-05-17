# express-migration survey

## Summary

Sources: 1 | Surfaces: 3 | Candidates: 1 | Unresolved: 0

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| express-app | ./source | typescript | 31 | 3 |

## Candidate inventory

### express-app [acceptable, 31 LOC]

```yaml
kind: candidate
sources: [express-app]
touches:
  - src/routes/users.ts
  - src/server.ts
  - src/services/user-service.ts
  - src/utils/db.ts
surfaces:
  - express-app:http-get-health
  - express-app:http-get-users
  - express-app:http-post-users
declared-at:
  - express-app:src/server.ts:10
  - express-app:src/server.ts:6
  - express-app:src/server.ts:9
```
