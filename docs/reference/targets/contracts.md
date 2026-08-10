# Contracts Adapter

- **Identifier:** `contracts`
- **Package:** `emery:contracts@<semver>` in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/contracts)
- **Purpose:** API contract authoring, import, and validation (OpenAPI, AsyncAPI, JSON Schema)

## Operations

Closed WIT target contract: `guidance`, `build`, `verify`, `repair`, `review`, `merge`. Format sub-flows live in the adapter; the verify-repair loop is engine-driven — the adapter contributes one pass per dispatch.

| Operation | Owns |
| --------- | ---- |
| `guidance` | Idiom guidance for synthesis — [`prose/prompts/guidance.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/guidance.md) |
| `build` | Author / import format sub-flows (one pass) — [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/build.md) |
| `verify` | One contract-validation pass over the lent workspace |
| `repair` | One findings-directed repair pass (engine-routed origin: verification or review) |
| `review` | One engineering-standards review pass |
| `merge` | Baseline validator gate after merge — [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/merge.md) |

The engine dispatches the four build-loop operations one pass at a time and owns the retry budgets and the terminal report; see the [Adapter contract](../adapter-contract.md#target-adapter-contract).

In-guest validator: [Contracts in-guest validator](../cli/contract.md).

## See also

- [Target adapters index](index.md)
- [Anatomy of an adapter](../../explanation/adapter-anatomy.md)
