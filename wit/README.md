# `emery:adapter` WIT

This directory owns [`emery.wit`](emery.wit) — the adapter contract alone (the `source` / `target` interfaces and the adapter worlds) — and publishes it as the wasm-pkg package `emery:adapter`. [augentic/emery-adapters](https://github.com/augentic/emery-adapters) consumes it as a vendored copy. The package is self-contained: it imports nothing outside itself, so this directory carries no `deps/`.

The engine guest's `workflow` world lives in its own host-only package, `emery:engine@0.1.0`, owned by [`crates/guest/wit/`](../crates/guest/wit/) — the crate that binds it. Its `deps/` symlinks resolve this package plus the host capability packages. Host-seam changes (capability imports on the `workflow` world) therefore never move `emery:adapter` or the adapter release train; adapters never see the engine package.

Host-only capabilities live in their capability crates (e.g. `emery:exec-mode` in [`crates/wasi-exec/wit/`](../crates/wasi-exec/wit/)), resolved into the engine package through `crates/guest/wit/deps/` symlinks; adapters never see those packages either.

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
