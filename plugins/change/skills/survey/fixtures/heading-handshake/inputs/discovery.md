# Discovery — sample-migration

## Adapter inventory

<!-- source-key: ops-docs -->
### user-management

```yaml
summary: Manage user accounts and adapters.
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
