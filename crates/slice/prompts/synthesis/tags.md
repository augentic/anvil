# Tag grammar

The kernel renders three review-signal tags into `spec.md` from each requirement's `status` (see [`authority.md`](authority.md)). Each tag appears after the human-readable name, separated by a single space. Tags never park the slice — they document uncertainty so the operator can reconcile (amend the override or sources, then re-run `emery plan execute` to re-refine) after `refined`.

## Closed tag set

| Tag | Mirrors `Status:` | Meaning | Operator action |
| --- | ----------------- | ------- | --------------- |
| `[unknown]` | `unknown` | No contributing Evidence claim | Add a source via `emery plan amend --add-source`; re-run `emery plan execute` |
| `[conflict]` | `conflict` | Same-class disagreement; no winner | Pin via `emery plan amend --authority-override` (or drop a source); re-run `emery plan execute` |
| `[divergence]` | `divergence` | Disagreement; one wins by authority (`intent > documentation > behaviour`) | Override if the winner is wrong; otherwise proceed |

Headline: `### Requirement: <Name> [<tag>]`. One tag per headline; no stacking. `Status: agreed` carries no tag.

## Coherence rule

The provenance parser refuses output where the headline tag and `Status:` disagree (hand-edit only — the kernel renders the mirror exactly):

| Headline | Required `Status:` |
| -------- | ------------------ |
| no tag | `agreed` |
| `[unknown]` / `[conflict]` / `[divergence]` | matching status |

## Per-tag body conventions

Agent authors `statement` and `notes` (`Note:` lines); kernel renders the tag from `status`.

- **`[unknown]`** — gap statement (what exists / is mentioned, that behaviour is not evidenced) plus ≥1 WHEN/THEN scenario that does not invent behaviour. `Sources: []` only.
- **`[conflict]`** — only `Note:` lines (one per source value) plus `Operator reconciliation required before the build phase.` No operative body sentence. `Sources:` lists every contributing key.
- **`[divergence]`** — winning value as operative body; one `Note:` per loser. `Sources:` winner first.

## Journal-event hand-off

For each tagged requirement, `emery slice validate` appends one `slice.synthesis.{unknown|conflict|divergence}` event after validation succeeds. Record verdicts honestly so the events surface real gaps.

## Anti-patterns

- Stacked tags; tags on non-spec headings; tag without provenance lines; auto-resolving `[conflict]`; suppressing `[unknown]` when Evidence is empty.
