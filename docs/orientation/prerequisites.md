# Prerequisites

## Cursor IDE

Specify skills run inside [Cursor](https://cursor.com). Install Cursor and ensure you are on a recent version that supports the plugin marketplace.

## Augentic plugin marketplace

Install the Augentic plugins from the Cursor marketplace:

1. Open Cursor Settings.
2. Navigate to Plugins.
3. Search for **Augentic**.
4. Install the plugin marketplace.
5. Restart Cursor.

This installs all six plugins (Specify, Omnia, Vectis, Contracts, RT, Client) and their skills.

## The `specify` CLI

The `specify` binary backs every skill in the Specify plugin. If you are setting up a project through Cursor, run `/spec:init`; when the CLI is missing, the skill can install it for you with:

```bash
cargo install --git https://github.com/augentic/specify-cli
```

For manual setup, install via one of the following methods:

```bash
# macOS + Linux (primary)
brew install augentic/tap/specify

# Any platform with a Rust toolchain
cargo install specify

# Pre-built binary, any POSIX shell
curl -sSfL https://specify.sh/install.sh | sh

# Local checkout of specify-cli
make build
```

Pin a specific version with `SPECIFY_VERSION=v0.1.0` in front of the `curl` command, or override the install location with `SPECIFY_INSTALL_DIR=/usr/local/bin`.

Verify the installation:

```bash
specify --version
```

## Schema-specific prerequisites

Depending on which schema you use, you may need additional tooling.

### Omnia schema

- [Rust toolchain](https://rust-lang.org/tools/install/)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`

### Vectis schema

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

See the [Vectis plugin reference](../reference/plugins/vectis.md) for complete setup instructions.

## Verify your setup

Run through this checklist to confirm everything is ready:

1. **CLI installed:** `specify --version` prints a version number.
2. **Cursor plugins:** Open Cursor Settings > Plugins and confirm the Augentic plugins are listed.
3. **Schema tooling:** If using Omnia, run `rustup target list --installed` and confirm `wasm32-wasip2` appears. If using Vectis, confirm `rustc --version` succeeds.

If all three checks pass, proceed to the [Quick Start](../tutorials/quick-start.md).
