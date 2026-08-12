# Omnia Adapter

- **Identifier:** `omnia`
- **Package:** `emery:omnia@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/omnia)
- **Purpose:** Omnia Wasm guest / Rust crate generation

## Operations

Closed WIT target contract: `guidance`, `build`, `verify`, `repair`, `review`, `merge`. Core synthesis (the refinement stage, `emery plan refine`) writes the canonical artifacts; Omnia never writes them.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/guidance.md) |
| `build` | Preparation, generation, and capture replay (one pass) — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/build.md) |
| `verify` | One Cargo-check pass over the lent workspace |
| `repair` | One findings-directed writer pass (engine-routed origin: verification or review) |
| `review` | One engineering-standards review pass |
| `merge` | Adoption gates around the merge phase — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/merge.md) |

The engine dispatches the four build-loop operations one pass at a time and owns the retry budgets and the terminal report; see the [Adapter contract](../adapter-contract.md#target-adapter-contract).

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
