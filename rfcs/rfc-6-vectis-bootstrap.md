# RFC-6: Vectis Bootstrap CLI

> **Status: Superseded.** The standalone `vectis` binary described here has been folded into the `specify` CLI as the `specify vectis ...` subcommand tree, living in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (`crates/vectis/` library + `templates/vectis/`). This RFC documents the original standalone-binary design and is preserved for historical context; for current invocation, paths, and JSON contract see the vectis plugin SKILLs (`plugins/vectis/skills/{template-updater,core-writer,ios-writer,android-writer}/`).

> Status: Draft · Depends: — · Enables: skill simplification for `core-writer`, `ios-writer`, `android-writer`

## Abstract

Replace the agent-driven Create Mode scaffolding in the vectis writer skills with a Rust CLI binary (`vectis`) that generates a minimum-viable Crux cross-platform project and verifies all assemblies compile. The agent retains judgment over feature-specific implementation; the CLI handles deterministic project bootstrapping.

## Motivation

The vectis plugin has three writer skills that scaffold greenfield projects: `core-writer` (13 steps), `ios-writer` (11 steps), and `android-writer` (15 steps). Together they produce ~40 files across Rust, Swift, and Kotlin — workspace manifests, build scripts, FFI scaffolding, bridge classes, screen views, Gradle wrappers, and more. Every file is deterministic: given an app name, a set of capabilities, and target platforms, the output is fully predictable.

Today, the agent writes each file individually by interpreting prose instructions, then iterates on compilation errors until all three assemblies build. This is expensive:

- **Token cost**: The agent reads multiple reference documents (~3,000 lines of prose) and generates dozens of files in sequence.
- **Error iteration**: The agent frequently makes minor mistakes (wrong version pin, missing `#[repr(C)]`, wrong import path) that require multiple check-fix-recheck cycles.
- **Time**: A full greenfield bootstrap can take 10-20 minutes of agent time before any feature-specific work begins.
- **Fragility**: When reference documents update (e.g., a new Crux version), every agent invocation must independently discover and apply the change.

The scaffolded output is not a judgment call — it is a structured decision tree with enumerable inputs and deterministic outputs. This is exactly the category of operation that belongs in a CLI, following the same principle established in [RFC-1](archive/rfc-1-cli.md): **deterministic operations belong in a binary, not in agent prose.**

## Design Principles

| Use CLI (`vectis ...`) when: | Use agent judgment when: |
| --- | --- |
| The output is a known file with parameterized placeholders | The output requires reading spec/design artifacts |
| The file structure is identical across all apps of a given shape | The code structure depends on feature requirements |
| Correctness is verified by compilation | Correctness requires semantic review |
| Version pins and boilerplate must be exact | Logic and data flow require design decisions |
| The operation runs once at project creation | The operation runs repeatedly as features evolve |

The boundary is clear: `vectis init` produces the "Hello World" starting point — a compiling, runnable app with the correct project structure, version pins, and build pipeline. The writer skills then operate in Update Mode to transform this starting point into the feature-specific implementation described by the Specify artifacts.

## Detailed Design

### CLI Surface

Four subcommands: `init`, `add-shell`, `verify`, and `update-versions`.

#### `vectis init`

```
vectis init <app-name> [OPTIONS]

Arguments:
  <app-name>    App struct name (PascalCase, e.g. "Counter", "TodoApp", "NoteEditor")

Options:
  --dir <path>              Project directory [default: current directory]
  --caps <list>             Comma-separated capabilities [default: none]
                            Values: http, kv, time, platform, sse
  --shells <list>           Comma-separated shell platforms [default: none]
                            Values: ios, android
  --android-package <pkg>   Android package name [default: com.vectis.<appname lowercase>]
```

Core (the Rust shared crate, FFI scaffolding, and codegen binary) is always generated. No Crux app can exist without a core. The `--shells` flag controls which platform shells to scaffold alongside it.

The `--caps` flag determines which capability crates are included in `Cargo.toml`, which Effect variants appear in `app.rs`, which handler cases appear in shell bridge classes, and which platform-specific dependencies are added. When no capabilities are specified, the app uses Render only.

Examples:

```bash
# Render-only app with no shells
vectis init Counter

# HTTP app with both shells
vectis init TodoApp --caps http --shells ios,android

# Full stack with custom Android package
vectis init NoteEditor --caps http,kv --shells ios,android \
  --android-package com.example.noteeditor
```

#### `vectis add-shell`

