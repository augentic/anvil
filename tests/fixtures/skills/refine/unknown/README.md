# Fixture: `[unknown]`

Single-source slice whose `documentation` extract returned `claims: []` (the candidate exists in `discovery.md` but the extract brief could not resolve any docs for it — empty-but-valid per workflow §Extraction reliability "Empty / Invalid Evidence"). Synthesis still surfaces the requirement gap as `[unknown]`.

Playbook rules exercised:

- [`authority.md`](../../../../../plugins/spec/references/synthesis/authority.md) — no contributing claims → `Status: unknown`, `[unknown]` tag.
- [`tags.md`](../../../../../plugins/spec/references/synthesis/tags.md) — coherence between headline and `Status:`; `Sources: []` is legal only on `Status: unknown`; `slice.synthesis.unknown` journal event.
- [`substeps.md`](../../../../../plugins/spec/references/synthesis/substeps.md) — synthesis still authors `proposal.md` and a placeholder `spec.md` block so the operator sees the gap; lifecycle reaches `refined` cleanly.

Slice: `audit-trail-retention`, target `omnia`. The candidate exists in `discovery.md` but no docs under the bound source directory matched.
