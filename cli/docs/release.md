# Release process

Specify publishes platform binaries and WASI tool packages on every tagged release — those are the only release artifacts. The workspace crates are not published to crates.io (there are no external crate consumers; workspace deps are path-only by design). This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML.

## Triggering a release

Releases are driven by an annotated tag matching `v*.*.*`:

```bash
git tag -a v0.1.0 -m "Specify v0.1.0"
git push origin v0.1.0
```

The `.github/workflows/release.yaml` workflow fires on the tag push and runs three jobs in order.

## Jobs that run

1. **`build` (matrix).** Compiles a release binary for each supported target:
   - `x86_64-unknown-linux-gnu` on `ubuntu-latest` (native `cargo build`).
   - `aarch64-unknown-linux-gnu` on `ubuntu-latest` via [`cross`](https://github.com/cross-rs/cross) (portable glibc toolchain, mirrors rustup's own release workflow — avoids hand-wiring `gcc-aarch64-linux-gnu` env vars per step).
   - `x86_64-apple-darwin` on `macos-13` (native).
   - `aarch64-apple-darwin` on `macos-14` (native).
   - `x86_64-pc-windows-msvc` on `windows-latest` (native).

   Each job produces a versioned archive (`specify-${TAG}-${TARGET}.tar.gz` on unix, `.zip` on Windows) plus a companion `.sha256` file, uploaded via `actions/upload-artifact@v4`.

2. **`release`.** Waits for every native matrix leg, downloads all artifacts, and creates the GitHub Release with `softprops/action-gh-release@v2`. Release notes are generated from `.github/release.yaml`.

## Adapter extension packages

First-party adapter extensions (`contract`, `vectis`) are **not** built or published by this repo. They live with their adapters in `augentic/specify-adapters` and are packaged + published as immutable registry artifacts (`specify:<name>@<version>`) by that repo's own release workflow (RFC-48). The `specify` binary resolves them at read time from the global adapter store; operators only need the runtime binary.

## Installing a release

Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`. `specify upgrade` handles subsequent updates channel-natively.

## Adding a new target triple

1. Add a new entry to the `matrix.include` list in `.github/workflows/release.yaml`, choosing the `runs-on` runner and whether `use_cross: true` is needed.
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Document the new target in this file.

## Troubleshooting

- **`cross` installation fails.** Pin to a known-good commit in the `Install cross` step.
- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `release.yaml` are authoritative.
