# `survey.md` shape

Canonical shape for `.specify/plans/<change>/survey.md`. The file is byte-stable: re-running on unchanged inputs produces byte-identical output.

## Required sections (in order)

### 1. Title

```markdown
# <change-name> survey
```

### 2. Summary

One-line counts:

```markdown
## Summary

Sources: 2 | Surfaces: 8 | Candidates: 5 | Unresolved: 1
```

### 3. Source inventory

One row per input source:

```markdown
## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-monolith | ./legacy/monolith | typescript | 4200 | 6 |
| legacy-billing | ./legacy/billing | typescript | 1800 | 2 |
```

### 4. Candidate inventory

One fenced-YAML block per candidate. Fields appear in fixed order so re-runs diff cleanly: `kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`.

```markdown
## Candidate inventory

### identity.user-registration [acceptable, 894 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
handler: src/auth/register.ts:registerUser
touches:
  - src/auth/register.ts
  - src/notifications/email.ts
  - src/users/repository.ts
surfaces:
  - legacy-monolith:http-post-users
  - legacy-monolith:message-pub-user-created
declared-at:
  - legacy-monolith:src/server.ts:42
  - legacy-monolith:src/users/events.ts:18
```

### billing.invoice-sync [too-large, 1320 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
touches:
  - src/billing/invoices.ts
  - src/billing/reconciliation.ts
  - src/billing/settlement.ts
surfaces:
  - legacy-billing:scheduled-job-invoice-sync
  - legacy-billing:message-sub-payment-settled
declared-at:
  - legacy-billing:src/billing/scheduler.ts:24
  - legacy-billing:src/billing/subscriptions.ts:11
unresolved: true
```
```

## Field rules

- **`kind`** — always `candidate`.
- **`sources`** — list of source keys. For survey-derived candidates, typically a single source key.
- **`handler`** — the handler or call site for the candidate's primary surface. Omit for source-level candidates (Decision 1) or when multiple handlers apply.
- **`touches`** — deduplicated, alphabetically sorted list of source files reached from the handler(s). Paths are relative to the source root.
- **`surfaces`** — namespaced `<source-key>:<surface-id>`, alphabetically sorted.
- **`declared-at`** — namespaced `<source-key>:<path>` or `<source-key>:<path>:<line>`, alphabetically sorted. Non-empty.
- **`unresolved`** — present and `true` only when the candidate is `too-large` and cannot be split further. Omit when false.

## Heading format

Each candidate heading follows the pattern:

```text
### <candidate-name> [<size-bucket>, <LOC> LOC]
```

Where `<size-bucket>` is `acceptable` or `too-large`.
