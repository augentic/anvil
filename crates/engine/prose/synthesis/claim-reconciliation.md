# Claim reconciliation

How the closed claim kinds group and where each lands across the two artifacts. Grouping and resolution are engine-computed (see [authority.md](authority.md)); this file tells you where each kind's content belongs.

## Per-kind landing

| Kind | Lands in | Key |
| ---- | -------- | --- |
| `requirement` | `spec.md` — one block per `id` group | `id` (required) |
| `criterion` | the spec block whose requirement id prefixes its own → `#### Scenario:` | `id` (required) |
| `intent` | `spec.md` headline requirement when it names a behaviour; also the `design.md` overview | none |
| `decision` | `design.md` under the H2 it informs; quote `(from <source>)` | none |
| `section` | `design.md` relevant H2 | none |
| `excerpt` | `design.md` `## Technical logic`; contributes to `spec.md` only through its reconciliation row | optional `id` |
| `type` | `design.md` `## Domain model` — `signature` verbatim | optional `id` |
| `call` | `design.md` `## APIs and integrations` (the section plan requires it); internal delegation may also inform `## Technical logic` | optional `id` |
| `example` | `spec.md` scenario via matching `id` prefix; `design.md` references keep `path` | required `id` |
| `region` / `container` / `leaf` | `design.md` `## UI / layout` tree; never `spec.md` blocks | none (positional) |
| `diagram` / `contract` | `design.md` relevant H2 | none |

## Grouping on `id`

`requirement` and `criterion` claims carry dotted-kebab `id`s. The engine groups every contributing claim by exact `id` across all sources — that is the cross-source key. A `criterion` whose id extends a requirement's id (`<requirement-id>.<rest>`) supplies that requirement's acceptance scenario. A requirement group with no such criterion is an evidence gap: the engine appends one `[unknown]` acceptance-criteria row for it, and you write the gap block per [tags.md](tags.md).

## Order and stability

- Requirements appear in reconciliation-row order; the engine minted `REQ-NNN` ids in that order.
- `Sources:` lists every contributing key, highest authority class first, binding order within a class — exactly as the row states.
- Re-running over identical claims must produce byte-identical artifacts: no timestamps, counters, or free-running variation.
