# Divergence-journal golden

Pins the `plan.propose.divergence` journal event the propose sub-step emits when it sets `slices[].divergence: likely` on a slice.

## Scenario

Same shape as `cross-source-identity-revamp/` plus a third slice (`identity-session-expiry`) where the two contributing sources surfaced the **same** candidate id but with materially-disagreeing summaries:

- `identity-design-notes` says: "Session tokens expire after 60 minutes of inactivity."
- `legacy-monolith` says: "Session tokens expire after 24 hours of absolute lifetime."

`propose` fuses them into one slice, sets `divergence: likely`, adds the pair to `change.md` under `## Likely divergences`, and emits one journal line for the affected slice. Operator override at Gate 1 is `specify plan amend identity-session-expiry --divergence accepted` (or `rejected`).

## Files

- `plan.yaml.fragment` — the `slices[]` row carrying `divergence: likely`.
- `change.md.fragment` — the `## Likely divergences` block.
- `journal.jsonl` — one event line, exactly as the skill appends it to `.specify/journal.jsonl`.
