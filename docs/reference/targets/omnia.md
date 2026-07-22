# Omnia Adapter

- **Identifier:** `omnia`
- **Package:** `specify:omnia@<semver>` in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters/tree/main/targets/omnia)
- **Purpose:** Omnia Wasm guest / Rust crate generation

## Operations

Closed WIT target contract: `guidance`, `build`, `merge`. Core `/spec:refine` synthesises canonical artifacts; Omnia never writes them.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/guidance.md) |
| `build` | Crate / test / guest / review phases — [`prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/build.md) |
| `merge` | Adoption gates after `specify slice merge` — [`prose/prompts/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/merge.md) |

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