```
vectis add-shell <platform> [OPTIONS]

Arguments:
  <platform>    Shell platform to add. Values: ios, android

Options:
  --dir <path>              Project directory [default: current directory]
  --android-package <pkg>   Android package name [default: com.vectis.<appname lowercase>]
```

Add a platform shell to an existing project that already has a core assembly. The command reads the existing `shared/src/app.rs` to determine the app name and capability set, then scaffolds only the shell assembly — no need to re-specify `--caps` or the app name. This is the common path for teams that start with core + one platform and add the second later.

The command fails with a clear error if:

- No core assembly exists (`shared/src/app.rs` not found) — use `vectis init` instead.
- The target shell already exists (`iOS/` or `Android/` directory present) — nothing to do.
- The existing `app.rs` cannot be parsed for capabilities — report which constructs were unrecognized and stop.

Examples:

```bash
# Add iOS shell to existing core-only project
vectis add-shell ios --dir /path/to/project

# Add Android shell with custom package
vectis add-shell android --dir /path/to/project \
  --android-package com.example.noteeditor
```

The `app.rs` parser extracts: the `App` struct name (from `impl App for {Name}`), capability type aliases (e.g. `type Http = crux_http::Http<Event>`), and Effect enum variants. This is a limited, structural parse — it matches known Crux patterns, not arbitrary Rust. If the app uses custom capabilities the parser doesn't recognize, it reports them as warnings and scaffolds the shell with only the recognized capabilities.

#### `vectis verify`

```
vectis verify [OPTIONS]

Options:
  --dir <path>     Project directory [default: current directory]
```

Verify auto-detects which assemblies exist and checks each one compiles. Core is always verified. iOS is verified if an `iOS/` directory exists. Android is verified if an `Android/` directory exists.

### Output Format

All subcommands produce JSON on stdout. This CLI is invoked by agent skills, not humans at a terminal, so a single structured format is sufficient.

#### `vectis init` output

```json
{
  "app_name": "Counter",
  "app_struct": "Counter",
  "project_dir": "/path/to/project",
  "assemblies": {
    "core": {
      "status": "created",
      "files": [
        "Cargo.toml",
        "clippy.toml",
        "rust-toolchain.toml",
        ".gitignore",
        "shared/Cargo.toml",
        "shared/src/lib.rs",
        "shared/src/app.rs",
        "shared/src/ffi.rs",
        "shared/src/bin/codegen.rs",
        "deny.toml",
        "supply-chain/config.toml",
        "supply-chain/audits.toml",
        "supply-chain/imports.lock"
      ]
    },
    "ios": {
      "status": "created",
      "files": ["iOS/project.yml", "iOS/Makefile", "..."]
    },
    "android": {
      "status": "created",
      "files": ["Android/build.gradle.kts", "..."]
    }
  },
  "capabilities": ["http"],
  "shells": ["ios", "android"]
}
```

#### `vectis add-shell` output

```json
{
  "app_name": "TodoApp",
  "project_dir": "/path/to/project",
  "platform": "ios",
  "source": "app.rs",
  "detected_capabilities": ["http", "kv"],
  "unrecognized_capabilities": [],
  "assembly": {
    "status": "created",
    "files": ["iOS/project.yml", "iOS/Makefile", "..."]
  }
}
```

If unrecognized capabilities are present, they appear in `unrecognized_capabilities` as warnings — the shell is still scaffolded with the recognized set.

#### `vectis verify` output

```json
{
  "project_dir": "/path/to/project",
  "passed": true,
  "assemblies": {
    "core": {
      "passed": true,
      "steps": [
        { "name": "cargo check", "passed": true },
        { "name": "cargo clippy", "passed": true },
        { "name": "cargo deny", "passed": true },
        { "name": "cargo vet", "passed": true },
        { "name": "codegen swift", "passed": true },
        { "name": "codegen kotlin", "passed": true }
      ]
    },
    "ios": {
      "passed": true,
      "steps": [
        { "name": "cargo swift package", "passed": true },
        { "name": "xcodegen", "passed": true },
        { "name": "xcodebuild sim", "passed": true }
      ]
    },
    "android": {
      "passed": false,
      "steps": [
        { "name": "make build", "passed": true },
        { "name": "gradlew cargoBuild", "passed": true },
        { "name": "gradlew assembleDebug", "passed": false,
          "error": "resource style/Theme.Counter not found" }
      ]
    }
  }
}
```

### File Manifests

The CLI generates files based on templates derived from the existing skill references. Each assembly's manifest is enumerated below.

