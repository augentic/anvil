# Synthesis playbook

The synthesis playbook is the agent-facing reconciliation contract `/spec:refine` follows when it folds per-source `Evidence[]` plus the active slice's target `shape` brief into the canonical slice artifacts (`proposal.md`, `specs/<unit>/spec.md`, `design.md`, `tasks.md`). The skill body owns the CLI choreography (slice create, serial extract, validate, transition); this playbook owns **what to write into the artifacts**.

## Substeps in fixed order

The skill body invokes synthesis in this order — each substep is hand-coded, there is no `specrun slice synthesize` verb:

1. **`proposal.md`** — why, units, non-goals. Carries the slice's *why*.
2. **`specs/<unit>/spec.md`** — behavioural requirements. One file per `## Units` entry. Every block carries `ID:`, `Sources:`, `Status:`. This is the only artifact the provenance parser validates.
3. **`design.md`** — domain model, APIs, integrations, configuration, technical logic, target-idiom folding (provider DI / Crux idioms / contract format choice), and the UI / layout subsection that spatial Evidence (`region` / `container` / `leaf` from the `screenshots` source adapter) folds into.
4. **`tasks.md`** — implementation sequencing as plain markdown checkboxes (`- [ ] …`).

See [`substeps.md`](substeps.md) for the per-artifact contract.

## What each file in this playbook owns

| File                                       | Owns                                                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------ |
| [`substeps.md`](substeps.md)               | Per-artifact contract for the four substeps; what each artifact MUST contain.              |
| [`authority.md`](authority.md)             | Authority hierarchy, the per-slice override on `plan.yaml` (per-Evidence per-kind overrides are deferred), the resolution order synthesis walks, and the agreement → `Status` decision table. |
| [`requirement-block.md`](requirement-block.md) | Canonical `spec.md` requirement-block template + worked examples per `Status` variant. |
| [`claim-reconciliation.md`](claim-reconciliation.md)       | How to reconcile per-`kind` and per-`authority` claims; where each claim kind lands.            |
| [`provenance.md`](provenance.md)                   | Provenance projection (`specrun slice provenance`, inline in `model.yaml`): block grammar per `resolution` enum value, inline `value` truncation, `winner` markers, and `resolution-trace` step names. |
| [`tags.md`](tags.md)                       | Tag grammar (`[unknown]` / `[conflict]` / `[divergence]`) and the tag ↔ `Status` coherence rule. |

## Posture

Uncertainty produces review tags rather than parking the slice. The slice lifecycle stays `refining → refined → built → merged` regardless of how many `[unknown]` / `[conflict]` / `[divergence]` tags survive synthesis. Operators reconcile by hand-editing `spec.md` after `/spec:refine` transitions; the playbook's job is to surface the tag, not to second-guess it.
