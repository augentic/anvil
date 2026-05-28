# Divergence-journal golden

Pins the `plan.amend.divergence` journal event the CLI fires when the propose sub-step invokes `specrun plan amend <name> <slice> --divergence likely`.

## Scenario

Same shape as `cross-source-identity-revamp/` plus a third slice (`identity-session-expiry`) where the two contributing sources surfaced the **same** lead id but with materially-disagreeing summaries:

- `identity-design-notes` says: "Session tokens expire after 60 minutes of inactivity."
- `legacy-monolith` says: "Session tokens expire after 24 hours of absolute lifetime."

`propose` fuses them into one slice, then invokes `specrun plan amend identity-revamp identity-session-expiry --divergence likely`. The CLI is the single writer of `plan.yaml.slices[].divergence` and fires one `plan.amend.divergence` journal event per invocation; the skill keeps authoring the `## Likely divergences` block in `change.md`. Operator override at Gate 1 is `specrun plan amend identity-session-expiry --divergence accepted` (or `rejected`).

## Files

- `plan.yaml.fragment` — the `slices[]` row carrying `divergence: likely` (written by the CLI on amend).
- `change.md.fragment` — the `## Likely divergences` block (authored by the skill).
- `journal.jsonl` — one event line, exactly as the CLI appends it to `.specify/journal.jsonl`.
