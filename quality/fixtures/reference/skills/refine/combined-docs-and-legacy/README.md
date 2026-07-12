# Fixture: combined documentation + legacy

Multi-source slice where a `documentation` Evidence and a `typescript` Evidence agree on a `id` after reconciliation. Matches workflow §Worked multi-source `plan.yaml` (slice `identity-user-registration`).

Playbook rules exercised:

- [`claim-reconciliation.md`](../../../../../../plugins/spec/references/synthesis/claim-reconciliation.md) — reconciliation on `id` (`users.register.email-validation`); behaviour `excerpt` corroborates the `documentation` `requirement`.
- [`authority.md`](../../../../../../plugins/spec/references/synthesis/authority.md) — multiple sources agree → `Status: agreed`, both keys in `Sources:`, highest authority (`documentation`) first.
- [`substeps.md`](../../../../../../plugins/spec/references/synthesis/substeps.md) — design.md folds in the `excerpt` and `type` claims under `## Domain model` / `## Technical logic`, plus the Omnia `shape` brief's provider DI shape.

Slice: `identity-user-registration`, target `omnia`.
