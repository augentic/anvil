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

This installs all five plugins (Specify, Omnia, Vectis, RT, Plan) and their skills.

## The `specify` CLI

The `specify` binary backs every skill in the Specify plugin. Install via one of the following methods (in order of preference):

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
- `wasm32-wasi` target: `rustup target add wasm32-wasi`

### Vectis schema

- [Rust toolchain](https://rust-lang.org/tools/install/)
- [Rust Analyzer](https://open-vsx.org/extension/rust-lang/rust-analyzer) Cursor extension

**For iOS shells:**
- Xcode command line tools
- `xcode-build-server`: `brew install xcode-build-server`
- iOS simulator targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
- Swift bindings: `cargo install --locked uniffi-bindgen-swift@0.4.0+v0.28.3`

**For Android shells:**
- Android Studio with SDK 35+
- Android NDK via `sdkmanager "ndk;28.0.13004108"`
- `cargo-ndk`: `cargo install cargo-ndk`
- Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`

See the [Vectis plugin reference](../reference/plugins/vectis.md) for complete setup instructions.
