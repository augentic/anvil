# Specify WIT package

[`specify.wit`](specify.wit) is the `augentic:specify` WebAssembly Component Model package: the typed adapter contract — the shared `types` interface, the per-axis `target` / `source` interfaces, the `references` shelf each adapter exports, and their worlds. See [RFC-51](../rfcs/rfc-51-adapter-wit.md) and [RFC-52](../rfcs/rfc-52-effect.md).

Judgment is **not** a WIT effect. It is the binary's native tool-use loop ([RFC-53](../rfcs/rfc-53-tool-server.md)) behind a native `ModelClient` boundary, so there is no `augentic:model` / `eval` package — the former `model.wit` is retired. The model-facing `augentic:tools` surface is specified inline in [RFC-53](../rfcs/rfc-53-tool-server.md); it becomes a published WIT package only when the optional Mode-B MCP / HTTP guest is built — the default Mode-A facade is native code and needs no WIT.

This repo owns and publishes `augentic:specify`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get augentic:specify@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
