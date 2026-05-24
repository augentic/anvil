# `discovery.md` — three-section form

`discovery.md` is the single plan-time discovery artifact. `/spec:plan` writes it in step 5 after enumerating each bound source. The file has three required sections in this order, each owned by `/spec:plan`:

1. `## Summary` — one-line counts (`Sources`, `Candidates`). Adapter-specific tallies are permitted.
2. `## Source inventory` — one row per bound source under `plan.yaml.sources.<key>`: key, adapter, path or value.
3. `## Candidate inventory` — one fenced or list block per candidate. Stable `id` is the handle re-enumeration writes against; `sources[]` lists every source that surfaced the candidate.

Re-enumerating the same source key replaces candidates by `id`. Enumerating a different source key appends new ids. No `candidates.yaml` exists in v1 — `discovery.md` is the only persisted candidate artifact.

## Minimal candidate block

The propose sub-step matches across sources using `id`, `summary`, and `sources[]` on these blocks:

```markdown
### user-registration

- id: user-registration
- sources: [legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

When the agent's propose sub-step judges a fusion uncertain, it adds a `tentative: true` bullet to each contributing block in this section and reasons about it in `change.md` under `## Tentative merges`. The candidate block keeps every other field unchanged.

## N=1 degenerate form (`intent.enumerate`)

A pure-intent change scaffolds with a single `intent` binding. Discovery stays minimal but the file still exists:

```markdown
# Discovery — fix-typo

## Summary

Sources: 1. Candidates: 1.

## Source inventory

| key    | adapter | value                      |
|--------|---------|----------------------------|
| intent | intent  | "fix typo in user.rs"      |

## Candidate inventory

### fix-typo

- id: fix-typo
- sources: [intent]
- summary: fix typo in user.rs
```

The slice row `propose` writes against this candidate uses the bare-string shorthand `sources: [intent]` (which the CLI normalises to `{ key: intent, candidate: fix-typo }`).

## Multi-source skeleton

When two source adapters surface the same unit of work, both candidate blocks share an `id` and each lists every source that surfaced it. The propose sub-step then writes one `slices[]` row with both bindings:

```markdown
### user-registration

- id: user-registration
- sources: [identity-design-notes, legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

When the two surfacing sources disagree on the summary materially (different numeric values, conflicting verbs, mutually exclusive nouns), the propose sub-step still merges them, invokes `specify plan amend <name> <slice> --divergence likely` (the CLI is the single writer of `slices[].divergence`), and records the side-by-side summaries in `change.md` under `## Likely divergences`. The candidate block itself keeps the consensus or last-written summary; pair-level detail lives in `change.md`.
