# `emery:adapter` WIT

This directory owns [`emery.wit`](emery.wit) — the adapter contract alone (one `source` interface exporting `extract` + `metadata`, and the `source-adapter` world — survey and the target world are deleted; retrieve them from tag `v1`) — and publishes it as the wasm-pkg package `emery:adapter`. [augentic/emery-adapters](https://github.com/augentic/emery-adapters) consumes it as a vendored copy. The package is self-contained: it imports nothing outside itself, so this directory carries no `deps/`. The `source-adapter` world imports `source` as well as exporting it, so the one binding generation in `emery-source` carries both the export side the SDK's `source!` wires into and the engine guest's caller side (`emery_source::wire::import`); the guest has no separate WIT world.

## Publishing

Publishing is manual. Registry identities are immutable — bump the `package emery:adapter@<ver>;` declaration in `emery.wit` first, and never re-publish an existing version.

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) and configure the `emery:` namespace with credentials that can write to the backing registry (a GitHub token with `packages: write`):

```toml
# ~/.config/wasm-pkg/config.toml
[namespace_registries]
emery = "augentic.io"

[registry."augentic.io".oci]
auth = { username = "<github-user>", password = "<token>" }
```

Then, from the repo root:

```bash
wkg wit build --wit-dir wit --output emery-adapter.wasm
wkg publish emery-adapter.wasm --package emery:adapter@<ver>
```

## Consuming

Pulls are anonymous — the namespace mapping alone is enough:

```bash
wkg get emery:adapter@<semver> --format wit --output emery.wit
```

### `wkg` Registry

The `emery:` namespace maps to `augentic.io`, whose `/.well-known/wasm-pkg/registry.json` resolves to the backing OCI registry.

See [Composing and Distributing](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html) for more information.
