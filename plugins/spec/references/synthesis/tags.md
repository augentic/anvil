# Tag grammar

The kernel renders three review-signal tags into `spec.md` from each requirement's `status` (derived in turn from the agent's `agreement` verdict and the resolved authority — see [`authority.md`](authority.md)). Each tag appears inside the requirement-block headline, after the human-readable name, separated by a single space. Tags never park the slice — they document uncertainty so the operator can reconcile by re-running `/spec:refine` (with a different verdict or amended `authority-override`) after the slice transitions to `refined`.

## Closed tag set

| Tag             | Mirrors `Status:` | Meaning                                                                          | Operator action                                            |
| --------------- | ----------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `[unknown]`     | `unknown`         | No contributing Evidence supplied a claim for this requirement.                  | Add a source via `specify plan amend --add-source` and re-run `/spec:refine`. |
| `[conflict]`    | `conflict`        | Multiple sources at the same authority class disagree; no winner.                | Pin the winner via `specify plan amend --authority-override` (or amend the plan to drop the losing source), then re-run `/spec:refine`. |
| `[divergence]`  | `divergence`      | Multiple sources disagree, but one wins by authority class (`intent > documentation > behaviour`). | If the authority-resolved winner is wrong, pin the right source via `specify plan amend --authority-override` and re-run `/spec:refine`; otherwise proceed. |

Headline shape:

```markdown
### Requirement: <Name> [<tag>]
```

One tag per headline. Tags do not stack — a requirement is in at most one of the three states. `Status: agreed` carries no tag; the headline ends at the requirement name.

## Coherence rule

The W1.3 provenance parser refuses output where the headline tag and `Status:` field disagree. The kernel renders the mirror exactly, so a divergent pair only arises from a post-synthesis hand-edit:

| Headline                                     | Required `Status:` |
| -------------------------------------------- | ------------------ |
| `### Requirement: <Name>` (no tag)           | `agreed`           |
| `### Requirement: <Name> [unknown]`          | `unknown`          |
| `### Requirement: <Name> [conflict]`         | `conflict`         |
| `### Requirement: <Name> [divergence]`       | `divergence`       |

A headline tag without the matching `Status:` (or vice versa) is a parser failure that keeps the slice in `refining`. The skill body refuses to transition until validation passes.

## Per-tag body conventions

The agent authors the body prose (the requirement `statement` and any `notes` rendered as `Note:` lines); the kernel renders the headline tag from `status`. The shapes below are the agent's body for each derived `status`.

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

Each line appended to `.specify/journal.jsonl` must be one JSON object, newline-terminated, with kebab-case keys only — no snake_case field names. Wire shape is adjacency-tagged `{ timestamp, event, payload }` (see the worked line in [plan divergence journal fixture](https://github.com/augentic/specify/blob/main/plugins/spec/skills/plan/fixtures/divergence-journal/journal.jsonl)).

For each requirement block written with a `[unknown]` / `[conflict]` / `[divergence]` tag, `specify slice validate` (step 6 of `/spec:refine`) appends one journal event after validation succeeds:

| Tag             | Event id                       | Payload                                  |
| --------------- | ------------------------------ | ---------------------------------------- |
| `[unknown]`     | `slice.synthesis.unknown`      | `{ slice-name, requirement-id }`         |
| `[conflict]`    | `slice.synthesis.conflict`     | `{ slice-name, requirement-id }`         |
| `[divergence]`  | `slice.synthesis.divergence`   | `{ slice-name, requirement-id }`         |

The event is the durable hand-off `/spec:execute` and downstream review tooling consume to surface synthesis tags at loop boundaries. The journal event is emitted regardless of whether the operator subsequently reconciles by re-running `/spec:refine` (after amending an `authority-override` or the source set).

## Anti-patterns

- **Stacked tags** (`[divergence][unknown]`) — illegal; pick the dominant state.
- **Tags on `proposal.md` / `design.md` / `tasks.md` headings** — synthesis tags only appear on `spec.md` requirement headlines.
- **Tag without provenance** — `### Requirement: Foo [conflict]` with no `Sources:` line below fails the parser; every tagged requirement still carries the three provenance lines.
- **Auto-resolving `[conflict]`** — the kernel never picks a winner when authorities tie. The operator reconciles.
- **Suppressing `[unknown]` for empty Evidence** — a lead whose Evidence emits `claims: []` legitimately produces `[unknown]` requirements; do not silently omit them.
