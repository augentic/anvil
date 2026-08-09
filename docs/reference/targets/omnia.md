# Omnia Adapter

- **Identifier:** `omnia`
- **Package:** `emery:omnia@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/omnia)
- **Purpose:** Omnia Wasm guest / Rust crate generation

## Operations

Closed WIT target contract: `guidance`, `build`, `merge`. Core synthesis (the refine phase) writes the canonical artifacts; Omnia never writes them.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/guidance.md) |
| `build` | Crate / test / guest / review phases — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/build.md) |
| `merge` | Adoption gates around the merge phase — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/merge.md) |

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
