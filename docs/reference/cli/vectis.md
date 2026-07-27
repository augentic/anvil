# Vectis in-guest tools

Vectis deterministic helpers are **in-guest library code** compiled into the vectis adapter's published component in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis). There is no host dispatch verb: the vectis build and merge orchestrations invoke these behaviours directly.

Authoritative behaviour (validate / verify / scaffold / sync modes, artifact cascade, report semantics) lives with the adapter:

- Adapter tree: [`targets/vectis/`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis)
- Build brief: [`prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build.md)
- Merge brief: [`prose/prompts/merge.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/merge.md)

## See also

- [Vectis target](../targets/vectis.md)
