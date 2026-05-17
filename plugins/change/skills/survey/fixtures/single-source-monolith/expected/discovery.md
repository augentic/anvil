# Discovery — migrate-monolith

## Candidate inventory

<!-- source-key: legacy-monolith -->
### user-management [acceptable, 700 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
touches:
  - src/users/get.ts
  - src/users/register.ts
  - src/users/repository.ts
  - src/users/validate.ts
surfaces:
  - legacy-monolith:http-get-users-id
  - legacy-monolith:http-post-users
declared-at:
  - legacy-monolith:src/server.ts:14
  - legacy-monolith:src/server.ts:18
```

<!-- source-key: legacy-monolith -->
### order-creation [acceptable, 620 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/pricing.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-monolith:http-post-orders
declared-at:
  - legacy-monolith:src/server.ts:22
```
