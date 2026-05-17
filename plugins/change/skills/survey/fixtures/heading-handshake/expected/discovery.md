# Discovery — sample-migration

## Capability inventory

<!-- source-key: ops-docs -->
### user-management

```yaml
summary: Manage user accounts and profiles.
sources:
  - ops-docs
depends-on: []
hints:
  entry_points: [POST /users, GET /users/:id]
  external_deps: [postgres]
confidence: high
```

## Candidate inventory

<!-- source-key: ops-docs -->
### rotate-api-key

```yaml
kind: candidate
sources: [ops-docs]
surfaces:
  - ops-docs:cli-command-rotate-api-key
declared-at:
  - ops-runbook.md#rotate-the-api-key
```

<!-- source-key: legacy-api -->
### legacy-api [acceptable, 420 LOC]

```yaml
kind: candidate
sources: [legacy-api]
touches:
  - src/users/create.ts
  - src/users/get.ts
  - src/users/repository.ts
  - src/users/validate.ts
surfaces:
  - legacy-api:http-get-users-id
  - legacy-api:http-post-users
declared-at:
  - legacy-api:src/server.ts:12
  - legacy-api:src/server.ts:15
```
