# Vectis Adapter

- **Identifier:** `vectis`
- **Package:** `emery:vectis@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis)
- **Purpose:** Cross-platform Crux application development (Rust core, Swift iOS shell, Kotlin Android shell)

**Prerequisite:** greenfield Vectis builds materialize from a local [`vectis-exemplar`](https://github.com/augentic/vectis-exemplar) checkout at `../vectis-exemplar` (relative to the consumer project root) or `VECTIS_EXEMPLAR_DIR`. Emery does not clone or refresh that tree.

## Operations

Closed WIT target contract: `guidance`, `build`, `verify`, `repair`, `review`, `merge`. Core synthesis (the refine phase) writes the canonical artifacts; Vectis never writes them. `composition.yaml` is a build output, not a synthesis artifact.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/guidance.md) |
| `build` | Composition regeneration and core/shell writers (one pass) — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build.md) |
| `verify` | One check pass over the lent workspace |
| `repair` | One findings-directed writer pass (engine-routed origin: verification or review) |
| `review` | One engineering-standards review pass |
| `merge` | Adoption gates around the merge phase — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/merge.md) |

The engine dispatches the four build-loop operations one pass at a time and owns the retry budgets and the terminal report; see the [Adapter contract](../adapter-contract.md#target-adapter-contract).

In-guest helpers: [Vectis in-guest tools](../cli/vectis.md). Component catalog: [Component factoring](../../explanation/components.md).

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
