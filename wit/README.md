# Specify WIT package

[`specify.wit`](specify.wit) is the `augentic:specify` WebAssembly Component Model package: the typed adapter contract (the shared `types` interface, the per-axis `target` / `source` interfaces, and their worlds). See [RFC-51](../rfcs/rfc-51-adapter-wit.md).

This repo owns and publishes `augentic:specify`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get augentic:specify@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
