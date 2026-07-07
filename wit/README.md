# Specify WIT package

[`specify.wit`](specify.wit) is the `specify:adapter` WebAssembly Component Model package: the typed adapter contract — the shared `types` interface, the per-axis `target` / `source` interfaces, and their worlds. The file reflects the RFC-61 contract revision (RFC-61 is completed and removed from the tree; recoverable from git history): no resource types cross guest-to-guest calls, each adapter world exports `wasi:http/incoming-handler` as its MCP reference surface (superseding the typed `references` interface of earlier drafts), and every judgment answer is schema-gated. The original contract RFC (RFC-51) was removed from the tree and is recoverable from git history.

Judgment is an Omnia host effect, not part of this package: Omnia's `wasi-model` host (`omnia:model@0.1.0`) exports `create(request) -> result<reply, error>`, which guests import like any other host interface. Behind it sits a swappable model backend — `omnia-cursor` spawns `cursor-agent` against the mounted working tree and delivers reference documents through the MCP grants named in the request; `omnia-genai` drives hosted APIs; `ModelDefault` replays recorded answers (a test-and-example convenience only). The model id and vendor SDK live in that backend, never in this package or the runtime floor.

This repo owns and publishes `specify:adapter`; [`specify-adapters`](https://github.com/augentic/specify-adapters) consumes it as a pinned dependency.

## Publish

The package publishes on each `v*` release tag: the `publish-wit` job in [`release-binaries.yaml`](../.github/workflows/release-binaries.yaml) runs `cargo make publish-wit`, which parses the version from this file's `package specify:adapter@<ver>;` declaration and pushes through the probe-first idempotent helper ([`scripts/wkg-publish-idempotent.sh`](../scripts/wkg-publish-idempotent.sh)). A tag that did not bump the declaration finds the version already published and no-ops — bumping the `package` version here is the whole release action for a contract change. See [`docs/release.md`](../docs/release.md).

## Consume

Map the `specify:` namespace to `augentic.io` in your [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) config (`[namespace_registries]` → `specify = "augentic.io"`); the host's `/.well-known/wasm-pkg/registry.json` resolves the rest, and pulls are anonymous:

```bash
wkg get specify:adapter@<semver> --format wit --output specify.wit
```

`specify-adapters` vendors the pinned version at `wit/deps/specify/specify.wit` via its `cargo make wit-vendor` task (with a `wit-vendor-sibling` dev override pointing at this checkout while a contract change iterates before publish).
