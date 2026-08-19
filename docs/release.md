# Release process

A Emery release ships four artifacts: the **platform binaries** (the archives the `binaries` job in `publish.yaml` builds as workflow artifacts), the **workspace crates** published to crates.io under `emery-*` package names (the shared `crates.yaml` job), the **engine guest** published as the wasm-pkg package `emery:engine@<version>`, and — when the WIT `package` declaration moved — the **adapter contract** published as `emery:adapter@<wit version>`. Publish builds the platform archives first, then runs the shared Omnia-shaped workflow (tag + Release notes, with those archives attached at release creation) and `cargo publish --workspace`; the two wasm-pkg packages are **published manually** with `wkg publish` (see [Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)). Workspace packages publish as `emery-<crate>` (short `use` paths come from each crate's `[lib] name`); the lab crate (`emery-guest`) and the root `emery` package stay `publish = false`. **The first successful crates publish is gated on omnia pin hygiene**: the workspace rides `[patch.crates-io]` git pins for the omnia stack (currently 0.35.0 against a crates.io 0.34.0), and Cargo patches do not propagate to dependents — until omnia 0.35+ (plus `omnia-guest` / `omnia-cursor` and friends) is on crates.io, the `crates` job fails and the release stands on the other artifacts. Adapter components ship from the adapters repo, not here. This page describes the end-to-end flow so a maintainer can cut a release without reading workflow YAML.

## Three version axes

Three surfaces version independently — never force them to share a number:

| Axis | Identity | Where it lives |
| ---- | -------- | -------------- |
| **Host** | `emery` binary / `emery:engine@<version>` | `[workspace.package].version` in `Cargo.toml`, the `v*` tag, `RELEASES.md` |
| **WIT contract** | `emery:adapter@<wit version>` | the `package` declaration in `wit/emery.wit`; versions independently of the binary |

The engine guest's own world lives in a separate, host-only WIT package — `emery:engine@0.1.0` at `crates/guest/wit/` — so host-seam changes (capability imports on the `workflow` world) never move the `emery:adapter` contract or the adapter train. That package is not published for adapters; only the `emery:adapter` contract is a vendored pin. Do not confuse the `emery:engine` WIT package (the world declaration, versioned independently) with the `emery:engine@<version>` wasm-pkg **component** publish below (the compiled guest, versioned with the binary) — same namespace, different artifact kinds.
| **Adapter train** | `emery:<name>@<semver>` → `ghcr.io/augentic/emery-adapters/<name>:<version>` | the adapters repo's shared `[workspace.package]` SemVer |

Compatibility between host and adapters is declared — exact pins plus each adapter's `emery-floor` (minimum host) — not implied by equal numbers. The Cursor `/emery:*` plugin is an ultrathin CLI wrapper; bump its marketplace / `plugin.json` versions only when `plugins/` content changes, not on every host release.

The host embeds no adapter-version recommendation and — since the Phase 3 spine cut (ADR-0002) — no download path: adapter resolution is local-only (project cache seed, the embedded first-party registry, the verified global store). The embedded first-party registry is populated at release-build time: the Publish Release workflow pulls each component pinned in [`scripts/first-party.txt`](../scripts/first-party.txt) from GHCR into `EMERY_EMBED_DIR` before building the binary. Bumping a pin is a release act — the adapter train publishes first, then the pin file moves in the release PR.

## Release lines

Releases live on durable `release-X.Y.Z` branches, the same shape as Omnia's shared `augentic/.github` workflows. `main` always carries the *next unreleased* version (Cargo version plus the `Unreleased` heading in `RELEASES.md`). The four verbs:

1. **Cut** — dispatch **Create Release** on `main`. A preflight `scorecard` job refuses the cut unless a committed green scorecard under [`rfcs/scorecards/`](../rfcs/scorecards/) names the release-tip sha (CONSTITUTION invariant 6; the live graded eval stays operator-invoked in `emery-adapters`). Then it pushes `release-X.Y.Z` at the current tip and opens a PR that bumps `main` to the next unreleased version and resets `RELEASES.md`. Merge that PR; edit release notes on the release branch, not on `main`.
2. **Stabilize** — on the release branch only: check the omnia pins (`cargo build --locked` must resolve on a clean runner — re-pin any local-path `[patch.crates-io]` entry to a pushed rev) and backport fixes from `main` (fixes land on `main` first when applicable). See [the developer loop](contributing/dev-loop.md) for the local rungs.
3. **Publish** — dispatch **Publish Release** on the release branch. The same `scorecard` preflight guards this tip: backports and patch bumps re-earn a green scorecard naming the branch tip before artifacts cut. Omnia shape: shared CI, then the `binaries` matrix builds the platform archives as workflow artifacts, then the shared publish workflow (dates `RELEASES.md`, pushes `vX.Y.Z`, creates the GitHub Release with notes and those archives attached), then the shared `crates.yaml` job publishes the `emery-*` workspace crates to crates.io. Then publish the wasm-pkg packages manually (below).
4. **Patch** — bugfix and security only, on the same `release-X.Y.Z` branch: land the fix on `main` when applicable, backport, dispatch **Create Patch** on the branch (bumps `X.Y.Z → X.Y.Z+1` and preps `RELEASES.md`), then dispatch **Publish Release** on the same branch. Never invent a new line from a floating tag; never merge to `main` as the publish trigger.

Pre-1.0 SemVer follows Omnia's convention: **minor may be breaking**; patches remain compatible within the line. The hard major-cut / re-init product policy is called out in release notes, never smuggled into a patch.

## Three release shapes

Every release chooses exactly one shape; the order prevents adapters shipping against unpublished seam changes:

| Shape | Trigger | Order |
| ----- | ------- | ----- |
| **WIT-breaking** | `package emery:adapter@…` moves | 1) engine release branch + publish WIT 2) engine publish 3) adapters bump pin + train release 4) announce hard-cut / re-init when product policy requires it |
| **Host-only** | CLI / lifecycle / engine guest; `emery:adapter` WIT unchanged (changes confined to the `emery:engine` world's host imports are this shape) | engine cut → publish; adapters unchanged unless the floor must rise |
| **Adapter-only** | prompts, rules, target behavior; seam unchanged | adapters cut → publish; engine unchanged |

Never release adapters against an unpublished WIT or an unreleased engine commit that changed the seam.

Each release's notes entry in `RELEASES.md` includes a short compatibility row:

```text
engine 0.28.x  ↔  adapters 0.5.x  (WIT emery:adapter@0.1.0, floor ≥ 0.28.0)
```

Keep the table short — it is a statement of what was tested together, not a version solver.

## Jobs that run

Publish composes five jobs (plus the release-branch skip gate):

1. **`ci`.** Shared `augentic/.github` CI over the release branch, alongside the **`scorecard`** preflight (`scripts/scorecard-gate.sh` over the committed record in `rfcs/scorecards/`).
2. **`binaries`.** Local matrix job in `publish.yaml`: each leg pulls the first-party components pinned in `scripts/first-party.txt` from GHCR into `EMERY_EMBED_DIR` (the root build script's embedded registry), then builds and packages its archive, uploading it as a workflow artifact (`archive-<target>`).
3. **`publish`.** Shared `augentic/.github` publish: date `RELEASES.md`, push `vX.Y.Z`, create the GitHub Release with notes and the `archive-*` workflow artifacts attached.
4. **`crates`.** Shared `augentic/.github` `crates.yaml`: `cargo publish --workspace --locked` over the publishable `emery-*` packages (needs the org `CARGO_REGISTRY_TOKEN`). Until the omnia stack the workspace patches is itself on crates.io, this job fails on dependency resolution — expected, and no other artifact depends on it.

Each leg runs native `cargo build --release --locked --target <triple> --bin emery`. `build.rs` embeds the engine via a child `wasm32-wasip2` build and then ahead-of-time compiles it for the leg's target triple (same path as `cargo install --git`); compile-affecting runtime settings (`MAX_FUEL`, `BRANCH_HINTING`, `MEMORY_RESERVATION`, `MEMORY_GUARD_SIZE`, `DEBUG_SYMBOLS`, `GENERATE_ADDRESS_MAP`) default-match between build and run — an operator override at run time that diverges from the build rejects the embedded artifact at startup. Supported targets (Homebrew + `cargo-binstall`; no `cross`):

- `x86_64-unknown-linux-gnu` on `ubuntu-latest`
- `x86_64-apple-darwin` on `macos-15-intel` (last hosted x86_64 macOS runner; retired August 2027)
- `aarch64-apple-darwin` on `macos-latest`
- `x86_64-pc-windows-msvc` on `windows-latest` — temporarily out of the matrix until upstream `omnia-wasi-model` compiles on Windows again

Each leg produces `emery-v${VERSION}-${TARGET}.tar.gz` (unix) or `.zip` (Windows) plus a companion `.sha256`; the shared publish workflow attaches both when it creates the GitHub Release. Root `Cargo.toml` carries `[package.metadata.binstall]` pointing at those archive names.

The shipped surface is the `emery` binary alone: the binary is one `omnia::runtime!` command-mode invocation (`src/main.rs`) embedding the engine guest as static component bytes, with mounts and the adapters-only guest resolver contributed by the root package's `launcher` module — so there is no second binary or component to package.

## Publishing the wasm-pkg packages

Both wasm-pkg packages are published manually with `wkg publish` by a maintainer whose wkg config maps the `emery:` namespace to `augentic.io` with a GitHub token carrying `packages: write` (see [`wit/README.md`](../wit/README.md) for the config shape). Registry identities are immutable — never re-publish an existing version; bump the version first.

- **Engine guest.** After tagging, publish the release-built engine component as `emery:engine@<version>`, where `<version>` is the workspace package version. The shipped binary does **not** consume the published package — it embeds the identical bytes at build time — but the registry identity remains the canonical distribution for other Omnia hosts composing the engine guest (and keeps the published identity equal to the binary version).

```bash
cargo build --lib -p emery --release --target wasm32-wasip2
wkg publish target/wasm32-wasip2/release/emery.wasm \
  --package "emery:engine@$(cargo pkgid -p emery | sed 's/.*#//')"
```

- **Adapter contract.** When a contract change bumps the `package emery:adapter@<ver>;` declaration in `wit/emery.wit`, publish it as `emery:adapter@<ver>` — the WIT versions independently of the binary. See [`wit/README.md`](../wit/README.md) for the exact commands. `emery-adapters` consumes the published package as its vendored pin. On a WIT-breaking line, publish the WIT before or with the engine publish — never after adapters that need it have already shipped.

## Adapter components

First-party adapter components are **not** built or published by this repo. They live in `augentic/emery-adapters`, ride the same release-branch verbs on their own lockstep train SemVer, and ship as Wasm OCI artifacts to GHCR (`ghcr.io/augentic/emery-adapters/<name>:<version>`) from that repo's **Publish Release** workflow (same `cargo make publish <name>` path as a local breakout). Before an adapter train publishes, its tree must build against a **published** `emery:adapter` WIT pin, its engine git dependencies must be pinned to a **released** engine tag (`tag = "vX.Y.Z"`, no active sibling `[patch]` block), and each adapter's `emery-floor` must name the minimum host that can run the train. The `emery` binary resolves a pin (`emery:<name>@<version>`) from the local global store only — there is no pull-on-miss (ADR-0002). The release binaries carry the first-party components embedded as default registry entries: the `binaries` job pulls each pin in [`scripts/first-party.txt`](../scripts/first-party.txt) from GHCR and stages it as `EMERY_EMBED_DIR` before building.

## Installing a release

Supported install paths:

- **Installer script** — [`scripts/install.sh`](../scripts/install.sh) downloads the Release archive for the detected platform, verifies the `.sha256` companion, and installs to `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh
```

`/emery:init` bootstraps the CLI through the same script with an exact `--version` pin; bumping that pin in `plugins/emery/skills/init/SKILL.md` (and the version examples in the install docs) is a release-checklist step when a new host version publishes.

- **Homebrew** — [`augentic/homebrew-tap`](https://github.com/augentic/homebrew-tap) formula over these Release archives:

```bash
brew tap augentic/tap
brew install emery
```

- **`cargo binstall`** (prebuilt; no local compile). The root package is `publish = false`, so install from git with a package version pin:

```bash
cargo binstall --git https://github.com/augentic/emery emery@<version>
```

- **GitHub Release archives.** Download the archive for your platform from the GitHub Release page, verify it against the companion `.sha256` file, and place the `emery` binary on your `PATH`.
- **Source builds** — `build.rs` embeds the wasm32 engine (requires the `wasm32-wasip2` target):

```bash
cargo install --git https://github.com/augentic/emery --locked
```

Bump the Homebrew formula `version` and `sha256` values in `augentic/homebrew-tap` when publishing a new host release. Subsequent updates use the same installation channel. Guest-owned verbs additionally need `cursor-agent` on `PATH` (logged in) at run time — the model backend spawns it; the engine guest ships inside the binary, so replacing the binary replaces the engine with it.

## Adding a new target triple

1. Add a native `matrix.include` entry to the `binaries` job in `.github/workflows/publish.yaml` (`runs-on` must provide that triple without `cross`).
2. If the target needs system packages (e.g. `musl-tools` for `*-musl`), add an `apt-get install` step gated on `matrix.target == '<new triple>'`.
3. Update `[package.metadata.binstall]` overrides if the archive format differs from `tgz`.
4. Add the `uname -sm` → triple mapping to the platform detection in [`scripts/install.sh`](../scripts/install.sh).
5. Document the new target in this file.

## Troubleshooting

- **Archive SHA256 drift.** Never hand-edit a checksum. The `.sha256` companion files are generated in the same `binaries` leg that packages each archive and are authoritative.
- **`wkg publish` rejects or the identity already exists.** Registry identities are immutable — never re-push different bytes into an existing version. Bump the version (the workspace version for the engine, the WIT `package` declaration for the contract) and publish the new identity instead.
- **Publish Release refuses the tag.** The tag for the branch's Cargo version already exists — releases are immutable; dispatch **Create Patch** on the line to bump, then publish again.
