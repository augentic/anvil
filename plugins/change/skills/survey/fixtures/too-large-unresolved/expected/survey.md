# migrate-billing survey

## Summary

Sources: 1 | Surfaces: 2 | Candidates: 2 | Unresolved: 1

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-billing | ./legacy/billing | typescript | 1320 | 2 |

## Candidate inventory

### payment-settled [acceptable, 900 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
handler: src/billing/subscriptions.ts:onPaymentSettled
touches:
  - src/billing/core.ts
  - src/billing/settlement.ts
  - src/billing/subscriptions.ts
surfaces:
  - legacy-billing:message-sub-payment-settled
declared-at:
  - legacy-billing:src/billing/subscriptions.ts:11
```

### invoice-sync [too-large, 1020 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
handler: src/billing/invoices.ts:syncInvoices
touches:
  - src/billing/core.ts
  - src/billing/invoices.ts
surfaces:
  - legacy-billing:scheduled-job-invoice-sync
declared-at:
  - legacy-billing:src/billing/scheduler.ts:24
unresolved: true
```
