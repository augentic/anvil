# Release process

A Specify release ships three artifacts from one `v*` tag: the **platform binaries** (the archives `release-binaries.yaml` attaches to the GitHub release), the **core guest** published as the wasm-pkg package `specify:core@<version>`, and — when the WIT `package` declaration moved — the **adapter contract** published as `specify:adapter@<wit version>` (`DECISIONS.md` §"Publishing and distribution: one transport, idempotent legs"). The workspace crates are never published to crates.io: the root package is `publish = false` because the omnia runtime stack rides `[patch.crates-io]` path/git pins (Cargo patches do not propagate to dependents, so a published crate would be unbuildable), and there are no external crate consumers anyway. Adapter components ship from the adapters repo, not here. See `DECISIONS.md` §"Release identity". This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML.

## Before tagging

- **Refresh the embedded workflow guest.** The `specify` binary embeds the committed release-built workflow guest (`crates/workflow-guest/guest.wasm` via `include_bytes!` — `DECISIONS.md` §"Workflow-guest distribution"). If any guest-reachable code changed since the artifact was last regenerated, run `cargo make dist-guest` and commit the refreshed `guest.wasm` **and its `guest.wasm.sha256` sidecar** before the release PR merges. Staleness is CI-gated: `dist-guest` records the component's SHA-256 plus a fingerprint of the guest-reachable source trees in the sidecar, and the `tests/dist.rs` gate (part of the ordinary suite, so `cargo make ci` runs it) fails when either drifts. The gate does not fingerprint external dependency bumps (`Cargo.lock`) or toolchain changes — for those, this checklist item is still the backstop: if the guest's dependency graph moved, re-run `dist-guest` even though CI stays green. (This step retires with the standalone-deployment cut's core-by-binary-version switch, when the binary resolves its core from the registry-published `specify:core` and the committed blob is deleted; until then the publish leg below runs in parallel with the embed.)
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

2. **`publish-core`.** Publishes the core guest as `specify:core@<version>` through `cargo make publish-core`, where `<version>` is the `VERSION` file — the job guards that the tag matches `VERSION`, so the published core identity always equals the binary version. The task builds `specify-workflow-guest` for `wasm32-wasip2` and pushes it over the wasm-pkg transport; the `specify:` namespace resolves through `https://augentic.io/.well-known/wasm-pkg/registry.json` to the backing registry. This leg currently lands **inert**: the binary still embeds its committed core guest, and the consume side (hydrate-by-binary-version) belongs to the standalone-deployment cut — publishing on every tag de-risks that switch.

3. **`publish-wit`.** Publishes the adapter contract as `specify:adapter@<wit version>` through `cargo make publish-wit`, parsing the version from `wit/specify.wit`'s `package` declaration. There is no separate version-changed guard: a tag that did not bump the WIT version finds the identity already published and the leg no-ops (see idempotency below). `specify-adapters` consumes the published package as its vendored pin.

4. **`release`.** Waits for the matrix legs **and `publish-core`**, downloads all artifacts, and attaches them to the already-created GitHub Release with `softprops/action-gh-release@v2` (notes are owned by `publish.yaml`). The `needs: [build, publish-core]` edge is the **binary↔core-guest lockstep** (`DECISIONS.md` §"Publishing and distribution: one transport, idempotent legs"): a release whose `specify:core` push failed is a failed release — the binaries are never attached without the core published under the same version. `publish-wit` deliberately gates nothing: the WIT versions independently of the binary and most tags no-op that leg.

Both publish legs run through the probe-first `scripts/wkg-publish-idempotent.sh`: the registry is probed for the exact identity before anything is built or pushed, a present identity is skipped (registry identities are immutable — skip-if-present is what makes a re-run against an already-published tag a safe no-op that pushes nothing), and a probe that cannot distinguish *absent* from *unreachable* (network failure, auth, timeout) aborts the leg rather than risk re-publishing. Auth is `GITHUB_TOKEN` only — `permissions: packages: write` plus a workflow-written wkg config; there are no registry username/password secrets. CI pins `wkg` at 0.15.0 because the probe's not-found fingerprints are coupled to its error text — revalidate them when bumping. Local emergency publishing runs the same `cargo make publish-*` task with a developer's own token in their wkg config: one code path, two invocation surfaces.

The shipped binary is the one `specify`: the triage main serves the native verbs in-process and drives guest-owned verbs through the composed deployment (`DECISIONS.md` §"One `specify` binary"). The `specify-runtime-replay` binary target is a dev/test surface and is never packaged.

## Adapter components

First-party adapter components are **not** built or published by this repo. They live in `augentic/specify-adapters` and are published as immutable registry artifacts (`specify:<name>@<version>`) by that repo's own release workflow — the same idempotent, `GITHUB_TOKEN`-authenticated posture as the publish legs here (its `cargo make publish-adapters` probes each identity and pushes only what a tag actually bumped). The `specify` binary resolves them from the global adapter store; operators only need the runtime binary.

## Installing a release

Two supported install paths:

- **GitHub Release archives.** Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`.
- **`cargo install --git`** for Rust-native developers, building the binary from the tagged source.

A Homebrew tap (`brew install augentic/tap/specify`) is deferred future work — the formula and automated tap bump land with the publishing roadmap's tap-automation item.

`specify upgrade` handles subsequent updates channel-natively. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the binary itself carries the workflow guest.

## Adding a new target triple

1. Add a new entry to the `matrix.include` list in `.github/workflows/release-binaries.yaml`, choosing the `runs-on` runner and whether `use_cross: true` is needed.
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Document the new target in this file.

## Troubleshooting

- **`cross` installation fails.** Pin to a known-good commit in the `Install cross` step.
- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `release-binaries.yaml` are authoritative.
- **A publish leg aborts with "cannot distinguish absent from unreachable".** The identity probe failed for a reason other than a definitive not-found (registry outage, auth, timeout). Nothing was pushed; re-run the job once the registry is reachable — the probe-first design makes the re-run safe.
- **`publish-core` fails on the VERSION guard.** The tag and the `VERSION` file disagree; the release PR should have bumped `VERSION` to match. Fix the tree and re-release — never re-point a tag at different bytes.
