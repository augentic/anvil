# Contracts in-guest validator

Contracts deterministic validation is **in-guest library code** compiled into the contracts adapter's published component in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/contracts). The host dispatches no contracts tool: build and merge orchestrations invoke the validator directly.

Authoritative SemVer / id-format / cross-repo uniqueness rules live with the adapter:

- Adapter tree: [`targets/contracts/`](https://github.com/augentic/emery-adapters/tree/main/targets/contracts)
- Build brief: [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/build.md)
- Merge brief: [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/merge.md)

## See also

- [Contracts target](../targets/contracts.md)
