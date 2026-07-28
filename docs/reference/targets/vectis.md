# Vectis Adapter

- **Identifier:** `vectis`
- **Package:** `emery:vectis@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis)
- **Purpose:** Cross-platform Crux application development (Rust core, Swift iOS shell, Kotlin Android shell)

**Prerequisite:** greenfield Vectis builds materialize from a local [`vectis-exemplar`](https://github.com/augentic/vectis-exemplar) checkout at `../vectis-exemplar` (relative to the consumer project root) or `VECTIS_EXEMPLAR_DIR`. Emery does not clone or refresh that tree.

## Operations

Closed WIT target contract: `guidance`, `build`, `merge`. Core `/emery:refine` synthesises canonical artifacts; Vectis never writes them. `composition.yaml` is a build output, not a synthesis artifact.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/guidance.md) |
| `build` | Composition regeneration, core/shell writers, verify-repair — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build.md) |
| `merge` | Adoption gates after `emery slice merge` — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/merge.md) |

In-guest helpers: [Vectis in-guest tools](../cli/vectis.md). Component catalog: [Component factoring](../../explanation/components.md).

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
