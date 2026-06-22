# Specify WIT package

[`specify.wit`](specify.wit) is the `augentic:specify` WebAssembly Component Model package: the typed adapter contract (the shared `types` interface, the per-axis `target` / `source` interfaces, and their worlds). See [RFC-51](../rfcs/rfc-51-adapter-wit.md).

[`model.wit`](model.wit) is the `augentic:model` package: the Specify-owned `eval` effect interface (see [RFC-52](../rfcs/rfc-52-effect.md)) that the Omnia model host satisfies (see [RFC-54](../rfcs/rfc-54-model-host.md)). Omnia provides the host slot; Specify owns and versions the interface — so it carries an `augentic:` namespace, not `omnia:`.

This repo owns and publishes both packages; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes `augentic:specify` as a pinned dependency, and the Omnia model host consumes `augentic:model`.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get augentic:specify@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
