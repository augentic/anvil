# Tag grammar

Three review-signal tags render into `spec.md` from each requirement's reconciliation row. Each appears after the human-readable name, separated by a single space. Tags never park the run — they document uncertainty inline so the operator can reconcile and re-run `emery specify`.

## Closed tag set

| Tag | Mirrors `Status:` | Meaning | Operator action |
| --- | ----------------- | ------- | --------------- |
| `[unknown]` | `unknown` | No evidenced acceptance behaviour | Bind a source that evidences it; re-run |
| `[conflict]` | `conflict` | Same-class disagreement; no winner | Amend or drop a source; re-run |
| `[divergence]` | `divergence` | Disagreement; one wins by authority (`intent > documentation > behaviour`) | Override the source set if the winner is wrong; otherwise proceed |

Headline: `### Requirement: <Name> [<tag>]`. One tag per headline; no stacking. `Status: agreed` carries no tag.

## Coherence rule

The parser refuses output where the headline tag and `Status:` disagree:

| Headline | Required `Status:` |
| -------- | ------------------ |
| no tag | `agreed` |
| `[unknown]` / `[conflict]` / `[divergence]` | matching status |

## Per-tag body conventions

- **`[unknown]`** — a gap statement (what exists or is mentioned, that behaviour is not evidenced) plus ≥1 WHEN/THEN scenario that does not invent behaviour. `Sources: []` only.
- **`[conflict]`** — only `Note:` lines (one per source value) plus `Note: Operator reconciliation required.` No operative body sentence.
- **`[divergence]`** — winning value as operative body; one `Note:` per loser.

## Anti-patterns

Stacked tags; tags on non-requirement headings; a tag without its provenance lines; auto-resolving `[conflict]`; suppressing `[unknown]` when acceptance evidence is absent.
