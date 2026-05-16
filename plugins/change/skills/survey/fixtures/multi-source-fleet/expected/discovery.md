# Discovery — migrate-fleet

## Candidate inventory

<!-- source-key: legacy-api -->
### user-list [acceptable, 540 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/users/list.ts:listUsers
touches:
  - src/users/list.ts
  - src/users/repository.ts
surfaces:
  - legacy-api:http-get-users
declared-at:
  - legacy-api:src/server.ts:9
```

<!-- source-key: legacy-api -->
### order-creation [acceptable, 660 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-api:http-post-orders
declared-at:
  - legacy-api:src/server.ts:14
```

<!-- source-key: legacy-billing -->
### legacy-billing [acceptable, 780 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
touches:
  - src/invoices/list.ts
  - src/invoices/repository.ts
  - src/payments/create.ts
  - src/payments/repository.ts
surfaces:
  - legacy-billing:http-get-invoices
  - legacy-billing:http-post-payments
declared-at:
  - legacy-billing:src/server.ts:12
  - legacy-billing:src/server.ts:8
```
