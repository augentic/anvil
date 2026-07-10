# Release process

A Specify release ships three artifacts: the **platform binaries** (the archives `release-binaries.yaml` builds and attaches to the GitHub release on the `v*` tag), the **core guest** published as the wasm-pkg package `specify:core@<version>`, and — when the WIT `package` declaration moved — the **adapter contract** published as `specify:adapter@<wit version>`. CI builds and attaches the binaries; the two wasm-pkg packages are **published manually** with `wkg publish` (see [Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)). The workspace crates are never published to crates.io: the root package is `publish = false` because the omnia runtime stack rides `[patch.crates-io]` path/git pins (Cargo patches do not propagate to dependents, so a published crate would be unbuildable), and there are no external crate consumers anyway. Adapter components ship from the adapters repo, not here. This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML.

## Before tagging

- **Check the omnia pins.** Release builds run `cargo build --locked`, so the `[patch.crates-io]` entries in `Cargo.toml` must resolve on a clean runner: git-rev pins build anywhere, sibling *path* pins only build where the sibling checkout exists. Re-pin any local-path patch to a pushed rev before tagging.

## Triggering a release

Releases are PR-driven: `release.yaml` (manual dispatch) opens a `release/v*` PR; merging it triggers `publish.yaml`, which pushes the annotated `v*.*.*` tag and creates the GitHub Release with curated notes. The tag push then fires `.github/workflows/release-binaries.yaml`, which builds and attaches the platform archives.

## Jobs that run

1. **`build` (matrix).** Compiles `--release --locked --bin specify` for each supported target:
   - `x86_64-unknown-linux-gnu` on `ubuntu-latest` (native `cargo build`).
   - `aarch64-unknown-linux-gnu` on `ubuntu-latest` via [`cross`](https://github.com/cross-rs/cross) (portable glibc toolchain, mirrors rustup's own release workflow — avoids hand-wiring `gcc-aarch64-linux-gnu` env vars per step).
   - `x86_64-apple-darwin` on `macos-13` (native).
   - `aarch64-apple-darwin` on `macos-14` (native).
   - `x86_64-pc-windows-msvc` on `windows-latest` (native).

   Each job produces a versioned archive (`specify-${TAG}-${TARGET}.tar.gz` on unix, `.zip` on Windows) plus a companion `.sha256` file, uploaded via `actions/upload-artifact@v4`.

2. **`release`.** Waits for the matrix legs, downloads all artifacts, and attaches them to the already-created GitHub Release with `softprops/action-gh-release@v2` (notes are owned by `publish.yaml`).

The shipped surface is the `specify` binary alone: the binary is a single macro-generated command-mode runtime (`omnia::runtime!` in `src/runtime.rs`), so there is no second binary to build or package.

## Publishing the wasm-pkg packages

Both wasm-pkg packages are published manually with `wkg publish` by a maintainer whose wkg config maps the `specify:` namespace to `augentic.io` with a GitHub token carrying `packages: write` (see [`wit/README.md`](../wit/README.md) for the config shape). Registry identities are immutable — never re-publish an existing version; bump the version first.

- **Core guest.** After tagging, publish the release-built workflow component as `specify:core@<version>`, where `<version>` is the `VERSION` file — the published core identity must equal the binary version: a released binary consumes exactly `specify:core@<its own version>` and carries no embedded guest.

```bash
cargo build --lib -p specify-cli --release --target wasm32-wasip2
wkg publish target/wasm32-wasip2/release/specify.wasm --package "specify:core@$(cat VERSION)"
```

- **Adapter contract.** When a contract change bumps the `package specify:adapter@<ver>;` declaration in `wit/specify.wit`, publish it as `specify:adapter@<ver>` — the WIT versions independently of the binary. See [`wit/README.md`](../wit/README.md) for the exact commands. `specify-adapters` consumes the published package as its vendored pin.

## Adapter components

First-party adapter components are **not** built or published by this repo. They live in `augentic/specify-adapters` and are published as immutable registry artifacts (`specify:<name>@<version>`) over the same wasm-pkg transport. The `specify` binary resolves them from the global adapter store; operators only need the runtime binary.

## Installing a release

Two supported install paths:

- **GitHub Release archives.** Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`.
- **`cargo install --git`** for Rust-native developers, building the binary from the tagged source.

A Homebrew tap (`brew install augentic/tap/specify`) is deferred future work — the formula and automated tap bump land with the publishing roadmap's tap-automation item.

`specify upgrade` handles subsequent updates channel-natively. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the workflow (core) guest resolves by the binary's own version, `specify:core@<binary version>`.

## Adding a new target triple

1. Add a new entry to the `matrix.include` list in `.github/workflows/release-binaries.yaml`, choosing the `runs-on` runner and whether `use_cross: true` is needed.
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Document the new target in this file.

## Troubleshooting

- **`cross` installation fails.** Pin to a known-good commit in the `Install cross` step.
- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `release-binaries.yaml` are authoritative.
- **`wkg publish` rejects or the identity already exists.** Registry identities are immutable — never re-push different bytes into an existing version. Bump the version (the `VERSION` file for the core, the WIT `package` declaration for the contract) and publish the new identity instead.
