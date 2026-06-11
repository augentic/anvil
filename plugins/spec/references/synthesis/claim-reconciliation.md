# Claim reconciliation

How the agent groups claims across `Evidence[]` into the synthesis response and where each claim kind lands in the four artifacts. Grouping and the `agreement` verdict are the agent's; the kernel resolves authority, derives `status`, marks winners, and renders the `Sources:` list.

## Per-kind reconciliation

The closed `kind` enum (from `schemas/evidence.schema.json`) groups into four bands:

| Kind            | Carrying authority class    | Where it lands                                                                                  | Reconciliation key                                |
| --------------- | --------------------------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `requirement`   | `documentation`             | spec files (`specs/<domain>/spec.md`) — one requirement block per `id` group.               | `id` (required by the schema).      |
| `criterion`     | `documentation`             | spec files — folds into the requirement block whose `id` shares the same `<requirement>.*` prefix as a `#### Scenario:` H4 inline within that block; when no requirement prefix matches, attaches to the nearest requirement by source order. | `id` (required by the schema).      |
| `decision`      | `documentation`             | `design.md` — under the H2 the decision informs (transport → APIs; error strategy → Technical logic; provider choice → Configuration). Quote verbatim with `(from <source>)`. | None (free-form; not reconciled).              |
| `section`       | `documentation`             | `design.md` — folded as context under the most relevant H2; or `proposal.md` `## Why` when the section names the slice's *why*. | None (free-form; not reconciled).              |
| `excerpt`       | `behaviour`                 | Primarily `design.md` `## Technical logic` (paraphrased). When no other source contributes a requirement on the same behaviour, also drives a `spec.md` requirement with `Status: agreed`. When a `documentation` claim contradicts, becomes commentary on the resulting `[divergence]` block. | Optional `id`; fall back to grouping by handler name extracted from `path`. |
| `type`          | `behaviour`                 | `design.md` `## Domain model` — render the `signature` field verbatim as the type's canonical shape. | Optional `id`; fall back to the type name from `signature`. |
| `call`          | `behaviour`                 | `design.md` `## APIs and integrations` (external surfaces) or `## Technical logic` (internal delegation). | Optional `id`; fall back to `callee`. |
| `example`       | `behaviour`                 | `spec.md` — folds into the requirement block whose `id` shares the same prefix as the example's `id` (e.g. `users.register.happy-path` corroborates the `users.register` requirement); when no requirement prefix matches, drives its own `spec.md` requirement with `Status: agreed`. `design.md` `## Technical logic` references the fixture path for the operator to inspect concrete I/O. | Required `id` (per the per-kind body shape owned by `adapters/sources/captures/briefs/extract.md`). |
| `region`        | `documentation` (spatial)   | `design.md` `## UI / layout` — top-level layout regions per screen.                             | None (positional; not reconciled).             |
| `container`     | `documentation` (spatial)   | `design.md` `## UI / layout` — grouping within a region.                                        | None (positional; not reconciled).             |
| `leaf`          | `documentation` (spatial)   | `design.md` `## UI / layout` — individual UI element.                                           | None (positional; not reconciled).             |
| `intent`        | `intent`                    | `proposal.md` `## Why` (primary) and one headline `spec.md` requirement when the intent names a behaviour. | None (one claim per Evidence).            |
| `diagram`       | source-dependent            | `design.md` under the most relevant H2.                                                         | None.                                     |
| `contract`      | source-dependent            | `design.md` `## APIs and integrations`.                                                         | None.                                     |

### Deterministic reconciliation on `id`

`requirement` and `criterion` claims MUST carry `id` (enforced by `schemas/evidence.schema.json`). The agent groups every contributing claim by exact `id` match across all Evidence documents — that is the cross-source reconciliation key.

- All claims sharing one `id` collapse into one requirement, carrying every contributing `(source, id, kind)` claim.
- The kernel renders the `Sources:` list from those claims, highest authority first.
- The kernel derives `status` from the claim count, the agent's `agreement` verdict, and the resolved authority (see [`authority.md`](authority.md)).

When two contributing claims share `id` and their `statement:` / `criterion:` strings *agree* (after trivial whitespace normalisation), record `agreement: agreed`; the kernel renders the shared text with `Status: agreed`. When they *disagree*, record `agreement: disagreed` and let the kernel apply the per-authority resolution below.

### Behaviour claims as corroboration

`excerpt`, `type`, `call`, and `example` claims (authority class: `behaviour` by default) primarily drive `design.md`. They contribute to `spec.md` in two ways:

- **Standalone source** — when no other source supplied a `requirement` claim on the same behavioural surface, an `excerpt` whose paraphrase reads as a single behavioural assertion, or an `example` whose captured `input` / `output` pair reads as one, becomes a requirement carrying that single claim; the kernel derives `Status: agreed` (single-source).
- **Authority-loser** — when a `documentation` `requirement` contradicts an `excerpt` or `example`, record `agreement: disagreed`; the kernel resolves the `documentation` claim as the winner per the default ordering, and the behaviour-class claim survives with `winner: false` (rendered as the `Note:` line of the `[divergence]` block — see [`authority.md`](authority.md)). Operators flip that default per slice via per-slice `authority-override` — useful exactly when runtime captures should outrank stale docs.

