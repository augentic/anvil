# RFC-52: Effect Map

> Status: Draft · Order 2 of 10 · Stage S2 · Depends: [RFC-51](rfc-51-adapter-wit.md) · Enables: [RFC-53](rfc-53-wasi-model.md), [RFC-55](rfc-55-working-tree.md), [RFC-56](rfc-56-runtime-move.md) · Owns: the typed effect vocabulary and ownership map

## Abstract

Every capability a guest needs from the outside is a typed WIT effect it imports or a typed export another guest provides. This RFC names the effect vocabulary and assigns implementation ownership. It is intentionally a map, not the implementation plan for every effect.

## Effect ownership

| Effect | Role | Owner |
| ------ | ---- | ----- |
| `wasi:filesystem` | Capability-scoped access to input artifacts, assets, and materialized project trees | Generic host binding in [RFC-56](rfc-56-runtime-move.md); working-tree backend in [RFC-55](rfc-55-working-tree.md) |
| `wasi:keyvalue` | Host-held scratch and session state | Generic host binding in [RFC-56](rfc-56-runtime-move.md); used by [RFC-59](rfc-59-model-tool-loop.md) |
| lifecycle (`journal` / `transition`) | Durable lifecycle log and legal transitions | Runtime host service in [RFC-56](rfc-56-runtime-move.md) unless split into a later lifecycle RFC |
| `references` | Adapter-exported prose shelf | Contracted in [RFC-51](rfc-51-adapter-wit.md); called by [RFC-59](rfc-59-model-tool-loop.md) |
| `wasi-model` | Judgment host effect, `eval(prompt) -> result<answer, error>` | Core boundary in [RFC-53](rfc-53-wasi-model.md); tool loop in [RFC-59](rfc-59-model-tool-loop.md); verify profiles in [RFC-60](rfc-60-verify-profiles.md); backend catalogue in [RFC-58](rfc-58-model-backends.md) |

`eval`'s interface is Omnia-owned, like `wasi:keyvalue`, so the `augentic:specify` worlds gain it as an upstream import once the Omnia dependency is pinned. The `references` shelf and the per-axis source / target operations stay in `augentic:specify` ([RFC-51](rfc-51-adapter-wit.md)).

## Scope

- Name the effect vocabulary.
- Record which RFC owns each concrete implementation.
- Wire only the minimal WIT import / export declarations needed by the next implementation step.

## Out of scope

- Implementing host handlers for every effect.
- Defining the model tool loop or backend catalogue.
- Defining working-tree materialization.
- Defining lifecycle storage policy beyond assigning ownership.

## Acceptance criteria

1. The effect vocabulary is explicit and has no unowned implementation area.
2. Every effect passes handles or references, not bodies or artifact corpora.
3. The `augentic:specify` contract stays adapter- and model-agnostic.
4. No runtime behaviour changes are required by this RFC alone.

## Risks and invariants

- **Map, not monolith.** This RFC should not become the catch-all implementation document for unrelated effects.
- **Handles, not corpora.** Briefs, references, working trees, and changes cross as handles or content-addressed values.
- **Agnostic contract.** No adapter name, taxonomy, or model id belongs in the effect vocabulary.
