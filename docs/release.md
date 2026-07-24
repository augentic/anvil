# Release process

A Specify release ships three artifacts: the **platform binaries** (the archives `release-binaries.yaml` builds and attaches to the GitHub release on the `v*` tag), the **engine guest** published as the wasm-pkg package `specify:engine@<version>`, and — when the WIT `package` declaration moved — the **adapter contract** published as `specify:adapter@<wit version>`. CI builds the binaries and creates the release; the two wasm-pkg packages are **published manually** with `wkg publish` (see [Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)). The workspace crates are never published to crates.io: the root package is `publish = false` because the omnia runtime stack rides `[patch.crates-io]` path/git pins (Cargo patches do not propagate to dependents, so a published crate would be unbuildable), and there are no external crate consumers anyway. Adapter components ship from the adapters repo, not here. This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML. The design home is [RFC-77](../rfcs/rfc-77-release-process.md).

## Three version axes

Three surfaces version independently — never force them to share a number:

| Axis | Identity | Where it lives |
| ---- | -------- | -------------- |
| **Host** | `specify` binary / `specify:engine@<version>` | `[workspace.package].version` in `Cargo.toml`, the `v*` tag, `RELEASES.md` |
| **WIT contract** | `specify:adapter@<wit version>` | the `package` declaration in `wit/specify.wit`; versions independently of the binary |
| **Adapter train** | `specify:<name>@<semver>` → `ghcr.io/augentic/specify-adapters/<name>:<version>` | the adapters repo's shared `[workspace.package]` SemVer |

Compatibility between host and adapters is declared — exact pins plus each adapter's `specify-floor` (minimum host) — not implied by equal numbers. The Cursor `/spec:*` plugin manifests co-version with the host release; the release workflows sync them automatically.

## Release lines

Releases live on durable `release-X.Y.Z` branches, the same shape as Omnia's shared `augentic/.github` workflows. `main` always carries the *next unreleased* version (Cargo version plus the `Unreleased` heading in `RELEASES.md`). The four verbs:

1. **Cut** — dispatch **Create Release** on `main`. It pushes `release-X.Y.Z` at the current tip and opens a PR that bumps `main` to the next unreleased version, resets `RELEASES.md`, and syncs the plugin manifests. Merge that PR; edit release notes on the release branch, not on `main`.
2. **Stabilize** — on the release branch only: check the omnia pins (`cargo build --locked` must resolve on a clean runner — re-pin any local-path `[patch.crates-io]` entry to a pushed rev), run the operator rungs when the change warrants them (`cargo make wasm-run`, needs `CURSOR_API_KEY` in `examples/.env`; `cargo make eval`, needs command-mode model credentials — see [the developer loop](contributing/dev-loop.md)), and backport fixes from `main` (fixes land on `main` first when applicable).
3. **Publish** — dispatch **Publish Release** on the release branch. It gates on CI plus the Developer Guide links check, dates `RELEASES.md`, and pushes the `vX.Y.Z` tag. The tag fires `release-binaries.yaml`, which builds the platform archives and creates the GitHub Release with them attached (releases are immutable once published, so the assets must land at creation). Then publish the wasm-pkg packages manually (below).
4. **Patch** — bugfix and security only, on the same `release-X.Y.Z` branch: land the fix on `main` when applicable, backport, dispatch **Create Patch** on the branch (bumps `X.Y.Z → X.Y.Z+1` and preps `RELEASES.md`), then dispatch **Publish Release** on the same branch. Never invent a new line from a floating tag; never merge to `main` as the publish trigger.

Pre-1.0 SemVer follows Omnia's convention: **minor may be breaking**; patches remain compatible within the line. The hard major-cut / re-init product policy is called out in release notes, never smuggled into a patch.

## Three release shapes

Every release chooses exactly one shape; the order prevents adapters shipping against unpublished seam changes:

| Shape | Trigger | Order |
| ----- | ------- | ----- |
| **WIT-breaking** | `package specify:adapter@…` moves | 1) engine release branch + publish WIT 2) engine publish 3) adapters bump pin + train release 4) announce hard-cut / re-init when product policy requires it |
| **Host-only** | CLI / lifecycle / engine guest; WIT unchanged | engine cut → publish; adapters unchanged unless the floor must rise |
| **Adapter-only** | prompts, rules, target behavior; seam unchanged | adapters cut → publish; engine unchanged |

Never release adapters against an unpublished WIT or an unreleased engine commit that changed the seam.

Each release's notes entry in `RELEASES.md` includes a short compatibility row:

```text
engine 0.28.x  ↔  adapters 0.5.x  (WIT specify:adapter@0.1.0, floor ≥ 0.28.0)
```

Keep the table short — it is a statement of what was tested together, not a version solver.

## Jobs that run

1. **`build` (matrix).** Each job builds the wasm32 engine guest first, then the platform binary that embeds it:

