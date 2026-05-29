# Tag grammar

Synthesis surfaces three review-signal tags in `spec.md`. Each tag appears inside the requirement-block headline, after the human-readable name, separated by a single space. Tags never park the slice — they document uncertainty so the operator can reconcile by hand-editing `spec.md` after the slice transitions to `refined`.

## Closed tag set

| Tag             | Mirrors `Status:` | Meaning                                                                          | Operator action                                            |
| --------------- | ----------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `[unknown]`     | `unknown`         | No contributing Evidence supplied a claim for this requirement.                  | Add a source via `specrun plan amend --add-source` and re-refine, or hand-edit. |
| `[conflict]`    | `conflict`        | Multiple sources at the same authority class disagree; no winner.                | Hand-edit `spec.md` to pick a value and flip `Status: agreed`, or amend the plan to drop the losing source. |
| `[divergence]`  | `divergence`      | Multiple sources disagree, but one wins by authority class (`intent > documentation > behaviour`). | Hand-edit `spec.md` if the authority-resolved winner is wrong, otherwise proceed. |

Headline shape:

```markdown
### Requirement: <Name> [<tag>]
```

One tag per headline. Tags do not stack — a requirement is in at most one of the three states. `Status: agreed` carries no tag; the headline ends at the requirement name.

## Coherence rule

The W1.3 provenance parser refuses output where the headline tag and `Status:` field disagree. Synthesis MUST enforce the mirror exactly:

| Headline                                     | Required `Status:` |
| -------------------------------------------- | ------------------ |
| `### Requirement: <Name>` (no tag)           | `agreed`           |
| `### Requirement: <Name> [unknown]`          | `unknown`          |
| `### Requirement: <Name> [conflict]`         | `conflict`         |
| `### Requirement: <Name> [divergence]`       | `divergence`       |

A headline tag without the matching `Status:` (or vice versa) is a parser failure that keeps the slice in `refining`. The skill body refuses to transition until validation passes.

## Per-tag body conventions

### `[unknown]`

The body is a single line stating the gap:

```markdown
No contributing source supplied a claim for this requirement. Operator review required.
```

`Sources: []` is the only legal `Sources:` value for `Status: unknown`.

### `[conflict]`

The body carries only `Note:` lines (one per contributing source value) plus an operator-reconciliation prompt:

```markdown
Note: product-notes says reset links expire after 30 minutes.
Note: identity-design-notes says reset links expire after 60 minutes.

Operator reconciliation required before /spec:build.
```

No operative body sentence — picking a value is the operator's job. `Sources:` lists every contributing key (alphabetical within the tied authority class).

### `[divergence]`

The body carries the authority-resolved winning value as the operative requirement, followed by one `Note:` line per losing source preserving its observation:

```markdown
The system expires password reset links after 30 minutes. (from identity-design-notes; documentation)

Note: legacy-monolith observed 24-hour expiry; the documentation authority overrides. Operator review recommended.
```

`Sources:` lists every contributing key, winner first.

## Journal-event hand-off

Each line appended to `.specify/journal.jsonl` must be one JSON object, newline-terminated, with kebab-case keys only — no snake_case field names. Wire shape is adjacency-tagged `{ timestamp, event, payload }` (see the worked line in [`../../skills/plan/fixtures/divergence-journal/journal.jsonl`](../../skills/plan/fixtures/divergence-journal/journal.jsonl)).

For each requirement block written with a `[unknown]` / `[conflict]` / `[divergence]` tag, `specrun slice validate` (step 6 of `/spec:refine`) appends one journal event after validation succeeds:

| Tag             | Event id                       | Payload                                  |
| --------------- | ------------------------------ | ---------------------------------------- |
| `[unknown]`     | `slice.synthesis.unknown`      | `{ slice-name, requirement-id }`         |
| `[conflict]`    | `slice.synthesis.conflict`     | `{ slice-name, requirement-id }`         |
| `[divergence]`  | `slice.synthesis.divergence`   | `{ slice-name, requirement-id }`         |

The event is the durable hand-off `/spec:execute` and downstream review tooling consume to surface synthesis tags at loop boundaries. The journal event is emitted regardless of whether the operator subsequently reconciles by hand-editing `spec.md`.

## Anti-patterns

- **Stacked tags** (`[divergence][unknown]`) — illegal; pick the dominant state.
- **Tags on `proposal.md` / `design.md` / `tasks.md` headings** — synthesis tags only appear on `spec.md` requirement headlines.
- **Tag without provenance** — `### Requirement: Foo [conflict]` with no `Sources:` line below fails the parser; every tagged requirement still carries the three provenance lines.
- **Auto-resolving `[conflict]`** — synthesis never picks a winner when authorities tie. The operator reconciles.
- **Suppressing `[unknown]` for empty Evidence** — a lead whose Evidence emits `claims: []` legitimately produces `[unknown]` requirements; do not silently omit them.
