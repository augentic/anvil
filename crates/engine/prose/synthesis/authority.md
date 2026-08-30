# Authority hierarchy

Every Evidence document carries one closed `authority` class. Highest wins:

1. **`intent`** — inline operator directives bound at `emery specify --description`.
2. **`documentation`** — written product or technical intent the operator supplied.
3. **`behaviour`** — what legacy code actually does (code, captures, observation sources).

The **engine** resolves authority deterministically before you are called; the reconciliation rows carry the outcome. You never pick winners, derive `status`, or reorder `Sources:` — you render the rows and write honest prose around them.

## Status derivation (engine-computed)

| Contributing claims | Values | `Status:` | Tag |
| ------------------- | ------ | --------- | --- |
| 1, or ≥2 that agree | match | `agreed` | (none) |
| ≥2, unique top authority disagrees with a lower one | differ | `divergence` | `[divergence]` |
| ≥2 at the same top authority | differ | `conflict` | `[conflict]` |
| requirement with no acceptance criterion in evidence | — | `unknown` | `[unknown]` |

## Body conventions per resolution

- **`divergence`** — the winning (highest-authority) statement is the operative body; each losing value survives as one `Note:` line naming its source and class. Never delete the loser.
- **`conflict`** — no operative body sentence: one `Note:` line per contributing value, then `Note: Operator reconciliation required.` Never pick a side.
- **`agreed`** — the shared statement, lightly normalised; quote documentation language where possible, paraphrase behaviour into present-tense system prose.

### Worked divergence

Docs say sessions expire after 30 minutes; code observes 15. Documentation outranks behaviour:

```markdown
### Requirement: Session timeout [divergence]

ID: REQ-001
Sources: [docs, code]
Status: divergence

The system expires idle sessions after 30 minutes. (from docs; documentation)

Note: code observed 15-minute expiry; the documentation authority overrides.
```
