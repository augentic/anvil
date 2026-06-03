# Synthesis playbook

The synthesis playbook is the agent-facing reconciliation contract `/spec:refine` follows when it authors the **synthesis response** that `specify slice synthesize` projects into the canonical slice artifacts (`proposal.md`, `specs/<unit>/spec.md`, `design.md`, `tasks.md`) and the typed `model.yaml`. The skill body owns the CLI choreography (slice create, serial extract, the two-phase `specify slice synthesize` handoff, validate, transition); this playbook owns **what to write into the response**.

## Agent / kernel split

`specify slice synthesize <slice> --dry-run` assembles the **inputs** envelope (each bound source's inline `lead` + `claims`, plus the resolved target `shape` brief); the agent authors the **response**; `specify slice synthesize <slice> --from <response.json>` runs the CLI-owned projection kernel and persists everything. The line between the two is sharp:

- **Agent (response).** Reconciles `Evidence[]` into the requirement set: which requirements exist and how claims merge or split. Per requirement it records the contributing `(source, id, kind)` claims, an `agreement` verdict (`agreed` / `disagreed`), the behavioural prose (`title`, `statement`, `scenarios[]`, `notes`), and the owning `unit`; plus the prose-only `proposal.md` / `design.md` / `tasks.md` bodies and the spec bodies written **without** `ID:` / `Sources:` / `Status:` lines. The agent authors `TASK` ids and `satisfies[]` references (pointing at the declaration-order `REQ` ids the kernel will assign).
- **Kernel (CLI).** Stamps the `version` / `slice` / `project` header, resolves authority, assigns `REQ` ids in declaration order, derives `status` and per-claim `winner` markers, renders the highest-authority-first `Sources:` lists, writes the inline provenance into `model.yaml`, and renders the `ID:` / `Sources:` / `Status:` lines into `spec.md`. Any kernel-owned field the agent supplies is ignored and re-derived — the kernel **normalises, never rejects**.

## Sections of the response, in fixed order

Author the response so its prose reads top-down in the same order the artifacts persist:

1. **`proposal.md`** — why, units, non-goals. Carries the slice's *why*.
2. **`specs/<unit>/spec.md` bodies** — behavioural requirement prose. One spec body per `## Units` entry. The kernel injects the provenance lines; you write the heading and body only.
3. **`design.md`** — domain model, APIs, integrations, configuration, technical logic, target-idiom folding (provider DI / Crux idioms / contract format choice), and the UI / layout subsection that spatial Evidence (`region` / `container` / `leaf` from the `screenshots` source adapter) folds into.
4. **`tasks.md`** — implementation sequencing as plain markdown checkboxes (`- [ ] …`).

See [`substeps.md`](substeps.md) for the per-section contract.

## What each file in this playbook owns

| File                                       | Owns                                                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------ |
| [`substeps.md`](substeps.md)               | Per-section contract for the response; what each artifact's prose MUST contain.            |
| [`authority.md`](authority.md)             | Authority hierarchy, the per-slice override on `plan.yaml` (per-Evidence per-kind overrides are deferred), the kernel's resolution order, and the agent's `agreement` verdict → kernel `status` derivation table. |
| [`requirement-block.md`](requirement-block.md) | The agent-authored requirement prose plus the kernel-rendered `spec.md` block it projects into, per `status` variant. |
| [`claim-reconciliation.md`](claim-reconciliation.md)       | How to reconcile per-`kind` and per-`authority` claims into the response; where each claim kind lands. |
| [`provenance.md`](provenance.md)                   | Provenance projection (`specify slice provenance`, inline in `model.yaml`): block grammar per `resolution` enum value, inline `value` truncation, `winner` markers, and `resolution-trace` step names. |
| [`tags.md`](tags.md)                       | Tag grammar (`[unknown]` / `[conflict]` / `[divergence]`) and the tag ↔ `status` coherence the kernel renders. |

## Posture

Uncertainty produces review tags rather than parking the slice. The slice lifecycle stays `refining → refined → built → merged` regardless of how many `[unknown]` / `[conflict]` / `[divergence]` tags the kernel derives. The agent records the `agreement` verdict; the kernel derives the `status` and tag from it. Operators reconcile by re-running `/spec:refine` (or amending `plan.yaml.slices[].authority-override`) after the slice transitions; the playbook's job is to surface the disagreement honestly, not to second-guess it.
