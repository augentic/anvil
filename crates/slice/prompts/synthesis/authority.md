# Authority hierarchy

Top-level `authority:` on every `Evidence` document is a closed enum. Highest wins:

1. **`intent`** — operator override at slice time (`intent` source adapter).
2. **`documentation`** — operator-provided written product / technical intent (`documentation`, `screenshots`). Distinct from synthesised `design.md` and from the refine substep named `design`.
3. **`behaviour`** — what legacy code actually does (`typescript`, `captures`, and future code/observation adapters).

Authority defaults to the **Evidence document**. v1 adds one opt-in surface (see [§Authority overrides](#authority-overrides)): a per-slice override on `plan.yaml`. Without `authority-override`, the document-level rule applies. (A per-Evidence per-kind `authority-overrides` surface is deferred to a future RFC.)

The **agent** never resolves authority or marks winners. It records contributing `(source, id, kind)` claims and an `agreement` verdict (`agreed` / `disagreed`). The **kernel** resolves authority after the response returns, then derives `status`, winner markers, and `Sources:`.

## `agreement` verdict → kernel `status` derivation

| `claims` | `agreement` | Kernel `status` | Tag | Winner markers |
| -------- | ----------- | --------------- | --- | -------------- |
| 0 | *(omitted)* | `unknown` | `[unknown]` | none |
| 1 | *(omitted)* | `agreed` | (none) | none |
| ≥2 | `agreed` | `agreed` | (none) | none |
| ≥2 | `disagreed`, unique top authority | `divergence` | `[divergence]` | winner `true`, losers `false` |
| ≥2 | `disagreed`, top authority ties | `conflict` | `[conflict]` | none |

Headline tag must match `status` per [`tags.md`](./tags.md); the provenance parser refuses hand-edits where they disagree.

## Worked applications

Kernel **output** shapes — agent authors heading/body prose, claims, and `agreement` only:

| Case | Agent records | Kernel renders |
| ---- | ------------- | -------------- |
| Single source | one claim; omit `agreement` | `Status: agreed`; no tag |
| Multiple agree | ≥2 claims; `agreement: agreed` | shared statement; `Sources:` highest authority first |
| Disagree, one wins | ≥2 claims; `agreement: disagreed` | winner as body; loser as `Note:`; `Status: divergence` |
| Tied top authority | ≥2 same class; `agreement: disagreed` | only `Note:` lines; `Status: conflict` |
| No Evidence | empty claims | `Sources: []`; `Status: unknown` |

### Divergence example (documentation beats behaviour)

Docs say 30 minutes; code observed 15. Default `documentation > behaviour`. Agent: both claims + `agreement: disagreed` + docs-winning statement + loser in `notes`. Kernel:

```markdown
### Requirement: Session timeout [divergence]

ID: REQ-001
Sources: [docs, code]
Status: divergence

The system expires idle sessions after 30 minutes. (from docs; documentation)

Note: code observed 15-minute expiry; the documentation authority overrides. Operator review recommended.
```

## Authority overrides

Document-level `authority:` is the default. One opt-in override sharpens it — typically when production behaviour should beat stale docs.

> **Deferred (future RFC).** Per-Evidence per-kind `authority-overrides: { <claim-kind>: <authority-class> }` is out of scope for v1.

### Per-slice overrides on `plan.yaml`

Each `plan.yaml.slices[]` entry MAY carry `authority-override: { <claim-kind>: <source> }`. Keys are the closed claim-kind enum; values MUST appear in that slice's `sources[]`.

- Scoped to a single slice (no plan-/project-wide overrides).
- Orphan source keys → `slice-authority-override-orphan-source` from `emery slice validate` before the refine phase.
- Operators author via CLI; agents never hand-edit `plan.yaml`:

```bash
emery plan amend <entry> --authority-override <entry> <claim-kind>=<source>
emery plan amend <entry> --clear-authority-override <entry> <claim-kind>
emery plan amend <entry> --clear-authority-overrides
emery plan add   <entry> --authority-override <claim-kind>=<source>   # repeatable on create
```

Example: `authority-override: { criterion: runtime }`. Agent still records both claims with `agreement: disagreed`; kernel promotes `runtime`, loser as `Note:`. Audit: `resolution-trace.step: per-slice-authority-override` with `override: { criterion: runtime }` and `winner: runtime`.

### Resolution order

When `agreement` is `disagreed`, the kernel walks these steps; the first winner stops the walk. The step name lands at `requirements[].resolution-trace.step` in `model.yaml` (surfaced by `emery slice provenance`).

1. **`per-slice-authority-override`** — `authority-override.<kind>` names a contributing source → that source wins; `status: divergence` (or `agreed` if values align); runner-up `winner: false`.
2. **`document-authority-ordering`** — fall back to `intent > documentation > behaviour`. Highest class wins; top-class ties continue to step 3.
3. **`tied-conflict`** — still tied → `status: conflict`, `[conflict]`, no winners. Operator amends the override or sources, then re-runs `emery plan execute` to re-refine before the build phase.

Steps 1–2 yield `divergence` when the winner disagrees with another contributor and `agreed` when all values match the winner. Step names are byte-stable and match `resolution-trace.step` exactly. See [`claim-reconciliation.md`](./claim-reconciliation.md) for per-kind landing. (Deferred per-Evidence surface would insert `per-evidence-authority-override` between 1 and 2.)

## Notes

- Authority is slice-time only (not plan-time `propose`).
- Below per-slice granularity: amend `authority-override` and re-run the refine phase (`emery plan execute`) — never hand-edit kernel-rendered provenance.
- `Sources:` lists every contributing key, highest authority first **after override resolution** (override-promoted behaviour keys sort first for that block).
- Provenance parser cross-resolves `Sources:` and override values against slice bindings.
- Every resolution (including step-2 fallbacks) lands in `model.yaml` `resolution-trace.step` and is projected by `emery slice provenance`; `spec.md` stays operator-facing prose.
