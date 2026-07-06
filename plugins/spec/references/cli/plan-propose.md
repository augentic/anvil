# Lead reconciliation (inside `specify plan author`)

The reconcile leg inside the guest-routed `specify plan author` groups the surveyed `discovery.md` leads into the plan's `slices[]` rows (the standalone two-phase envelope verb retired at the Omnia-migration cutover).

- The **request** side is a flat catalog of raw `(source, lead)` leads read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter).
- The **write** side is the **only slice writer**. It schema-validates the judgment response (`proposal-schema`), re-reads `discovery.md`, rebuilds the lead catalog, validates the agent's `slices[]` grouping, enforces total lead coverage, validates explicit slice names, binds projects, atomically replaces `plan.yaml.slices[]`, then emits `plan.reconcile.completed`. It never trusts a stale snapshot.

**Replaceable gate.** The write runs only while the plan is replaceable — `lifecycle: pending` and every entry `pending`; otherwise `plan-reconcile-plan-not-replaceable`.

Validation codes (all exit 2):

| Code | Meaning |
|------|---------|
| `proposal-schema` | The judgment response failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `discovery.md` surfaced no leads to reconcile. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | Grouped leads do not achieve total coverage — a surveyed lead is referenced by no slice. |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same plan slice name. |
| `plan-reconcile-depends-on-cycle` | Projected `depends-on` edges form a cycle. |
| `plan-reconcile-project-binding-required` | A slice omits `project` when more than one project exists. |
| `plan-reconcile-project-orphan` | A slice binds a `project` absent from the request topology. |
| `plan-reconcile-plan-not-replaceable` | The plan is approved or carries a non-pending entry. |

Advisory findings (non-blocking; the write still succeeds, exit 0):

| Code | Meaning |
|------|---------|
| `lead-decision-topic-overlap` | A surveyed lead's `topics[]` overlaps an accepted decision's `topics[]` on the slice's bound project. Review nudge: confirm the slice aligns with that decision (or record a superseding one). Latent until both leads and decisions carry topics. |
| `slice-divergence-unrecorded` | A slice flags `divergence: likely`/`accepted` but records no adequate `disagreements[]` (≥2 distinct source values per field). |
| `slice-divergence-orphan-values` | A slice records `disagreements[]` without a `divergence` flag. |
| `greenfield-seed-shadowed` | A bound project still declares a `registry.yaml` `greenfield_seed` after acquiring a baseline (`.specify/specs/` exists); the derived `surface[]` supersedes the seed — remove it. |

Envelopes validate against `schemas/discovery/proposal.schema.json` (closed `kind: request | response`). Full CLI reference: [specify plan](https://specify.augentic.io/reference/cli/plan.html).
