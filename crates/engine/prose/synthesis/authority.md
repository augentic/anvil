# Authority hierarchy

Every Evidence document carries one closed `authority` class. Highest wins:

1. **`intent`** — inline operator directives (the `intent` source adapter is the only first-party emitter).
2. **`documentation`** — operator-provided written product or technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` source adapter.
3. **`behaviour`** — what legacy code actually does. Emitted by behaviour sources such as `typescript` and future code or observation adapters.

The **engine** resolves authority before you are called; the requirement rows carry the outcome. You never pick winners, derive `Status:`, or order `Sources:` — you draft honest content for the rows as they stand.

## Status derivation (engine-computed)

A requirement's contributing claims were grouped into agreeing classes. The engine ranks the classes by authority: one class is `agreed`; a unique highest-authority class winning over lower ones is `divergence`; two classes tied at the top authority are an unresolvable `conflict`. A requirement with no acceptance criterion in evidence is uncovered.

| Contributing classes | `Status:` | Tag |
| -------------------- | --------- | --- |
| 1, covered | `agreed` | (none) |
| 1, uncovered | `unknown` | `[unknown]` |
| ≥2, unique top authority | `divergence` | `[divergence]` |
| ≥2 at the same top authority | `conflict` | `[conflict]` |

An uncovered `divergence` or `conflict` row keeps its tag; the engine adds the gap note beneath its loser notes.

## What you draft per resolution

- **`agreed`** / **`unknown`** — the shared statement as the operative body: quote documentation language lightly normalised, paraphrase behaviour into present-tense system prose.
- **`divergence`** — the winning class's statement as the operative body. Do not mention the losers: the engine renders one `Note:` per losing class from their verbatim statements.
- **`conflict`** — no body at all (`"body": []`). The engine renders one `Note:` per class and `Note: Operator reconciliation required.`; you draft only the scenario, which must not pick a side.
