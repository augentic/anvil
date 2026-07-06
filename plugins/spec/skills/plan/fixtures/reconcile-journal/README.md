# Reconcile-journal golden

Pins the journal tail the `plan author` reconcile kernel appends to `.specify/journal.jsonl` after it projects the agent reconciliation response onto `plan.yaml.slices[]`. A single `plan.reconcile.completed` event fires per successful invocation. The `/spec:plan` skill never runs `specify journal emit` for D2; the CLI owns this event.

## Scenario

A workspace `identity-revamp` change where the agent matched four surveyed leads across two sources (`docs`, `legacy`) into three slices:

- `identity-contracts` and `identity-service` — both reference the shared `identity-api` lead (matched by a shared slug across `docs` + `legacy`), fanned out to two projects and joined by `depends-on`.
- `password-reset` — `docs`'s `password-reset` and `legacy`'s `reset-password`, judged the same flow; bound to `identity-service` only.

That yields three slices (`slice-count: 3`).

## What the line pins

- **Shipped wire order** — the line serialises `timestamp` first, then `event`, then `payload` (workflow §Wire format), with kebab-case payload keys.
- **Single event** — the reconcile kernel emits exactly one `plan.reconcile.completed` event per successful invocation; there is no separate agent-phase journal line.
- **Completed payload** — `plan.reconcile.completed` carries `plan-name`, the matching `slice-count`, and the `slice-names` in the agent's response order.

## Files

- `journal.jsonl` — the single event line, in the byte shape the CLI appends it.
