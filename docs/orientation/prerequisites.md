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

The `specify` binary backs every skill in the Specify plugin. `/spec:init` installs or refreshes it from source with:

```bash
cargo install --git https://github.com/augentic/specify --locked --force
```

<details>
<summary>Manual install (alternative)</summary>

For manual setup, install via one of the following methods:

```bash
# Prebuilt from GitHub Releases (no local compile; needs cargo-binstall)
cargo binstall --git https://github.com/augentic/specify

# GitHub Release archive: download for your platform, verify against the
# companion .sha256 file, and place `specify` on PATH
# https://github.com/augentic/specify/releases

# Pre-built binary, any POSIX shell (installer deferred; archives already ship)
curl -sSfL https://specify.sh/install.sh | sh

# Local checkout of this repo
cargo install --path . --locked
```

A Homebrew tap (`brew install augentic/tap/specify`) is planned but not yet published — darwin archives on each GitHub Release are the inputs.

Pin a specific version with `SPECIFY_VERSION=v0.1.0` in front of the `curl` command, or override the install location with `SPECIFY_INSTALL_DIR=/usr/local/bin`.

</details>

Verify the installation:

```bash
specify --version
```

### Keeping the CLI current

Update the CLI through the same channel used to install it:

```bash
cargo binstall --git https://github.com/augentic/specify            # prebuilt
cargo install --git https://github.com/augentic/specify --locked    # source
# or upgrade with your package manager / replace the release binary
```

`specify init --upgrade` is a separate project re-entry command: it updates the project's Specify pin and preservation-safe scaffold while retaining operator-authored artifacts. It does not update the installed CLI.

### Contributing to the repo

The above covers installing `specify` to *use* Specify in your own project. Contributing to the [`augentic/specify`](https://github.com/augentic/specify) repo itself — editing skills, adapters, references, docs, or the CLI (the Cargo workspace at the repo root) — needs only a Rust toolchain, not a separately installed `specify`:

```bash
cargo make links # Developer Guide link integrity
make ci          # the full Rust workspace gate (cargo make ci)
cargo install --path . --locked # install the working-tree CLI into ~/.cargo/bin
```

No published binary is downloaded — every invocation builds from the in-tree Cargo workspace, so CI and clean clones build the same source. The Rust workspace pins its own toolchain in [`rust-toolchain.toml`](https://github.com/augentic/specify/blob/main/rust-toolchain.toml); `cargo make fmt` uses nightly rustfmt. (This is unrelated to the `SPECIFY_VERSION=vX.Y.Z` prefix accepted by the `curl` installer above, which pins the version to *install* for operators.) See [Quality gates](../contributing/quality-gates.md#consistency-links).

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