#### Core Assembly (always generated)

Source: `core-writer` SKILL.md steps 3-9, `crux-project-config.md`, `crux-ffi-scaffolding.md`.

| Path | Parameterized by | Source reference |
| --- | --- | --- |
| `Cargo.toml` | capabilities | `crux-project-config.md` § Workspace Cargo.toml |
| `clippy.toml` | — | `crux-project-config.md` § clippy.toml |
| `rust-toolchain.toml` | — | `crux-project-config.md` § rust-toolchain.toml |
| `.gitignore` | — | `crux-project-config.md` § .gitignore |
| `shared/Cargo.toml` | capabilities | `crux-project-config.md` § Shared Crate Cargo.toml |
| `shared/src/lib.rs` | — | `crux-ffi-scaffolding.md` § lib.rs |
| `shared/src/app.rs` | app name, capabilities | `crux-app-pattern.md`, examples |
| `shared/src/ffi.rs` | app name | `crux-ffi-scaffolding.md` § ffi.rs |
| `shared/src/bin/codegen.rs` | app name | `crux-project-config.md` § Codegen Binary |
| `deny.toml` | — | dependency/license policy |
| `supply-chain/config.toml` | — | pre-configured audit criteria |
| `supply-chain/audits.toml` | — | pre-audited entries for pinned deps |
| `supply-chain/imports.lock` | — | trusted import sources |

Total: 13 files.

**Template parameterization for `app.rs`**: The generated `app.rs` produces a minimal compiling app with:

- A `Page` enum with a single `Home` variant (default)
- A `Route` enum with a single `Home` variant
- A `Model` with `page: Page` field
- A `HomeView` struct with a `message: String` field
- A `ViewModel` enum with `Loading` and `Home(HomeView)` variants
- An `Event` enum with `Navigate(Route)` plus one placeholder event per capability:
  - HTTP: `FetchData` (shell-facing) + `Fetched(Result<...>)` (internal)
  - KV: `LoadData` (shell-facing) + `Loaded(Result<...>)` (internal)
  - Time: no additional events (timer-based apps are too varied to template)
  - Platform: no additional events
- An `Effect` enum with `Render` plus one variant per capability
- Type aliases for each capability
- A skeleton `update()` that handles all events
- A `view()` that maps `Page` to `ViewModel`

This is deliberately minimal — just enough to compile and verify the full pipeline. The writer skills transform it into the real implementation via Update Mode.

#### iOS Assembly (when `--shells ios`)

Source: `ios-writer` SKILL.md steps 4-10, `ios-project-config.md`, `crux-ios-shell-pattern.md`.

| Path | Parameterized by | Source reference |
| --- | --- | --- |
| `iOS/project.yml` | app name | `ios-project-config.md` |
| `iOS/Makefile` | app name | `ios-project-config.md` |
| `iOS/{AppName}/{AppName}App.swift` | app name | `ios-writer` SKILL.md step 10 |
| `iOS/{AppName}/Core.swift` | app name, capabilities | `crux-ios-shell-pattern.md` |
| `iOS/{AppName}/ContentView.swift` | app name | `swiftui-view-patterns.md` |
| `iOS/{AppName}/Views/LoadingScreen.swift` | — | `swiftui-view-patterns.md` |
| `iOS/{AppName}/Views/HomeScreen.swift` | capabilities | `swiftui-view-patterns.md` |

Total: 7 files.

The iOS shell imports `VectisDesign` in `project.yml` but uses it conditionally — if the design system package does not resolve at build time, the app still compiles with fallback styling. This is consistent with the `ios-writer` SKILL.md which says "If the design system files do not exist, generate views without design system imports."

#### Android Assembly (when `--shells android`)

Source: `android-writer` SKILL.md steps 5-14, `android-project-config.md`, `crux-android-shell-pattern.md`.

