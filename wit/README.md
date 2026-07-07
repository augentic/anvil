# Specify WIT package

[`specify.wit`](specify.wit) is the `specify:adapter` WebAssembly Component Model package: the typed adapter contract — the shared `types` interface, the per-axis `target` / `source` interfaces, and their worlds. The file reflects the RFC-61 contract revision (RFC-61 is completed and removed from the tree; recoverable from git history): no resource types cross guest-to-guest calls, each adapter world exports `wasi:http/incoming-handler` as its MCP reference surface (superseding the typed `references` interface of earlier drafts), and every judgment answer is schema-gated. The original contract RFC (RFC-51) was removed from the tree and is recoverable from git history.

Judgment is an Omnia host effect, not part of this package: Omnia's `wasi-model` host (`omnia:model@0.1.0`) exports `create(request) -> result<reply, error>`, which guests import like any other host interface. Behind it sits a swappable model backend — `omnia-cursor` spawns `cursor-agent` against the mounted working tree and delivers reference documents through the MCP grants named in the request; `omnia-genai` drives hosted APIs; `ModelDefault` replays recorded answers (a test-and-example convenience only). The model id and vendor SDK live in that backend, never in this package or the runtime floor.

This repo owns and publishes `specify:adapter`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Prerequisites

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools).

## Consume

```bash
wkg get specify:adapter@<semver> --config .wkg-config.toml --output ./wit/deps/specify.wit
```
