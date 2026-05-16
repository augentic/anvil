# Discovery — migrate-billing

## Candidate inventory

<!-- source-key: legacy-billing -->
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

<!-- source-key: legacy-billing -->
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
