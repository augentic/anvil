# `discovery.md` — three-section form

`discovery.md` is the single plan-time discovery artifact. `/spec:plan` writes it in step 5 after surveying each bound source. The file has three required sections in this order, each owned by `/spec:plan`:

1. `## Summary` — one-line counts (`Sources`, `Leads`). Adapter-specific tallies are permitted.
2. `## Source inventory` — one row per bound source under `plan.yaml.sources.<key>`: key, adapter, path or value.
3. `## Lead inventory` — one fenced or list block per lead. Stable `id` is the handle re-survey writes against; `sources[]` lists every source that surfaced the lead.

Re-surveying the same source key replaces leads by `id`. Surveying a different source key appends new ids. No `leads.yaml` exists in v1 — `discovery.md` is the only persisted lead artifact.

## Minimal lead block

The propose sub-step matches across sources using `id`, `summary`, and `sources[]` on these blocks:

```markdown
### user-registration

- id: user-registration
- sources: [legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

When the agent's propose sub-step judges a reconciliation uncertain, it adds a `tentative: true` bullet to each contributing block in this section and reasons about it in `change.md` under `## Tentative merges`. The lead block keeps every other field unchanged.

## N=1 degenerate form (`intent.survey`)

A pure-intent change scaffolds with a single `intent` binding. Discovery stays minimal but the file still exists:

```markdown
# Discovery — fix-typo

## Summary

Sources: 1. Leads: 1.

## Source inventory

| key    | adapter | value                      |
|--------|---------|----------------------------|
| intent | intent  | "fix typo in user.rs"      |

## Lead inventory

### fix-typo

- id: fix-typo
- sources: [intent]
- summary: fix typo in user.rs
```

The slice row `propose` writes against this lead uses the bare-string shorthand `sources: [intent]` (which the CLI normalises to `{ key: intent, lead: fix-typo }`).

## Multi-source skeleton

When two source adapters surface the same unit of work, both lead blocks share an `id` and each lists every source that surfaced it. The propose sub-step then writes one `slices[]` row with both bindings:

```markdown
### user-registration

- id: user-registration
- sources: [identity-design-notes, legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

When the two surfacing sources disagree on the summary materially (different numeric values, conflicting verbs, mutually exclusive nouns), the propose sub-step still merges them, invokes `specrun plan amend <name> <slice> --divergence likely` (the CLI is the single writer of `slices[].divergence`), and records the side-by-side summaries in `change.md` under `## Likely divergences`. The lead block itself keeps the consensus or last-written summary; pair-level detail lives in `change.md`.
