# `emery:adapter` WIT

This directory owns [`emery.wit`](emery.wit) and publishes it as the wasm-pkg package `emery:adapter`. [augentic/emery-adapters](https://github.com/augentic/emery-adapters) consumes it as a vendored copy.

Host-only capabilities live in their capability crates. `emery:exec-bits` is owned by [`crates/wasi-exec-bits/wit/`](../crates/wasi-exec-bits/wit/) and resolved here through the `deps/exec-bits` symlink so the `workflow` world can import it; adapters never see that package.

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
