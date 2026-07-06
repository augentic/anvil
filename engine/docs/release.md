# Release process

Specify ships **platform binaries only** — the archives the `release-binaries.yaml` workflow attaches to each GitHub release. The workspace crates are never published to crates.io: the root package is `publish = false` because the omnia runtime stack rides `[patch.crates-io]` path/git pins (Cargo patches do not propagate to dependents, so a published crate would be unbuildable), and there are no external crate consumers anyway. WASI adapter extension packages ship from the adapters repo, not here. See `DECISIONS.md` §"Release identity". This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML.

## Before tagging

- **Refresh the embedded workflow guest.** The `specify` binary embeds the committed release-built workflow guest (`crates/workflow-guest/guest.wasm` via `include_bytes!` — `DECISIONS.md` §"Workflow-guest distribution"). If any guest-reachable code changed since the artifact was last regenerated, run `cargo make dist-guest` and commit the refreshed `guest.wasm` before the release PR merges; a stale embed ships stale workflow logic even though the composed tests (which self-build the guest from source) stay green.
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

2. **`release`.** Waits for every matrix leg, downloads all artifacts, and attaches them to the already-created GitHub Release with `softprops/action-gh-release@v2` (notes are owned by `publish.yaml`).

The shipped binary is the one `specify`: the triage main serves the native verbs in-process and drives guest-owned verbs through the composed deployment (`DECISIONS.md` §"One `specify` binary"). The `specify-runtime-replay` binary target is a dev/test surface and is never packaged.

## Adapter extension packages

First-party adapter extensions (`contract`, `vectis`) are **not** built or published by this repo. They live with their adapters in `augentic/specify-adapters` and are packaged + published as immutable registry artifacts (`specify:<name>@<version>`) by that repo's own release workflow (RFC-48). The `specify` binary resolves them at read time from the global adapter store; operators only need the runtime binary.

## Installing a release

Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`. `specify upgrade` handles subsequent updates channel-natively. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the binary itself carries the workflow guest.

## Adding a new target triple

1. Add a new entry to the `matrix.include` list in `.github/workflows/release-binaries.yaml`, choosing the `runs-on` runner and whether `use_cross: true` is needed.
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Document the new target in this file.

## Troubleshooting

- **`cross` installation fails.** Pin to a known-good commit in the `Install cross` step.
- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `release-binaries.yaml` are authoritative.
