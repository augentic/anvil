# Fixture: `[divergence]`

Two sources contribute claims with the same `claim-id` but contradictory bodies; authority hierarchy (`documentation > behaviour`) picks the winner. Mirrors workflow §Per-requirement provenance variants — `[divergence]` from authority resolution.

Playbook rules exercised:

- [`authority.md`](../../../../../plugins/spec/references/synthesis/authority.md) — multi-source disagreement with strict-greater authority on one side → `Status: divergence`, `[divergence]` tag, loser as `Note:` commentary.
- [`tags.md`](../../../../../plugins/spec/references/synthesis/tags.md) — tag/`Status` coherence; the `slice.synthesis.divergence` journal event the skill body emits.
- [`claim-fusion.md`](../../../../../plugins/spec/references/synthesis/claim-fusion.md) — `excerpt` claim becomes commentary on a `[divergence]` block when the documentation `requirement` overrides it.

Slice: `identity-password-reset`, target `omnia`. `documentation` says expiry is 30 minutes; `code-typescript` observed 24 hours.