| Path | Parameterized by | Source reference |
| --- | --- | --- |
| `Android/Makefile` | — | `android-project-config.md` |
| `Android/.gitignore` | — | `android-writer` SKILL.md step 5 |
| `Android/build.gradle.kts` | — | `android-project-config.md` |
| `Android/settings.gradle.kts` | app name | `android-project-config.md` |
| `Android/gradle.properties` | — | `android-project-config.md` |
| `Android/gradle/libs.versions.toml` | capabilities | `android-project-config.md` |
| `Android/app/build.gradle.kts` | android package, capabilities | `android-project-config.md` |
| `Android/shared/build.gradle.kts` | android package | `android-project-config.md` |
| `Android/app/src/main/AndroidManifest.xml` | app name, android package, capabilities | `android-writer` SKILL.md step 14 |
| `Android/app/src/main/res/values/themes.xml` | app name | `android-writer` SKILL.md step 14 |
| `Android/app/src/main/res/xml/network_security_config.xml` | — (only if HTTP/SSE) | `android-writer` SKILL.md step 14 |
| `Android/app/src/main/java/{pkg}/{AppName}Application.kt` | app name, android package | `android-writer` SKILL.md step 10 |
| `Android/app/src/main/java/{pkg}/MainActivity.kt` | app name, android package | `android-writer` SKILL.md step 12 |
| `Android/app/src/main/java/{pkg}/core/Core.kt` | app name, android package, capabilities | `crux-android-shell-pattern.md` |
| `Android/app/src/main/java/{pkg}/ui/screens/LoadingScreen.kt` | android package | `compose-view-patterns.md` |
| `Android/app/src/main/java/{pkg}/ui/screens/HomeScreen.kt` | android package, capabilities | `compose-view-patterns.md` |
| `Android/app/src/main/java/{pkg}/ui/theme/Color.kt` | android package | `android-writer` SKILL.md step 13 |
| `Android/app/src/main/java/{pkg}/ui/theme/Theme.kt` | app name, android package | `android-writer` SKILL.md step 13 |
| `Android/app/src/main/java/{pkg}/ui/theme/Type.kt` | android package | `android-writer` SKILL.md step 13 |

Total: 19 files (plus the Gradle wrapper files generated by `gradle wrapper`).

Where `{pkg}` is the Android package path (e.g., `com/vectis/counter` for `com.vectis.counter`).

If HTTP or SSE capabilities are present, `HttpClient.kt` and/or `SseClient.kt` are also generated in `core/`, and `network_security_config.xml` is included. If KV is present, `KeyValueClient.kt` is generated. If more than one non-Render effect exists, Koin DI files (`di/AppModule.kt`) are also generated.

### Prerequisite Detection

Every `vectis` subcommand that scaffolds or verifies code depends on external toolchains. If a required tool is missing, the CLI must **stop immediately** and report the problem — never attempt partial work that will fail in confusing ways downstream.

#### Workstation Requirements

The full set of tools a developer needs on their workstation, by assembly:

