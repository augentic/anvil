# `specify:adapter` WIT

This repo owns [`specify.wit`](specify.wit) and publishes it as the wasm-pkg package `specify:adapter`. [augentic/specify-adapters](https://github.com/augentic/specify-adapters) consumes it as a vendored copy.

## Publishing

Publishing is manual. Registry identities are immutable — bump the `package specify:adapter@<ver>;` declaration in `specify.wit` first, and never re-publish an existing version.

Install [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) and configure the `specify:` namespace with credentials that can write to the backing registry (a GitHub token with `packages: write`):

```toml
# ~/.config/wasm-pkg/config.toml
[namespace_registries]
specify = "augentic.io"

[registry."augentic.io".oci]
auth = { username = "<github-user>", password = "<token>" }
```

Then, from the repo root:

```bash
wkg wit build --wit-dir wit --output specify-adapter.wasm
wkg publish specify-adapter.wasm --package specify:adapter@<ver>
```

## Consuming

Pulls are anonymous — the namespace mapping alone is enough:

```bash
wkg get specify:adapter@<semver> --format wit --output specify.wit
```

### `wkg` Registry

The `specify:` namespace maps to `augentic.io`, whose `/.well-known/wasm-pkg/registry.json` resolves to the backing OCI registry.

See [Composing and Distributing](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html) for more information.
