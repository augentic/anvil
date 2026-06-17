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

This installs the bundled plugins (Specify, Capture, Client) and their skills. Domain code generation for Omnia and Vectis lives in target adapters under [`adapters/targets/`](../../adapters/targets/), not in plugins.

## The `specify` CLI

The `specify` binary backs every skill in the Specify plugin. If you are setting up a project through Cursor, run `/spec:init`; when the CLI is missing, the skill can install it for you with:

```bash
cargo install --git https://github.com/augentic/specify
```

<details>
<summary>Manual install (alternative)</summary>

For manual setup, install via one of the following methods:

```bash
# macOS + Linux (primary)
brew install augentic/tap/specify

# Any platform with a Rust toolchain
cargo install specify

# Pre-built binary, any POSIX shell
curl -sSfL https://specify.sh/install.sh | sh

# Local checkout of this repo
make install-cli
```

Pin a specific version with `SPECIFY_VERSION=v0.1.0` in front of the `curl` command, or override the install location with `SPECIFY_INSTALL_DIR=/usr/local/bin`.

</details>

Verify the installation:

```bash
specify --version
```

### Keeping the CLI current

`specify upgrade` self-updates the binary in place. It detects the install channel from the running binary's path — `cargo` (under `$CARGO_HOME/bin`, or `~/.cargo/bin`), `brew` (a Homebrew Cellar/prefix), `binary` (a system install such as `/usr/local/bin` or `/opt/specify`), or `unknown` (it then prints manual-upgrade guidance via a structured `unknown-install-channel` diagnostic). Pass `--channel` to override detection.

It resolves the latest release before upgrading — `SPECIFY_RELEASE_TAG` env override first, then `gh release view -R augentic/specify` when `gh` is on `PATH`, then an unauthenticated `api.github.com` request. A probe failure is a warning, not an error: the upgrade proceeds against `HEAD` with a journal note. Preview with `--dry-run`; apply with `--yes`.

```bash
specify upgrade --dry-run            # report channel + target version, write nothing
specify upgrade --yes                # self-update and journal `cli.upgraded`
```

The `cargo` and `brew` executors are fully wired; the `binary`-channel in-process self-replace is deferred to a follow-up, so today that channel emits planned-action plus manual-upgrade guidance.

### Contributing to the repo

The above covers installing `specify` to *use* Specify in your own project. Contributing to the [`augentic/specify`](https://github.com/augentic/specify) repo itself — editing skills, adapters, references, docs, or the CLI under [`cli/`](https://github.com/augentic/specify/tree/main/cli) — needs only a Rust toolchain, not a separately installed `specify`. The CLI is an in-tree Cargo workspace, so the framework checks build it from source:

```bash
make lint        # build cli/ and run specify lint framework over the prose tree
make ci          # the full Rust workspace gate under cli/, then make lint
make install-cli # build cli/target/release/specify and symlink it onto your PATH
```

No published binary is downloaded — every invocation builds from the in-tree `cli/` Cargo workspace, so CI and clean clones build the same source. The Rust workspace pins its own toolchain in [`cli/rust-toolchain.toml`](https://github.com/augentic/specify/blob/main/cli/rust-toolchain.toml) (`cargo make fmt` uses nightly rustfmt). (This is unrelated to the `SPECIFY_VERSION=vX.Y.Z` prefix accepted by the `curl` installer above, which pins the version to *install* for operators.) See [Consistency Checks](../contributing/checks.md#the-in-tree-binary) for the full check model.

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