```bash
cargo build --release --locked --lib -p specify --target wasm32-wasip2
cargo build --release --locked --bin specify
```

   The root `build.rs` resolves `SPECIFY_WASM` — an explicit environment override wins (the workflow points it at the prebuilt engine; a relative value anchors at the workspace root, so it resolves inside the `cross` container too), else `build.rs` spawns its own child `cargo build --lib --target wasm32-wasip2` into an isolated target directory — and `src/omnia.rs` embeds the bytes with `include_bytes!`, so the shipped binary carries its own engine (no first-launch download, no network). There is no placeholder fallback: an empty or missing component fails the build. The workflow additionally guards the wasm build product with `test -s`. Supported targets:
   - `x86_64-unknown-linux-gnu` on `ubuntu-latest` (native `cargo build`).
   - `aarch64-unknown-linux-gnu` on `ubuntu-latest` via [`cross`](https://github.com/cross-rs/cross) (portable glibc toolchain, mirrors rustup's own release workflow — avoids hand-wiring `gcc-aarch64-linux-gnu` env vars per step).
   - `x86_64-apple-darwin` on `macos-13` (native).
   - `aarch64-apple-darwin` on `macos-14` (native).
   - `x86_64-pc-windows-msvc` on `windows-latest` (native) — temporarily out of the matrix until upstream `omnia-wasi-model` compiles on Windows again.

   Each job produces a versioned archive (`specify-${TAG}-${TARGET}.tar.gz` on unix, `.zip` on Windows) plus a companion `.sha256` file, uploaded via `actions/upload-artifact@v4`.

2. **`release`.** Waits for the matrix legs, downloads all artifacts, and creates the GitHub Release with the archives attached and the notes taken from the tag's `RELEASES.md` (releases are immutable once published, so assets and notes must land at creation).

The shipped surface is the `specify` binary alone: the binary is one `omnia::runtime!` command-mode invocation (`src/omnia.rs`) embedding the engine guest as static component bytes, with mounts and the adapters-only guest resolver contributed by the `launcher` crate's expressions — so there is no second binary or component to package.

## Publishing the wasm-pkg packages

Both wasm-pkg packages are published manually with `wkg publish` by a maintainer whose wkg config maps the `specify:` namespace to `augentic.io` with a GitHub token carrying `packages: write` (see [`wit/README.md`](../wit/README.md) for the config shape). Registry identities are immutable — never re-publish an existing version; bump the version first.

- **Engine guest.** After tagging, publish the release-built engine component as `specify:engine@<version>`, where `<version>` is the workspace package version. The shipped binary does **not** consume the published package — it embeds the identical bytes at build time — but the registry identity remains the canonical distribution for other Omnia hosts composing the engine guest (and keeps the published identity equal to the binary version).

```bash
cargo build --lib -p specify --release --target wasm32-wasip2
wkg publish target/wasm32-wasip2/release/specify.wasm \
  --package "specify:engine@$(cargo pkgid -p specify | sed 's/.*#//')"
```

- **Adapter contract.** When a contract change bumps the `package specify:adapter@<ver>;` declaration in `wit/specify.wit`, publish it as `specify:adapter@<ver>` — the WIT versions independently of the binary. See [`wit/README.md`](../wit/README.md) for the exact commands. `specify-adapters` consumes the published package as its vendored pin. On a WIT-breaking line, publish the WIT before or with the engine publish — never after adapters that need it have already shipped.

## Adapter components

First-party adapter components are **not** built or published by this repo. They live in `augentic/specify-adapters`, ride the same release-branch verbs on their own lockstep train SemVer, and are published manually as standard Wasm OCI artifacts to GHCR (`ghcr.io/augentic/specify-adapters/<name>:<version>`, via that repo's `cargo make publish <name>`). Before an adapter train publishes, its tree must build against a **published** `specify:adapter` WIT pin, its engine git dependencies must be pinned to a **released** engine tag (`tag = "vX.Y.Z"`, no active sibling `[patch]` block), and each adapter's `specify-floor` must name the minimum host that can run the train. The `specify` binary resolves a pin (`specify:<name>@<version>`) from the global adapter store and installs a miss automatically from that fixed GHCR mapping (pull-on-miss); operators only need the runtime binary.

## Installing a release

Two supported install paths:

- **GitHub Release archives.** Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`.
- **Source builds** for Rust-native developers: one command — `build.rs` builds and embeds the wasm32 engine itself (requires the `wasm32-wasip2` target; the build fails with the `rustup target add wasm32-wasip2` instruction when it is missing).

```bash
cargo install --git https://github.com/augentic/specify --tag <tag> --locked
```

A Homebrew tap (`brew install augentic/tap/specify`) is deferred future work — the formula and automated tap bump land with the publishing roadmap's tap-automation item.

Subsequent updates use the same installation channel: rerun `cargo install`, upgrade through the package manager, or replace the downloaded binary. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the engine guest ships inside the binary, so replacing the binary replaces the engine with it.

## Adding a new target triple

1. Add a new entry to the `matrix.include` list in `.github/workflows/release-binaries.yaml`, choosing the `runs-on` runner and whether `use_cross: true` is needed.
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Document the new target in this file.

## Troubleshooting

- **`cross` installation fails.** Pin to a known-good commit in the `Install cross` step.
- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `release-binaries.yaml` are authoritative.
- **`wkg publish` rejects or the identity already exists.** Registry identities are immutable — never re-push different bytes into an existing version. Bump the version (the workspace version for the engine, the WIT `package` declaration for the contract) and publish the new identity instead.
- **Publish Release refuses the tag.** The tag for the branch's Cargo version already exists — releases are immutable; dispatch **Create Patch** on the line to bump, then publish again.
