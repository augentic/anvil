# Release process

A Specify release ships three artifacts: the **platform binaries** (the archives `binaries.yaml` builds and attaches to the GitHub Release), the **engine guest** published as the wasm-pkg package `specify:engine@<version>`, and — when the WIT `package` declaration moved — the **adapter contract** published as `specify:adapter@<wit version>`. Publish uses the shared Omnia-shaped workflow (tag + Release notes), then attaches the platform archives; the two wasm-pkg packages are **published manually** with `wkg publish` (see [Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)). The workspace crates are never published to crates.io: the root package is `publish = false` because the omnia runtime stack rides `[patch.crates-io]` path/git pins (Cargo patches do not propagate to dependents, so a published crate would be unbuildable), and there are no external crate consumers anyway. Adapter components ship from the adapters repo, not here. This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML. The design home is [RFC-77](../rfcs/rfc-77-release-process.md).

## Three version axes

Three surfaces version independently — never force them to share a number:

| Axis | Identity | Where it lives |
| ---- | -------- | -------------- |
| **Host** | `specify` binary / `specify:engine@<version>` | `[workspace.package].version` in `Cargo.toml`, the `v*` tag, `RELEASES.md` |
| **WIT contract** | `specify:adapter@<wit version>` | the `package` declaration in `wit/specify.wit`; versions independently of the binary |
| **Adapter train** | `specify:<name>@<semver>` → `ghcr.io/augentic/specify-adapters/<name>:<version>` | the adapters repo's shared `[workspace.package]` SemVer |

Compatibility between host and adapters is declared — exact pins plus each adapter's `specify-floor` (minimum host) — not implied by equal numbers. The Cursor `/spec:*` plugin is an ultrathin CLI wrapper; bump its marketplace / `plugin.json` versions only when `plugins/` content changes, not on every host release.

## Release lines

Releases live on durable `release-X.Y.Z` branches, the same shape as Omnia's shared `augentic/.github` workflows. `main` always carries the *next unreleased* version (Cargo version plus the `Unreleased` heading in `RELEASES.md`). The four verbs:

1. **Cut** — dispatch **Create Release** on `main`. It pushes `release-X.Y.Z` at the current tip and opens a PR that bumps `main` to the next unreleased version and resets `RELEASES.md`. Merge that PR; edit release notes on the release branch, not on `main`.
2. **Stabilize** — on the release branch only: check the omnia pins (`cargo build --locked` must resolve on a clean runner — re-pin any local-path `[patch.crates-io]` entry to a pushed rev), run the operator rungs when the change warrants them (`cargo make wasm-run`, needs `CURSOR_API_KEY` in `examples/.env`; `cargo make eval`, needs command-mode model credentials — see [the developer loop](contributing/dev-loop.md)), and backport fixes from `main` (fixes land on `main` first when applicable).
3. **Publish** — dispatch **Publish Release** on the release branch. Omnia shape: shared CI, then the shared publish workflow (dates `RELEASES.md`, pushes `vX.Y.Z`, creates the GitHub Release with notes), then `binaries.yaml` builds the platform archives and attaches them to that release. Then publish the wasm-pkg packages manually (below).
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

Publish composes three jobs (plus the release-branch skip gate):

1. **`ci`.** Shared `augentic/.github` CI over the release branch.
2. **`publish`.** Shared `augentic/.github` publish: date `RELEASES.md`, push `vX.Y.Z`, create the GitHub Release with notes.
3. **`binaries`.** Local `binaries.yaml` (`workflow_call`): each matrix leg builds, packages, and attaches its archive to the release the shared publish step just created.

Each leg runs native `cargo build --release --locked --target <triple> --bin specify`. `build.rs` embeds the engine via a child `wasm32-wasip2` build when `SPECIFY_WASM` is unset (same path as `cargo install --git`). Supported targets (Homebrew + `cargo-binstall`; no `cross`):

- `x86_64-unknown-linux-gnu` on `ubuntu-latest`
- `x86_64-apple-darwin` on `macos-15-intel` (last hosted x86_64 macOS runner; retired August 2027)
- `aarch64-apple-darwin` on `macos-14`
- `x86_64-pc-windows-msvc` on `windows-latest` — temporarily out of the matrix until upstream `omnia-wasi-model` compiles on Windows again

Each leg produces `specify-v${VERSION}-${TARGET}.tar.gz` (unix) or `.zip` (Windows) plus a companion `.sha256`, and uploads both to the existing GitHub Release. Root `Cargo.toml` carries `[package.metadata.binstall]` pointing at those archive names.

The shipped surface is the `specify` binary alone: the binary is one `omnia::runtime!` command-mode invocation (`src/main.rs`) embedding the engine guest as static component bytes, with mounts and the adapters-only guest resolver contributed by the `launcher` crate's expressions — so there is no second binary or component to package.

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

First-party adapter components are **not** built or published by this repo. They live in `augentic/specify-adapters`, ride the same release-branch verbs on their own lockstep train SemVer, and ship as Wasm OCI artifacts to GHCR (`ghcr.io/augentic/specify-adapters/<name>:<version>`) from that repo's **Publish Release** workflow (same `cargo make publish <name>` path as a local breakout). Before an adapter train publishes, its tree must build against a **published** `specify:adapter` WIT pin, its engine git dependencies must be pinned to a **released** engine tag (`tag = "vX.Y.Z"`, no active sibling `[patch]` block), and each adapter's `specify-floor` must name the minimum host that can run the train. The `specify` binary resolves a pin (`specify:<name>@<version>`) from the global adapter store and installs a miss automatically from that fixed GHCR mapping (pull-on-miss); operators only need the runtime binary.

## Installing a release

Supported install paths (see [Prerequisites](orientation/prerequisites.md) for detail):

- **Homebrew** — [`augentic/homebrew-tap`](https://github.com/augentic/homebrew-tap) formula over these Release archives:

```bash
export HOMEBREW_GITHUB_API_TOKEN="$(gh auth token)"   # while specify is private
brew tap augentic/tap
brew install specify
```

- **`cargo binstall`** (prebuilt; no local compile). The root package is `publish = false`, so install from git with a package version pin:

```bash
cargo binstall --git https://github.com/augentic/specify specify@<version>
```

- **GitHub Release archives.** Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `specify` binary on your `PATH`.
- **Source builds** — `build.rs` embeds the wasm32 engine (requires the `wasm32-wasip2` target):

```bash
cargo install --git https://github.com/augentic/specify --locked
```

Bump the Homebrew formula `version` and `sha256` values in `augentic/homebrew-tap` when publishing a new host release. Subsequent updates use the same installation channel. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the engine guest ships inside the binary, so replacing the binary replaces the engine with it.

## Adding a new target triple

1. Add a native `matrix.include` entry in `.github/workflows/binaries.yaml` (`runs-on` must provide that triple without `cross`).
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Update `[package.metadata.binstall]` overrides if the archive format differs from `tgz`.
4. Document the new target in this file.

## Troubleshooting

- **Archive SHA256 drift.** Always regenerate after tagging — never hand-edit. The `.sha256` companion files uploaded by `binaries.yaml` are authoritative.
- **`wkg publish` rejects or the identity already exists.** Registry identities are immutable — never re-push different bytes into an existing version. Bump the version (the workspace version for the engine, the WIT `package` declaration for the contract) and publish the new identity instead.
- **Publish Release refuses the tag.** The tag for the branch's Cargo version already exists — releases are immutable; dispatch **Create Patch** on the line to bump, then publish again.
