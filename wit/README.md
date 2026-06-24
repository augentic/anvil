# Specify WIT package

[`specify.wit`](specify.wit) is the `augentic:specify` WebAssembly Component Model package: the typed adapter contract — the shared `types` interface, the per-axis `target` / `source` interfaces, the `references` shelf each adapter exports, and their worlds. See [RFC-51](../rfcs/rfc-51-adapter-wit.md) and [RFC-52](../rfcs/rfc-52-effect.md).

Judgment is an Omnia host effect, not part of this package: Omnia's `wasi-model` host exports `eval(prompt) -> result<answer, error>`, which guests import like any other host interface. Behind it, a swappable model backend runs the tool-use loop and follows a brief's internal references by calling the adapter's `references` shelf (`resolve`). The model id and vendor SDK live in that backend, never in this package or the runtime floor. See [RFC-53](../rfcs/rfc-53-wasi-model.md) (the `wasi-model` host) and [RFC-58](../rfcs/rfc-58-model-backends.md) (the model backends).

This repo owns and publishes `augentic:specify`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get augentic:specify@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