| Assembly | Tool | How to check | Install |
| --- | --- | --- | --- |
| **Core** | `rustup` + stable toolchain | `rustup show active-toolchain` | [rustup.rs](https://rustup.rs) |
| **Core** | `cargo-deny` | `cargo deny --version` | `cargo install cargo-deny` |
| **Core** | `cargo-vet` | `cargo vet --version` | `cargo install cargo-vet` |
| **iOS** | Xcode + Command Line Tools | `xcode-select -p` | Mac App Store |
| **iOS** | `xcodegen` | `xcodegen --version` | `brew install xcodegen` |
| **iOS** | `cargo-swift` | `cargo swift --version` | `cargo install cargo-swift` |
| **iOS** | `xcbeautify` | `xcbeautify --version` | `brew install xcbeautify` |
| **Android** | Android SDK (`$ANDROID_HOME`) | `echo $ANDROID_HOME` | [Android Studio](https://developer.android.com/studio) |
| **Android** | Java 21 | `java --version` | [Adoptium](https://adoptium.net) or `brew install openjdk@21` |
| **Android** | Rust Android targets | `rustup target list --installed` | `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android` |
| **Android** | Android NDK (via SDK Manager) | `ls $ANDROID_HOME/ndk/` | SDK Manager in Android Studio |
| **Android** | `gradle` (for initial wrapper) | `gradle --version` | `brew install gradle` |

#### Detection Behavior

Before performing any work, `vectis init`, `vectis add-shell`, and `vectis verify` run a prerequisite check for the assemblies they will touch. The check is scoped — core prerequisites are always checked, iOS prerequisites only when iOS is involved, Android prerequisites only when Android is involved.

If any prerequisite is missing, the CLI outputs a structured error and exits with a non-zero code. No files are written, no commands are run.

```json
{
  "error": "missing_prerequisites",
  "missing": [
    {
      "tool": "xcodegen",
      "assembly": "ios",
      "check": "xcodegen --version",
      "install": "brew install xcodegen"
    },
    {
      "tool": "cargo-swift",
      "assembly": "ios",
      "check": "cargo swift --version",
      "install": "cargo install cargo-swift"
    }
  ],
  "message": "Install the missing tools above and re-run the command."
}
```

This is a hard stop, not a warning. The rationale: partial scaffolding that cannot be verified is worse than no scaffolding, because the developer inherits a broken project with no clear diagnosis. The CLI's value proposition is "one command, working project" — a project that can't compile on first run violates that promise.

The prerequisite check also validates version minimums where they matter (e.g., Java 21, not Java 17) and reports the found version alongside the required version.

### Verify Pipeline

The `vectis verify` command runs the compilation chain for each detected assembly. Prerequisites are checked first (see above) — if the toolchain is incomplete, verify reports the missing tools and stops before running any build commands. This cleanly separates "your toolchain is incomplete" from "the generated project is broken."

Verify stops at the first failure within each assembly but checks all assemblies independently.

#### Core (always)

1. `cargo check` — type checking
2. `cargo clippy --all-targets` — lint checking
3. `cargo deny check` — dependency and license audit
4. `cargo vet` — supply-chain audit
5. `cargo run --bin codegen --features codegen,facet_typegen -- --language swift --output-dir /tmp/vectis-verify-swift` — codegen for Swift
6. `cargo run --bin codegen --features codegen,facet_typegen -- --language kotlin --output-dir /tmp/vectis-verify-kotlin` — codegen for Kotlin

Steps 5 and 6 use temporary directories to avoid polluting the project. They verify the codegen binary compiles and runs successfully.

#### iOS (if `iOS/` exists)

1. `make typegen` — generate SharedTypes Swift package
2. `make package` — build Shared UniFFI Swift package via `cargo swift`
3. `make xcode` — generate Xcode project via `xcodegen`
4. `xcodebuild build` — simulator build (same flags as `make sim-build`)

#### Android (if `Android/` exists)

1. Generate Gradle wrapper if missing (`gradle wrapper --gradle-version 8.13`)
2. Create `local.properties` with `sdk.dir` if missing
3. `make build` — generate Kotlin types via codegen
4. `./gradlew :shared:cargoBuild` — cross-compile Rust for Android ABIs
5. `./gradlew :app:assembleDebug` — build the APK

### Version Management

The goal is not "always use latest" — it is "easy to update to latest, impossible to use an incoherent set." Scaffolding happens once per project, but version maintenance happens for the life of the project.

#### `versions.toml`

Version pins live in an external TOML file rather than being embedded in the CLI binary. This separates the CLI's release cadence from the dependency ecosystem's release cadence.

```toml
# ~/.config/vectis/versions.toml
# Managed by `vectis update-versions`. Manual edits are valid.

[crux]
crux_core = "0.17.0"
crux_http = "0.16.0"
crux_kv = "0.11.0"
crux_time = "0.15.0"
crux_platform = "0.8.0"
facet = "=0.31"
uniffi = "=0.29.4"
serde = "1.0"

[android]
compose-bom = "2025.01.01"
koin = "4.0.4"
ktor = "3.1.1"
kotlin = "2.1.10"
agp = "8.8.2"
gradle = "8.13"

[ios]
# iOS shells depend on generated packages (SharedTypes, Shared) and
# optionally VectisDesign — all internal. External SPM dependencies
# would be pinned here if introduced.

[tooling]
cargo-deny = "0.19.1"
xcodegen = "2.42.0"
```

The file covers all three ecosystems. Crux crates are tightly coupled (a `crux_core` release dictates compatible `facet` and `uniffi` versions). Android uses Compose BOM to pin a coherent Compose version set, but Koin and ktor are independently versioned. iOS has negligible external surface today.

#### Resolution order

The CLI resolves version pins at runtime using the following precedence:

1. **`--version-file <path>`** — explicit override for enterprise or locked environments
2. **`versions.toml` in the project directory** — project-local pins (committed to source control)
3. **`~/.config/vectis/versions.toml`** — user-level pins (managed by `update-versions`)
4. **Embedded defaults** — compiled into the binary as a fallback so the CLI works without any config file

This means `vectis init` is deterministic for a given version file, works offline with no config at all (embedded defaults), and supports project-level pinning when teams need reproducible scaffolding.

#### `vectis update-versions`

```
vectis update-versions [OPTIONS]

Options:
  --version-file <path>   File to update [default: ~/.config/vectis/versions.toml]
  --dry-run               Show proposed changes without writing
  --verify                Scaffold a scratch project and run `vectis verify` before committing pins
```

The update process resolves latest stable versions while preserving coherence:

1. **Crux crates**: Query crates.io for the latest `crux_core` release. Read its published `Cargo.toml` to extract compatible `facet`, `uniffi`, and capability crate versions from its dependency tree. These are a coupled set — they move together or not at all.
2. **Android**: Query Maven Central for the latest Compose BOM. The BOM itself pins the coherent set of `compose-ui`, `compose-material3`, `compose-runtime` versions — no independent resolution needed. Query Koin, ktor, Kotlin, and AGP releases independently.
3. **iOS**: No external dependencies to resolve today. If external SPM dependencies are introduced, query GitHub tags for latest releases.
4. **Tooling**: Query crates.io for `cargo-deny`, Homebrew or GitHub for `xcodegen`.
5. **Validation** (when `--verify` is passed): Scaffold a temporary project with the proposed pins, run `vectis verify`, and only commit the new `versions.toml` if all assemblies compile. This catches incompatibilities before they reach real projects.

Without `--verify`, the command trusts the registry metadata. With `--verify`, it proves the pins work end-to-end. The former is fast (seconds); the latter is thorough (minutes, requires all platform toolchains).

`--dry-run` output shows a diff of current vs. proposed pins:

```
crux.crux_core: 0.17.0 → 0.18.0
crux.facet: =0.31 → =0.32
crux.uniffi: =0.29.4 → =0.30.0
android.compose-bom: 2025.01.01 → 2025.04.01
android.kotlin: 2.1.10 → 2.1.20
(all others unchanged)
```

### Template Maintenance

Moving scaffolding from agent prose to embedded templates shifts the maintenance burden — it does not eliminate it. When upstream dependencies change (a new Crux release alters the FFI pattern, `facet` introduces a new codegen mode, Compose BOM drops a deprecated API), someone must update the template files, the conditional logic in the template modules, and the version pins. This is an ongoing cost for the life of the CLI.

The `update-versions` subcommand handles the version pin side of this problem (query registries, prove coherence via `--verify`). But structural template changes — a new required import, a renamed trait, a changed build flag — require manual intervention. Today that intervention happens in three skill reference documents; after RFC-6 it happens in the template files and their corresponding Rust modules. The improvement is that changes are centralized (one place instead of three) and verifiable (`vectis init` + `vectis verify` on a scratch project proves the templates still produce compiling output).

The remaining gap is detection: how does the team know a template needs updating in the first place?

#### `template-updater` Agent Skill

A new agent skill in the vectis plugin (`plugins/vectis/skills/template-updater/`) automates the detection-and-update cycle:

1. **Detect**: Given a new `versions.toml` (produced by `update-versions`), the skill scaffolds a scratch project with the new pins and runs `vectis verify`. If verification fails, it has concrete compiler errors to work from.

2. **Diagnose**: The skill reads the compiler errors and diffs the changelog or migration guide for the updated crate (e.g., the `crux_core` CHANGELOG on GitHub). It maps each error to a template file and identifies the required change.

3. **Update**: The skill edits the template files in `templates/vectis/` and the conditional logic in `crates/vectis-cli/src/templates/` to fix the compilation errors. It re-runs `vectis verify` after each edit to confirm the fix.

4. **Validate**: Once `vectis verify` passes on the scratch project, the skill runs `vectis verify` against a matrix of capability combinations (render-only, http, http+kv, all caps) to ensure no conditional path is broken.

5. **Report**: The skill outputs a summary of what changed and why, suitable for a commit message or PR description.

This skill is judgment-appropriate work — interpreting compiler errors, reading changelogs, deciding how to modify templates — which is exactly the kind of task the agent handles well. The deterministic scaffolding and verification remain in the CLI; the adaptive maintenance stays with the agent.

The workflow for a version bump becomes:

```bash
vectis update-versions --dry-run         # see what changed
vectis update-versions                   # commit new pins
# invoke template-updater skill          # fix any template breakage
vectis update-versions --verify          # prove everything compiles
```

### Workspace Layout

The CLI lives in this repository as a Cargo workspace member. It is independent of the `specify` CLI from [RFC-1](archive/rfc-1-cli.md) — different binary, different purpose, different lifecycle. If RFC-1 is implemented first, the two can share a workspace; if not, `vectis` establishes its own.

```
specify/                              # repo root
├── Cargo.toml                        # workspace manifest
├── Cargo.lock
├── crates/
│   └── vectis-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs               # clap dispatch
│           ├── error.rs              # unified error types
│           ├── prerequisites.rs      # toolchain detection and reporting
│           ├── init.rs               # init subcommand orchestration
│           ├── add_shell.rs          # add-shell subcommand (app.rs parser + shell scaffold)
│           ├── verify.rs             # verify subcommand orchestration
│           └── templates/
│               ├── mod.rs            # template registry + parameterization
│               ├── core.rs           # core assembly templates
│               ├── ios.rs            # iOS assembly templates
│               └── android.rs        # Android assembly templates
├── templates/
│   └── vectis/                       # raw template files (included via include_str!)
│       ├── core/
│       │   ├── workspace-cargo.toml
│       │   ├── shared-cargo.toml
│       │   ├── app.rs
│       │   ├── ffi.rs
│       │   ├── lib.rs
│       │   ├── codegen.rs
│       │   ├── clippy.toml
│       │   ├── rust-toolchain.toml
│       │   ├── gitignore
│       │   ├── deny.toml
│       │   ├── supply-chain-config.toml
│       │   ├── supply-chain-audits.toml
│       │   └── supply-chain-imports.lock
│       ├── ios/
│       │   ├── project.yml
│       │   ├── Makefile
│       │   ├── App.swift
│       │   ├── Core.swift
│       │   ├── ContentView.swift
│       │   ├── LoadingScreen.swift
│       │   └── HomeScreen.swift
│       └── android/
│           ├── Makefile
│           ├── root-build.gradle.kts
│           ├── settings.gradle.kts
│           ├── gradle.properties
│           ├── libs.versions.toml
│           ├── app-build.gradle.kts
│           ├── shared-build.gradle.kts
│           ├── AndroidManifest.xml
│           ├── themes.xml
│           ├── network-security-config.xml
│           ├── Application.kt
│           ├── MainActivity.kt
│           ├── Core.kt
│           ├── LoadingScreen.kt
│           ├── HomeScreen.kt
│           ├── Color.kt
│           ├── Theme.kt
│           ├── Type.kt
│           └── gitignore
├── plugins/                          # existing — unchanged
├── schemas/                          # existing — unchanged
├── scripts/                          # existing — unchanged
└── Makefile                          # updated with vectis targets
```

Templates use simple placeholder substitution rather than a template engine. Placeholders follow the pattern `__PLACEHOLDER__` (double-underscore delimited, UPPER_SNAKE_CASE). This delimiter is chosen to avoid collision with Rust's `{}` format strings, Swift's `\()` interpolation, and Kotlin's `${}` templates — all of which appear in the generated source files.

| Placeholder | Example value | Used in |
| --- | --- | --- |
| `__APP_NAME__` | `Counter` | All assemblies |
| `__APP_STRUCT__` | `Counter` | Core `app.rs`, `ffi.rs`, `codegen.rs` |
| `__APP_NAME_LOWER__` | `counter` | Android package, iOS bundle ID |
| `__ANDROID_PACKAGE__` | `com.vectis.counter` | Android files |
| `__ANDROID_PACKAGE_PATH__` | `com/vectis/counter` | Android directory structure |
| `__CRUX_CORE_VERSION__` | `0.17.0` | `Cargo.toml` |
| `__FACET_VERSION__` | `=0.31` | `Cargo.toml` |
| `__UNIFFI_VERSION__` | `=0.29.4` | `Cargo.toml` |
| `__SERDE_VERSION__` | `1.0` | `Cargo.toml` |

Capability-dependent sections use conditional blocks in the template modules (Rust code that includes or excludes sections based on the `--caps` flags), not template-language conditionals.

### Dependencies

```toml
# crates/vectis-cli/Cargo.toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

No async runtime, no HTTP client, no template engine. The binary should compile in seconds and produce a small static binary. Template files are embedded via `include_str!` at compile time.

### Makefile Integration

```makefile
.PHONY: build-vectis checks dev-plugins prod-plugins

build-vectis:
	cargo build --release --package vectis-cli
	cp target/release/vectis .

checks:
	@$(DENO) run --allow-read scripts/checks.ts
```

The `vectis` binary is built separately from `make checks`. Skills reference it by absolute path or expect it on `$PATH`.

## Skill Integration

The key insight: writer skills already have a mode detection step. The CLI integration adds a single branch to that detection — when the project is greenfield, invoke `vectis init` before proceeding to Update Mode.

### `core-writer` Changes

Current mode detection (SKILL.md § Mode Detection):

> Check for `{project-dir}/shared/src/app.rs`. If the file exists, switch to update mode. If not, proceed with create mode.

New mode detection:

> Check for `{project-dir}/shared/src/app.rs`. If the file exists, switch to update mode. If not, run:
>
> ```bash vectis init {AppName} --dir {project-dir} --caps {detected-caps} vectis verify --dir {project-dir}
> ```
>
> If both commands succeed, switch to update mode and apply feature-specific changes from the Specify artifacts. If `vectis verify` fails, report the errors and stop.

The agent still reads the spec and design artifacts (Create Mode step 1) to determine the app name and capabilities. It passes these to `vectis init`. The remaining 12 steps of Create Mode (steps 2-13) are replaced by the single CLI invocation followed by Update Mode.

### `ios-writer` Changes

Current mode detection:

> Check for `{project-dir}/Core.swift` or `{project-dir}/*/Core.swift` to detect the mode.

New mode detection:

> Check for `{project-dir}/Core.swift` or `{project-dir}/*/Core.swift`. If found, switch to update mode. If not:
>
> 1. Check whether core exists (`{app-dir}/shared/src/app.rs`). If not, run `vectis init` with `--shells ios` (this scaffolds both core and the iOS shell).
> 2. If core already exists but iOS does not, run `vectis add-shell ios --dir {app-dir}`.
> 3. Run `vectis verify --dir {app-dir}` to confirm the iOS assembly compiles.
> 4. Switch to update mode.

### `android-writer` Changes

Same pattern as `ios-writer` but with `vectis add-shell android`.

### Greenfield Detection in Build Orchestration

The build orchestration layer (which coordinates writer skills in sequence: design-system-writer -> core-writer -> ios-writer -> android-writer) can detect a greenfield project once and invoke `vectis init` with all requested shells, rather than having each skill independently check and invoke the CLI. This is more efficient:

```bash
# Single invocation scaffolds everything
vectis init TodoApp --dir . --caps http,kv --shells ios,android
vectis verify --dir .
```

Then each writer skill detects existing code and enters Update Mode directly.

## Future Work

### Design System Integration

The current templates include VectisDesign references conditionally. A future `--design-system <path>` flag could generate design-system-aware shells from the start, reading `tokens.yaml` to populate color/typography constants.

## Alternatives Considered

**Agent-only approach (status quo).** Continue having the agent interpret prose instructions to scaffold projects. Rejected because the output is deterministic, the process is expensive, and version pins require exact precision that LLMs handle unreliably.

**`cargo-generate` templates.** Use the existing Rust template ecosystem. Rejected because (a) the iOS and Android assemblies are not Cargo projects, so `cargo-generate` cannot scaffold them; (b) the capability-dependent conditional logic (which Effect variants, which Gradle dependencies, which imports) is better expressed in Rust code than in template conditionals; and (c) a custom binary can include the `verify` step.

**Separate repositories per assembly.** Maintain three template repositories (core, iOS, Android) and compose them. Rejected because the assemblies have tight coupling (version pins, app name, capability set must be consistent) and a single binary enforces this consistency.

**Extend `specify` CLI with `specify vectis init`.** Make this a subcommand of the RFC-1 CLI. Rejected because the `specify` CLI owns Specify workflow operations (validation, merge, task tracking) while `vectis` owns Crux project scaffolding — different concerns, different release cadences. They can share a Cargo workspace without sharing a binary.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — the parallel effort for Specify workflow operations
- [RFC-1a: Deferred Validation](archive/rfc-1a-validation.md) — the Pass/Fail/Deferred model that inspires the verify output
- `plugins/vectis/skills/core-writer/SKILL.md` — Create Mode steps 1-13
- `plugins/vectis/skills/ios-writer/SKILL.md` — Create Mode steps 1-11
- `plugins/vectis/skills/android-writer/SKILL.md` — Create Mode steps 1-15
- `plugins/vectis/skills/core-writer/references/crux-versions.md` — version pins
- `plugins/vectis/skills/core-writer/references/crux-project-config.md` — project layout and manifests
- `plugins/vectis/skills/core-writer/references/crux-ffi-scaffolding.md` — FFI templates
- `plugins/vectis/skills/ios-writer/references/ios-project-config.md` — iOS build configuration
- `plugins/vectis/skills/ios-writer/references/crux-ios-shell-pattern.md` — iOS bridge pattern
- `plugins/vectis/skills/android-writer/references/android-project-config.md` — Android build configuration
- `plugins/vectis/skills/android-writer/references/crux-android-shell-pattern.md` — Android bridge pattern
