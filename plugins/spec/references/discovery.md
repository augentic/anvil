# `discovery.md` — three-section form

`discovery.md` is the single plan-time discovery artifact. `/spec:plan` writes it in step 5 after surveying each bound source. The file has three required sections in this order, each owned by `/spec:plan`:

1. `## Summary` — one-line counts (`Sources`, `Leads`). Adapter-specific tallies are permitted.
2. `## Source inventory` — one row per bound source under `plan.yaml.sources.<key>`: key, adapter, path or value.
3. `## Lead inventory` — one fenced or list block per **raw, unmerged lead**. Each block is one lead as surfaced by one source: a kebab-case `lead-id` and the scalar `source-key` that surfaced it. Identity is the `(source-key, lead-id)` pair, so the same `lead-id` MAY appear under different source keys.

Re-surveying the same source key replaces that source's leads by `(source-key, lead-id)` and leaves every other source's blocks untouched. `survey` never merges across sources — cross-source unification is `/spec:plan`'s `propose` sub-step. No `leads.yaml` exists in v1 — `discovery.md` is the only persisted lead artifact.

## Minimal lead block

The propose sub-step matches across sources using `lead-id`, `aliases[]`, `summary`, and `source-key` on these blocks:

```markdown
### legacy-monolith:user-registration

- lead-id: user-registration
- source-key: legacy-monolith
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

The heading is `### <source-key>:<lead-id>` so two sources surfacing the same `lead-id` stay distinct blocks. Survey lead-sets MAY omit `source-key` (the CLI stamps it from the survey binding); the persisted `discovery.md` always carries it.

Each `summary` SHOULD be content-bearing — name the lead's operation/surface and its salient constraint so a same-slug lead from another source can be matched or distinguished on content, not just the shared slug. It MAY span more than one line when one is too thin; it stays plan-time headline material, never a back-door for slice-time `Evidence`. There is no survey-time scope-uncertainty flag: a lead is always a lead. Grouping uncertainty is the agent's to express in `change.md` under `## Tentative merges`, never on a lead block — the `/spec:plan` propose sub-step never edits `discovery.md` (see [`specrun plan propose`](../../../docs/reference/cli/plan.md#specrun-plan-propose)).

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

### intent:fix-typo

- lead-id: fix-typo
- source-key: intent
- summary: fix typo in user.rs
```

`propose --from` writes the slice row against this lead as the structured binding `{ source-key: intent, lead-id: fix-typo }` under the auto-bound sole project; the bare-string shorthand `sources: [intent]` is the equivalent hand-authored sugar (lead-id defaults to the slice name).

## Multi-source skeleton

When two source adapters surface the same unit of work, each survey writes its **own** raw lead block: the same `lead-id` may appear once per source, each with its own `source-key` and per-source `summary`. The propose sub-step groups them by agent judgment (shared slug, alias hints, or summary) — not kernel lock — and writes one or more `slices[]` rows via `specrun plan propose --from`. The operator reviews cross-source merges at Gate 1:

```markdown
### identity-design-notes:user-registration

- lead-id: user-registration
- source-key: identity-design-notes
- summary: Registration endpoint accepting email + password with RFC-5322 validation.

### legacy-monolith:user-registration

- lead-id: user-registration
- source-key: legacy-monolith
- summary: POST /users handler validating email + password and inserting the new user record.
```

When the two surfacing sources disagree on the summary materially (different numeric values, conflicting verbs, mutually exclusive nouns), the propose sub-step still merges them into one slice, invokes `specrun plan amend <entry> --divergence likely` (the CLI is the single writer of `slices[].divergence`), and records the side-by-side summaries in `change.md` under `## Likely divergences`. Each raw lead block keeps its own per-source summary; pair-level detail lives in `change.md`.
