# Claim landing

Where each closed claim kind's content belongs across the two artifacts. Grouping and resolution are already done when you are called (see [authority.md](authority.md)); this file tells you where each kind's content lands.

## Per-kind landing

| Kind | Lands in | Key |
| ---- | -------- | --- |
| `requirement` | `spec.md` — one requirement row per group of claims | `id` (required) |
| `criterion` | the spec row whose contributing ids it equals or extends → `#### Scenario:` | `id` (required) |
| `intent` | `spec.md` headline requirement when it names a behaviour; also the `design.md` overview | none |
| `decision` | `design.md` under the section it informs; cite `(from <source>)` | none |
| `section` | `design.md` relevant section | none |
| `excerpt` | `design.md` `technical-logic` | optional `id` |
| `type` | `design.md` `domain-model` — one `type` block per claim; the engine inserts the `signature` verbatim | optional `id` |
| `call` | `design.md` `apis` (the section plan requires it); internal delegation may also inform `technical-logic` | optional `id` |
| `example` | `spec.md` scenario via matching `id` prefix; `design.md` references keep `path` | required `id` |
| `region` / `container` / `leaf` | `design.md` `ui-layout` tree; never `spec.md` requirements | none (positional) |
| `diagram` / `contract` | `design.md` relevant section | none |

## Requirement identity

Byte-equal `requirement` ids are always one requirement; beyond that, the engine's grouping judgement decided which claims across sources describe one requirement and which agree. The row's subject is the highest-authority contributor's id; the other contributors' ids appear in the loser notes the engine renders. You never regroup, rename, or split a row.

A `criterion` whose id equals or extends any contributing id (`<id>.<rest>`) covers the requirement. A requirement no criterion covers is an evidence gap: the engine tags it `[unknown]` when it is otherwise agreed and renders `Note: acceptance criteria not evidenced.`; you still draft a scenario that does not invent behaviour.

## Order and stability

- Requirements appear in row order; the engine minted `REQ-NNN` ids in that order. Your draft is keyed by subject, never by position.
- Re-running over identical claims must produce byte-identical artifacts: no timestamps, counters, or free-running variation.
