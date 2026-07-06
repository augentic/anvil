# `discovery.md` — three-section form

`discovery.md` is the single plan-time discovery artifact. `/spec:plan` writes it in step 5 after surveying each bound source. The file has three required sections in this order, each owned by `/spec:plan`:

1. `## Summary` — one-line counts (`Sources`, `Leads`). Adapter-specific tallies are permitted.
2. `## Source inventory` — one row per bound source under `plan.yaml.sources.<key>`: key, adapter, path or value.
3. `## Lead inventory` — one fenced or list block per **raw, unmerged lead**. Each block is one lead as surfaced by one source: a kebab-case `lead` and the scalar `source` that surfaced it. Identity is the `(source, lead)` pair, so the same `lead` MAY appear under different source keys.

Re-surveying the same source key replaces that source's leads by `(source, lead)` and leaves every other source's blocks untouched. `survey` never merges across sources — cross-source unification is `/spec:plan`'s `propose` sub-step. No `leads.yaml` exists in v1 — `discovery.md` is the only persisted lead artifact.

## Minimal lead block

The propose sub-step matches across sources using `lead`, `synopsis`, and `source` on these blocks:

```markdown
### legacy-monolith:user-registration

- lead: user-registration
- source: legacy-monolith
- synopsis: Registration endpoint accepting email + password with RFC-5322 validation.
- topics: [identity, account-creation, validation]
```

The heading is `### <source>:<lead>` so two sources surfacing the same `lead` stay distinct blocks. Survey lead-sets MAY omit `source` (the CLI stamps it from the survey binding); the persisted `discovery.md` always carries it.

`topics` is an **optional** inline list of kebab-case slugs the survey agent authors as extra per-lead context. It is additional grouping signal for the propose agent and the join key for the decision-contradiction warning; the CLI computes no grouping from it, and a lead with no `topics` reconciles on its `synopsis` alone. Omit the bullet entirely when a lead is unclassified.

Each `synopsis` SHOULD be content-bearing — name the lead's operation/surface and its salient constraint so a same-slug lead from another source can be matched or distinguished on content, not just the shared slug. It MAY span more than one line when one is too thin; it stays plan-time headline material, never a back-door for slice-time `Evidence`. There is no survey-time scope-uncertainty flag: a lead is always a lead. Grouping uncertainty is the agent's to express in `change.md` under `## Tentative merges`, never on a lead block — the `/spec:plan` reconcile sub-step never edits `discovery.md` (see [lead reconciliation](./cli/plan-propose.md)).

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

- lead: fix-typo
- source: intent
- synopsis: fix typo in user.rs
```

`propose --from` writes the slice row against this lead as the structured binding `{ source: intent, lead: fix-typo }` under the auto-bound sole project; the bare-string shorthand `sources: [intent]` is the equivalent hand-authored sugar (lead defaults to the slice name).

## Multi-source skeleton

When two source adapters surface the same unit of work, each survey writes its **own** raw lead block: the same `lead` may appear once per source, each with its own `source` and per-source `synopsis`. The reconcile sub-step groups them by agent judgment (shared slug or synopsis) — not kernel lock — and writes one or more `slices[]` rows through the `plan author` reconcile kernel. The operator reviews cross-source merges at Gate 1:

```markdown
### identity-design-notes:user-registration

- lead: user-registration
- source: identity-design-notes
- synopsis: Registration endpoint accepting email + password with RFC-5322 validation.

### legacy-monolith:user-registration

- lead: user-registration
- source: legacy-monolith
- synopsis: POST /users handler validating email + password and inserting the new user record.
```

When the two surfacing sources disagree on the synopsis materially (different numeric values, conflicting verbs, mutually exclusive nouns), the propose sub-step still merges them into one slice and flags it `divergence: likely` — either directly on the response slice (preferred, alongside the structured `disagreements[]`) or via `specify plan amend <entry> --divergence likely` (the CLI is the single writer of `slices[].divergence`). The machine-readable disagreeing values live in the slice's `disagreements[]` (`{ field, values: [{ source, value }] }`); the operator-facing side-by-side synopses go in `change.md` under `## Likely divergences`. Each raw lead block keeps its own per-source synopsis; the CLI advises on consistency (`slice-divergence-unrecorded` / `slice-divergence-orphan-values`) but never blocks.