### `example` claims from `captures`

`example` claims are emitted by the `captures` source adapter from captured request/response data. They share the `behaviour` authority class with `excerpt` and `call` claims, and they tie-break the same way:

- **Default precedence vs other behaviour-class claims.** `example`, `excerpt`, and `call` are siblings at the same authority class. Operators tie-break across them via per-slice `authority-override.<kind>` on `plan.yaml` (see [`authority.md` §Per-slice overrides](authority.md#per-slice-overrides-on-planyaml)). The synthesis playbook does not silently prefer one over another.
- **Per-Evidence override.** A `captures` Evidence document MAY emit `authority-overrides: { example: documentation }` to lift its `example` claims above the document-level `behaviour` default — rare, but useful when the captured data encodes an explicit contract the operator wants treated as documentation-class.
- **Per-kind body.** `example` claims carry `id`, `path` (the on-disk capture anchor), `replay-digest` (a `sha256:` content anchor over the capture bytes), `input` (the captured request shape), `output` (the captured response and side-effect shape), and an optional `statement:` line that paraphrases the example for prose use. The per-kind body shape is owned by `adapters/sources/captures/briefs/extract.md` (the current captures reference); refer to that brief rather than mirroring the fields here.

### Spatial claims fold into design

`region` / `container` / `leaf` claims describe layout, not behaviour. They land under `design.md` `## UI / layout` as a single tree per screen, preserving the spatial nesting from the Evidence. The Vectis target's `build` brief reads this section to regenerate `composition.yaml`; targets that do not consume spatial Evidence (Omnia, contracts) omit the H2 entirely. Spatial claims never produce `spec.md` requirements directly — behavioural assertions that *use* spatial structure live as separate `requirement` claims on a contributing `documentation` Evidence.

### Intent drives proposal

The `intent` adapter emits exactly one `intent` claim per Evidence (per the W2.1 contract). Synthesis:

- Renders the `statement` verbatim as the heart of `proposal.md` `## Why`.
- If the statement names a behaviour ("Add a search filter to the user list"), also records one headline requirement citing the `intent` claim; the kernel assigns `REQ-001` and derives `Status: agreed` (single-source).
- Pure-intent slices (the degenerate `[intent]` case) produce a spec with at most one requirement — additional requirements only appear when other sources contribute.

## Per-authority resolution (slice-time)

When a reconciled `id` group the agent marked `disagreed` carries claims from multiple authorities, the kernel's [§Resolution order](authority.md#resolution-order) picks the winner and derives the `status` — that reference is canonical for the precedence (`intent > documentation > behaviour`) and the override surfaces; nothing here re-orders it. What lands in the rendered block per outcome:

- **Strict-greater authority → `Status: divergence`.** A `documentation` `requirement` of "30 minutes" and a `typescript` `excerpt` of "24 hours" resolves to `Status: divergence`, body carries the 30-minute value, `Note:` line preserves the 24-hour observation. A per-slice override pinning `legacy-monolith` as the criterion winner flips the body and the `Note:` line without changing the `Status: divergence` posture.
- **Tied authority (same class on both sides) → `Status: conflict`.** Two `documentation` Evidence disagreeing on a `id`'s `statement` is a `[conflict]` unless a per-slice override breaks the tie. Two `behaviour` Evidence disagreeing on an `excerpt` paraphrase (or `example` capture) is a `[conflict]` unless a per-slice override picks the winning source.
- **Agreement at the same authority → `Status: agreed`.** Two `documentation` Evidence agreeing on a `id`'s `statement` collapses to one block with both keys in `Sources:`. Agreement after override resolution (e.g. per-slice override picks one source but every contributor's value matches) also lands as `Status: agreed`.

## Order and stability

- The kernel renders `Sources:` deterministically: sort by authority class (`intent` < `documentation` < `behaviour`), then alphabetically by source key within a class, highest-authority key first.
- Order requirements in the response by source order on the highest-authority Evidence document (when tied, fall back to alphabetical order on the first contributing source key); the kernel assigns `REQ` ids and renders the spec blocks in that declaration order.
- Re-running `/spec:refine` on identical `Evidence[]` and `shape` MUST produce byte-identical artifacts: the kernel is a deterministic, target-independent projection over a fixed response and emits no timestamps into the artifacts.

## Plan-time reconciliation is a separate playbook

Plan-time `Lead[]` reconciliation — the `propose` sub-step inside `/spec:plan` that writes `slices[]` rows — is documented by [`specify plan propose`](../cli/plan-propose.md). Cross-source matching is agent judgment; the kernel validates partition shape only. The operator curates at Gate 1 via `change.md` and `specify plan amend`.
