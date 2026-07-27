# Contracts Adapter

- **Identifier:** `contracts`
- **Package:** `emery:contracts@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/contracts)
- **Purpose:** API contract authoring, import, and validation (OpenAPI, AsyncAPI, JSON Schema)

## Operations

Closed WIT target contract: `guidance`, `build`, `merge`. Format sub-flows and verify-repair live in the adapter.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/guidance.md) |
| `build` | Author / import / verify sub-flows — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/build.md) |
| `merge` | Baseline validator gate after merge — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/merge.md) |

In-guest validator: [Contracts in-guest validator](../cli/contract.md).

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
