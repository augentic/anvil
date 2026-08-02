# Prerequisites

## Cursor IDE

Operators run `/emery:*` skills inside [Cursor](https://cursor.com). Install Cursor and ensure you are on a recent version that supports the plugin marketplace.

## Augentic plugin marketplace

Install the Augentic plugins from the Cursor marketplace:

1. Open Cursor Settings.
2. Navigate to Plugins.
3. Search for **Augentic**.
4. Install the plugin marketplace.
5. Restart Cursor.

This installs the Emery plugin (the `/emery:*` skill wrappers). Domain code generation for Omnia and Vectis lives in target adapters under [`targets/` in `augentic/emery-adapters`](https://github.com/augentic/emery-adapters/tree/main/targets/), not in Cursor skills.

## The `emery` CLI

The `emery` binary backs every skill in the Emery plugin. Pick one install route, then verify with `emery --version`.

### Installer script (recommended)

The [install script](https://github.com/augentic/emery/blob/main/scripts/install.sh) downloads the prebuilt Release archive for your platform, verifies it against the companion `.sha256`, and installs `emery` into `~/.local/bin` (override with `--dir <path>` or `EMERY_INSTALL_DIR`):

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh
```

Pin an exact release with `sh -s -- --version <version>`, using a version number from the [Releases page](https://github.com/augentic/emery/releases). `/emery:init` uses this path when it refreshes the CLI.

If `~/.local/bin` is not on your `PATH`, the script prints the exact `export PATH=…` line to add to your shell profile.

### Homebrew

The [augentic/homebrew-tap](https://github.com/augentic/homebrew-tap) formula installs the same prebuilt Release archives:

```bash
brew tap augentic/tap
brew install emery
```

Upgrade later with `brew upgrade emery`.

### cargo-binstall

Prebuilt archives, no local compile (install [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) first). The root package is `publish = false`, so install from git:

```bash
cargo binstall --git https://github.com/augentic/emery emery@<version>
```

### From source

Needs a Rust toolchain with the `wasm32-wasip2` target (`rustup target add wasm32-wasip2`). `build.rs` builds and embeds the engine guest:

```bash
cargo install --git https://github.com/augentic/emery --locked
```

### Manual archive

Download the archive for your platform from the [GitHub Releases](https://github.com/augentic/emery/releases) page, verify it against the companion `.sha256` file, and place `emery` on your `PATH`.

<details>
<summary>Local checkout (contributors)</summary>

```bash
cargo install --path . --locked
```

</details>

Verify the installation:

```bash
emery --version
```

### Keeping the CLI current

Update through the same channel used to install:

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh   # installer script
brew upgrade emery                                                                         # Homebrew
cargo binstall --git https://github.com/augentic/emery emery@<version> --force            # prebuilt
cargo install --git https://github.com/augentic/emery --locked --force                     # source
```

`emery init --upgrade` is a separate project re-entry command: it updates the project's Emery pin and preservation-safe scaffold while retaining operator-authored artifacts. It does not update the installed CLI.

### Contributing to the repo

The above covers installing `emery` to *use* Emery in your own project. Working on Emery itself needs only a Rust toolchain — see [Contributing to Emery](../contributing/index.md#building-from-a-checkout).

## Adapter-specific prerequisites

Depending on which adapter you use, you may need additional tooling.

### Omnia adapter

- [Rust toolchain](https://rust-lang.org/tools/install/)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`

### Vectis adapter

- [Rust toolchain](https://rust-lang.org/tools/install/)
- [Rust Analyzer](https://open-vsx.org/extension/rust-lang/rust-analyzer) Cursor extension

**Exemplar checkout (required for greenfield):** clone [`augentic/vectis-exemplar`](https://github.com/augentic/vectis-exemplar) as a sibling of the consumer project (`../vectis-exemplar`) or set `VECTIS_EXEMPLAR_DIR`. Emery does not clone it. Install BoltFFI so shell Makefiles can pack native bindings: `cargo install boltffi_cli` (see the exemplar README).

**For iOS shells:**
- Xcode command line tools
- Build and formatting tools: `brew install xcode-build-server xcbeautify swiftformat xcodegen`
- iOS simulator targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
- Cursor extensions: [Swift Language Support](https://open-vsx.org/extension/chrisatwindsurf/swift-vscode), [SweetPad](https://marketplace.visualstudio.com/items?itemName=SweetPad.sweetpad)

**For Android shells:**
- Android SDK (via Android Studio or command-line tools)
- JDK compatible with the template's Android `compileOptions` (today: Java 17 for `:app`, JVM 11 for `:shared` — not a hard "Java 21 only" pin)
- Confirm with `make -C Android doctor` after materialize
- Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`

See the [Vectis target adapter reference](../reference/targets/vectis.md) for complete setup instructions.

## Verify your setup

Run through this checklist to confirm everything is ready:

1. **CLI installed:** `emery --version` prints a version number.
2. **Cursor plugins:** Open Cursor Settings > Plugins and confirm the Augentic plugins are listed.
3. **Adapter tooling:** If using Omnia, run `rustup target list --installed` and confirm `wasm32-wasip2` appears. If using Vectis, confirm `rustc --version` succeeds.

If all three checks pass, proceed to the [Quick Start](../tutorials/quick-start.md).
