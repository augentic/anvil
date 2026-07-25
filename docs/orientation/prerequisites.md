# Prerequisites

## Cursor IDE

Operators run `/spec:*` skills inside [Cursor](https://cursor.com). Install Cursor and ensure you are on a recent version that supports the plugin marketplace.

## Augentic plugin marketplace

Install the Augentic plugins from the Cursor marketplace:

1. Open Cursor Settings.
2. Navigate to Plugins.
3. Search for **Augentic**.
4. Install the plugin marketplace.
5. Restart Cursor.

This installs the Specify plugin (the `/spec:*` skill wrappers). Domain code generation for Omnia and Vectis lives in target adapters under [`adapters/targets/`](https://github.com/augentic/specify-adapters/tree/main/targets/), not in Cursor skills.

## The `specify` CLI

The `specify` binary backs every skill in the Specify plugin. Pick one install route, then verify with `specify --version`.

### Homebrew (recommended on macOS / Linuxbrew)

The [augentic/homebrew-tap](https://github.com/augentic/homebrew-tap) formula installs prebuilt Release archives. While `augentic/specify` is private, export a token that can read that repo:

```bash
export HOMEBREW_GITHUB_API_TOKEN="$(gh auth token)"   # or a PAT with repo scope
brew tap augentic/tap
brew install specify
```

Upgrade later with `brew upgrade specify`.

### cargo-binstall

Prebuilt archives, no local compile (install [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) first). The root package is `publish = false`, so install from git:

```bash
cargo binstall --git https://github.com/augentic/specify specify@0.29.0
```

`/spec:init` uses this path when it refreshes the CLI (`--force -y`).

### From source

Needs a Rust toolchain with the `wasm32-wasip2` target (`rustup target add wasm32-wasip2`). `build.rs` builds and embeds the engine guest:

```bash
cargo install --git https://github.com/augentic/specify --locked
```

### Manual archive

Download the archive for your platform from the [GitHub Releases](https://github.com/augentic/specify/releases) page, verify it against the companion `.sha256` file, and place `specify` on your `PATH`.

<details>
<summary>Local checkout (contributors)</summary>

```bash
cargo install --path . --locked
```

</details>

Verify the installation:

```bash
specify --version
```

### Keeping the CLI current

Update through the same channel used to install:

```bash
brew upgrade specify                                                              # Homebrew
cargo binstall --git https://github.com/augentic/specify specify@0.29.0 --force   # prebuilt
cargo install --git https://github.com/augentic/specify --locked --force          # source
```

`specify init --upgrade` is a separate project re-entry command: it updates the project's Specify pin and preservation-safe scaffold while retaining operator-authored artifacts. It does not update the installed CLI.

### Contributing to the repo

The above covers installing `specify` to *use* Specify in your own project. Contributing to the [`augentic/specify`](https://github.com/augentic/specify) repo itself — editing skills, adapters, references, docs, or the CLI (the Cargo workspace at the repo root) — needs only a Rust toolchain, not a separately installed `specify`:

```bash
cargo make links # Developer Guide link integrity
make ci          # the full Rust workspace gate (cargo make ci)
cargo install --path . --locked # install the working-tree CLI into ~/.cargo/bin
```

No published binary is downloaded — every invocation builds from the in-tree Cargo workspace, so CI and clean clones build the same source. The Rust workspace pins its own toolchain in [`rust-toolchain.toml`](https://github.com/augentic/specify/blob/main/rust-toolchain.toml); `cargo make fmt` uses nightly rustfmt. See [Quality gates](../contributing/quality-gates.md#consistency-links).

## Adapter-specific prerequisites

Depending on which adapter you use, you may need additional tooling.

### Omnia adapter

- [Rust toolchain](https://rust-lang.org/tools/install/)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`

### Vectis adapter

- [Rust toolchain](https://rust-lang.org/tools/install/)
- [Rust Analyzer](https://open-vsx.org/extension/rust-lang/rust-analyzer) Cursor extension

**For iOS shells:**
- Xcode command line tools
- Build and formatting tools: `brew install xcode-build-server xcbeautify swiftformat xcodegen`
- iOS simulator targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
- Swift bindings: `cargo install cargo-swift`
- Cursor extensions: [Swift Language Support](https://open-vsx.org/extension/chrisatwindsurf/swift-vscode), [SweetPad](https://marketplace.visualstudio.com/items?itemName=SweetPad.sweetpad)

**For Android shells:**
- Android SDK (via Android Studio or command-line tools)
- Android NDK: `sdkmanager "ndk;29.0.14206865"`
- Java 21 LTS JDK (not Java 25+ -- Gradle compatibility)
- Gradle: `brew install gradle`
- Python 3 (required by rust-android-gradle)
- Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`

See the [Vectis target adapter reference](../reference/targets/vectis.md) for complete setup instructions.

## Verify your setup

Run through this checklist to confirm everything is ready:

1. **CLI installed:** `specify --version` prints a version number.
2. **Cursor plugins:** Open Cursor Settings > Plugins and confirm the Augentic plugins are listed.
3. **Adapter tooling:** If using Omnia, run `rustup target list --installed` and confirm `wasm32-wasip2` appears. If using Vectis, confirm `rustc --version` succeeds.

If all three checks pass, proceed to the [Quick Start](../tutorials/quick-start.md).
