# Specify WIT package

[`specify.wit`](specify.wit) is the `augentic:specify` WebAssembly Component Model package: the typed adapter contract — the shared `types` interface, the per-axis `target` / `source` interfaces, and their worlds. The pending contract revision (no resource types across guest-to-guest calls, the `wasi:http` MCP export as each adapter's reference surface, schema-gated answers) is owned by [RFC-61](../rfcs/rfc-61-omnia-migration.md); the original contract RFC is archived at [RFC-51](../rfcs/archive/rfc-51-adapter-wit.md).

Judgment is an Omnia host effect, not part of this package: Omnia's `wasi-model` host (`omnia:model@0.1.0`) exports `create(request) -> result<reply, error>`, which guests import like any other host interface. Behind it sits a swappable model backend — `omnia-cursor` spawns `cursor-agent` against the mounted working tree and delivers reference documents through the MCP grants named in the request; `omnia-genai` drives hosted APIs; `ModelDefault` replays recorded answers. The model id and vendor SDK live in that backend, never in this package or the runtime floor.

This repo owns and publishes `augentic:specify`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get augentic:specify@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
