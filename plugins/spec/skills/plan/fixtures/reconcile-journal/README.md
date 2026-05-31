# Reconcile-journal golden

Pins the paired journal tail `specrun plan propose --from` appends to `.specify/journal.jsonl` after it projects the agent reconciliation response onto `plan.yaml.slices[]`. Both events fire in one atomic, fsynced batch — `plan.reconcile.agent` first, then `plan.reconcile.completed`. The `/spec:plan` skill never runs `specrun journal emit` for D2; the CLI owns these events.

## Scenario

A hub `identity-revamp` change where the agent grouped four surveyed leads across two sources (`docs`, `legacy`) into two scopes:

- `identity-api` — matched by a shared slug across `docs` + `legacy`, then **fanned out** to two projects (`identity-contracts`, `identity-service`), so it projects to two `slices[]` rows.
- `password-reset` — `docs`'s `password-reset` and `legacy`'s `reset-password`, judged the same flow; bound to `identity-service` only.

That yields three slices (`slice-count: 3`) across two scopes.

## What the lines pin

- **Shipped wire order** — each line serialises `timestamp` first, then `event`, then `payload` (workflow §Wire format), with kebab-case payload keys.
- **Scope dedup** — `plan.reconcile.agent.payload.scopes` is deduped by `scope` id, so the `identity-api` fan-out contributes exactly **one** entry even though it projects to two slices.
- **`rationale` skip-when-absent** — `identity-api` carries its cross-source-match `rationale`; `password-reset` has none, so the field is omitted from that scope entry rather than serialised as `null`.
- **Completed payload** — `plan.reconcile.completed` carries the derived `slice-names` in the agent's response order alongside the matching `slice-count`.

## Files

- `journal.jsonl` — the two event lines, in the order and byte shape the CLI appends them.
