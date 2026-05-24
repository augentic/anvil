# Fixture: `[conflict]`

Two `documentation` Evidence contribute claims with the same `claim-id` but disagreeing bodies. Authority is tied at `documentation` — synthesis cannot pick a winner, so the requirement carries `[conflict]` and preserves both values for operator reconciliation. Matches workflow §Acceptance scenario #5c.

Playbook rules exercised:

- [`authority.md`](../../../../../plugins/spec/references/synthesis/authority.md) — tied top authority → `Status: conflict`, `[conflict]` tag, both values as `Note:` lines.
- [`tags.md`](../../../../../plugins/spec/references/synthesis/tags.md) — coherence between headline tag and `Status:`; the `slice.synthesis.conflict` journal event the skill body emits.
- [`requirement-block.md`](../../../../../plugins/spec/references/synthesis/requirement-block.md) — body shape for `[conflict]` (only `Note:` lines plus the operator-reconciliation prompt).

Slice: `identity-password-reset-expiry`, target `omnia`.
