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
cargo install --git https://github.com/augentic/specify-cli
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

# Local checkout of specify-cli
make build
```

Pin a specific version with `SPECIFY_VERSION=v0.1.0` in front of the `curl` command, or override the install location with `SPECIFY_INSTALL_DIR=/usr/local/bin`.

</details>

Verify the installation:

```bash
specify --version
```

### Keeping the CLI current

`specify upgrade` self-updates the binary in place. It detects the install channel from the running binary's path — `cargo` (under `$CARGO_HOME/bin`, or `~/.cargo/bin`), `brew` (a Homebrew Cellar/prefix), `binary` (a system install such as `/usr/local/bin` or `/opt/specify`), or `unknown` (it then prints manual-upgrade guidance via a structured `unknown-install-channel` diagnostic). Pass `--channel` to override detection.

It resolves the latest release before upgrading — `SPECIFY_RELEASE_TAG` env override first, then `gh release view -R augentic/specify-cli` when `gh` is on `PATH`, then an unauthenticated `api.github.com` request. A probe failure is a warning, not an error: the upgrade proceeds against `HEAD` with a journal note. Preview with `--dry-run`; apply with `--yes`.

```bash
specify upgrade --dry-run            # report channel + target version, write nothing
specify upgrade --yes                # self-update and journal `cli.upgraded`
```

The `cargo` and `brew` executors are fully wired; the `binary`-channel in-process self-replace is deferred to a follow-up, so today that channel emits planned-action plus manual-upgrade guidance.

### Contributing to the framework repo

The above covers installing `specify` to *use* Specify in your own project. Contributing to the [`augentic/specify`](https://github.com/augentic/specify) framework repo itself — editing skills, adapters, references, or docs — does **not** require a `specify-cli` checkout or a Rust toolchain. `make lint` (the only framework check) delegates to `./scripts/specify.sh fcheck`, which resolves a `specify` binary per the `SPECIFY_VERSION` environment variable and runs `specify lint framework --framework-root .`:

| `SPECIFY_VERSION` | Binary comes from |
| ----------------- | ----------------- |
| `next` (default) | a sibling/nested `specify-cli` source build, **falling back to the `.specify-version` pin acquired into a repo-local `./.bin`** when no checkout is present |
| `latest` | the newest published release |
| `X.Y.Z` | one pinned published release |
| `system` | whatever `specify` is already on `PATH` |

The default `next` keeps source builds the primary path for co-developing the workflow contract, but degrades gracefully so a docs/skills/rules contributor with no Rust toolchain gets a working `make lint` with zero manual setup. The single-line [`.specify-version`](https://github.com/augentic/specify/blob/main/.specify-version) file at that repo's root pins the published CLI release the framework currently targets; `./scripts/specify.sh fcheck` is the direct equivalent of `make lint` and works from any subdirectory. (This `SPECIFY_VERSION` knob is distinct from the `SPECIFY_VERSION=vX.Y.Z` prefix accepted by the `curl` installer above, which pins the version to *install* rather than the binary to *bind*.) See [Consistency Checks](../contributing/checks.md#binding-to-a-specify-binary) for the full binding model.

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
