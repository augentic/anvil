# Vectis -- Crux Application Development

Build cross-platform applications with a shared Rust core and native platform shells using the [Crux](https://github.com/redbadger/crux) framework.

## Why Crux

- Support multiple runtime platforms -- iOS, Android, Web, macOS, Linux, Windows -- from a single shared core.
- All application behavior lives in the shared core, testable independently of the runtime platform.
- An opinionated application structure that is well-suited to AI-assisted code generation.

Crux is written in Rust and documented at [docs.rs/crux_core](https://docs.rs/crux_core/latest/crux_core/).

## Prerequisites

### Rust Toolchain

- [Install Rust](https://rust-lang.org/tools/install/)
- [Install Cursor](https://cursor.com/home)
- Install the [Rust Analyzer](https://open-vsx.org/extension/rust-lang/rust-analyzer) Cursor extension

### iOS / macOS Development

Only required if you are building an iOS shell. Future platform shells will have their own prerequisites.

[Install Xcode command line tools](https://developer.apple.com/documentation/xcode/installing-the-command-line-tools/)

```shell
# Builder for Swift projects without needing Xcode UI
brew install xcode-build-server

# Pretty print formatter for xcodebuild command output in Cursor terminal
brew install xcbeautify

# Allow for advanced formatting and language features
brew install swiftformat

# Generate Xcode projects from declarative YAML (project.yml)
brew install xcodegen

# Build Rust static library as a Swift Package with XCFramework
cargo install cargo-swift
```

Install the [Swift Language Support](https://open-vsx.org/extension/chrisatwindsurf/swift-vscode) and [SweetPad](https://marketplace.visualstudio.com/items?itemName=SweetPad.sweetpad) Cursor extensions for Swift editing and Xcode integration.

## Creating a Crux App

App generation uses the Specify workflow with the `vectis` schema. Each app is a Specify **change** that produces a proposal, specs, design, and tasks. The build phase invokes the `core-writer` skill to generate a buildable `shared` crate with business logic, state management, side-effect orchestration, and tests.

### Define a new app

Describe what you want to build and Specify generates all artifacts:

> `/spec:define` -- A weather app that fetches 5-day forecasts from a REST API and displays them in a scrollable list. It should cache the last fetch in Key-Value storage for offline use.

Or start interactively and let the agent ask clarifying questions:

> `/spec:define`

Specify produces four artifacts in dependency order:

| Artifact      | Purpose                                                   |
| ------------- | --------------------------------------------------------- |
| `proposal.md` | App concept, motivation, module names and types            |
| `specs/*.md`  | Behavioral requirements with scenarios                     |
| `design.md`   | Crux type system, capabilities, API contracts, constraints |
| `tasks.md`    | Implementation checklist for skill invocation              |

Review the artifacts in `.specify/changes/<change-name>/`. Edit them by hand or ask the agent to revise before proceeding.

### Build

> `/spec:build`

The agent works through the tasks: invokes the `core-writer` skill to generate the `shared` crate, verifies with `cargo check`, `cargo test`, and `cargo clippy`, then runs the `core-reviewer` skill. The code review covers three passes:

- **Structural** -- missing `render()` calls, serde derives, input validation
- **Logic** -- state machine completeness, operation coalescing, race conditions, conflict-resolution gaps, spec gap detection
- **Quality** -- `unwrap()`/`expect()` in production, error handling, function length

Critical and Warning findings are addressed before proceeding.

### Merge

Once you are satisfied with the output:

> `/spec:merge`

This merges the change's specs into the project baseline at `.specify/specs/` and archives the change.

### Update an existing app

To modify an app that was previously generated:

1. Define a new change describing the update:
   > `/spec:define` -- Add dark mode support to the weather app

2. In the specs, provide updated or new behavioral requirements. The `core-writer` skill compares the specs against the existing code and makes targeted edits in update mode.

3. Build and verify as above.

### Check status

> `/spec:status`

Shows active changes, artifact completion, and task progress.

## Spec Format

The specs artifact follows a structured markdown format. Required elements for core modules:

| Section            | What to include                                                         |
| ------------------ | ----------------------------------------------------------------------- |
| **Purpose**        | One-line summary of the app or module                                   |
| **Requirements**   | Every user action and its expected outcome, with scenarios              |
| **Error Conditions** | Error states and recovery behavior                                   |
| **Metrics**        | Observable metrics (optional)                                           |

The design document captures the technical contract:

| Section            | What to include                                                         |
| ------------------ | ----------------------------------------------------------------------- |
| **Context**        | Module types in scope and their relationships                           |
| **Domain Model**   | The internal state the app tracks (Model fields)                        |
| **Type System**    | Event variants, ViewModel enum, Effect enum, Route enum                 |
| **Capabilities**   | Which external capabilities the app needs (see table below)             |
| **API Details**    | HTTP endpoints, methods, request/response shapes. Omit if no HTTP       |
| **Constraints**    | Implementation constraints (Crux version, uniffi pin, etc.)             |

### Capabilities

The skill detects which Crux capabilities your app needs from the **Capabilities** section of your design:

| Capability                     | When to include                                           |
| ------------------------------ | --------------------------------------------------------- |
| **Render**                     | Always included automatically                             |
| **HTTP** (`crux_http`)         | App calls a REST API or any remote endpoint               |
| **Key-Value** (`crux_kv`)      | App persists data locally (offline storage, caching)      |
| **Time** (`crux_time`)         | App uses timers, delays, intervals, or scheduling         |
| **Platform** (`crux_platform`) | App needs to detect the runtime platform or OS            |
| **SSE / Streaming** (custom)   | App subscribes to server-sent events or live data streams |

## What Gets Generated

A core module produces these files:

| Artifact                      | Description                                                                                |
| ----------------------------- | ------------------------------------------------------------------------------------------ |
| `Cargo.toml` (workspace root) | Workspace manifest with pinned Crux git dependencies                                       |
| `clippy.toml`                 | Clippy configuration for allowed duplicate crates                                          |
| `rust-toolchain.toml`         | Rust toolchain targeting iOS, Android, macOS, and WASM                                     |
| `spec.md`                     | Copy of the specification used to generate (or update) the core                            |
| `shared/Cargo.toml`           | Crate manifest with detected capabilities and feature gates                                |
| `shared/src/app.rs`           | App trait implementation: Model, Event, ViewModel, Effect, `update()`, `view()`, and tests |
| `shared/src/ffi.rs`           | FFI scaffolding for UniFFI and wasm-bindgen                                                |
| `shared/src/lib.rs`           | Module wiring and re-exports                                                               |

Custom capability modules (e.g. `shared/src/sse.rs` for Server-Sent Events) are generated when needed.

## Creating an iOS Shell

iOS shell generation uses the same Specify workflow. Each shell is a change that produces a SwiftUI application wired to the Crux core via UniFFI. Only iOS shells are currently supported; other platform shells will be added in future.

### Define

Define a change with an `ios-shell` module:

> `/spec:define` -- Create an iOS shell for the weather app. Put it in `ios/`.

The agent produces artifacts describing the shell:

| Artifact        | Purpose                                                             |
| --------------- | ------------------------------------------------------------------- |
| `proposal.md`   | Which app, target directory, design system notes                    |
| `specs/*.md`    | iOS-specific behavioral requirements (navigation, gestures)         |
| `design.md`     | Navigation style, screen customizations, platform features          |
| `tasks.md`      | Implementation checklist -- invoke ios-writer, verify build, review |

### Build

> `/spec:build`

The agent invokes the `ios-writer` skill, which reads the Crux core's `app.rs` to extract ViewModel, Event, Effect, and Route types. It generates:

- `project.yml` -- XcodeGen project configuration
- `Makefile` -- build automation
- `Core.swift` -- bridge between SwiftUI and the Rust core
- SwiftUI screen views -- one per ViewModel variant
- App entry point with navigation

All views use the shared `VectisDesign` package for colors, typography, and spacing tokens.

After generation, the `ios-reviewer` skill reviews the shell for:

- **Structural** -- missing screen views, incomplete effect handlers
- **Quality** -- concurrency safety, accessibility, design system compliance
- **Integration** -- Core.swift correctness, build configuration

## Design System

The design system provides platform-agnostic design tokens with platform-specific implementations. Currently only an iOS Swift Package is generated.

| Path                        | Purpose                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `design-system/spec.md`     | Semantic color roles, typography scale, spacing rules, usage guidance |
| `design-system/tokens.yaml` | Concrete token values (single source of truth for code generation)    |
| `design-system/ios/`        | `VectisDesign` Swift Package -- generated from `tokens.yaml`          |

The design system is shared across all apps generated by the ios-writer skill. Future platform shells (Android, Web) will add their own implementations under `design-system/` using the same tokens.

### Updating the Design System

Design system updates follow a three-layer flow:

```
spec.md (describes intent) → tokens.yaml (defines values) → iOS Swift code (generated)
```

**1. Decide what to change.** Read `design-system/spec.md` to understand the current token roles and usage rules. The spec describes the *why* behind each token.

**2. Edit `tokens.yaml`.** This is the single source of truth for all concrete values. Common changes:

- **Change a value** -- edit the token's entry (e.g. change `primary.light` from `"#007AFF"` to `"#0066CC"`)
- **Add a token** -- add a new entry under an existing category, following the naming conventions in `spec.md`
- **Add a category** -- add a new top-level key (e.g. `elevation`) with entries that follow one of the three value shapes: color (`light`/`dark`), font (`size`/`weight`), or scalar (plain number)
- **Remove a token** -- delete the entry; check downstream shells for references before removing

**3. Update `spec.md`** if the change is semantic (new roles, changed usage rules, new categories). For pure value tweaks (adjusting a hex color), the spec usually stays the same.

**4. Regenerate the iOS code.** Use the `design-system-writer` skill:

> Use the design-system-writer skill to regenerate the iOS design system

The skill reads `tokens.yaml` and overwrites the Swift files under `design-system/ios/Sources/VectisDesign/`. It then runs `swift build` to verify the package compiles.

The generated Swift files carry a "do not edit manually" comment. All customization goes through `tokens.yaml`.

### Design System Examples

**Change the primary color:**

1. Edit `design-system/tokens.yaml`:

```yaml
colors:
  primary:
    light: "#0066CC"    # was #007AFF
    dark: "#0A84FF"
```

2. Regenerate:
   > Use the design-system-writer skill to regenerate the iOS design system

**Add a tertiary color role:**

1. Update `design-system/spec.md` to document the new role and its purpose.
2. Add entries to `design-system/tokens.yaml`:

```yaml
colors:
  # ... existing entries ...
  tertiary:
    light: "#34C759"
    dark: "#30D158"
  tertiaryContainer:
    light: "#D4F5DD"
    dark: "#0A3D1A"
  onTertiary:
    light: "#FFFFFF"
    dark: "#FFFFFF"
  onTertiaryContainer:
    light: "#0A3D1A"
    dark: "#D4F5DD"
```

3. Regenerate with the design-system-writer skill.

**Add a new token category (e.g. elevation):**

1. Document the category in `design-system/spec.md`.
2. Add a new top-level key to `design-system/tokens.yaml`:

```yaml
elevation:
  none: 0
  sm: 2
  md: 4
  lg: 8
  xl: 16
```

3. Regenerate. The skill detects the scalar value shape, creates `Elevation.swift` with a `VectisElevation` enum, and adds it to `Theme.swift`.

## Working with Xcode

After generating an iOS shell, the directory contains a `project.yml` (XcodeGen spec) and a `Makefile` but no `.xcodeproj` yet. The Xcode project file is generated and gitignored; `project.yml` is the source of truth.

### First-time setup

```bash
cd path/to/ios
make build
```

This runs three phases: `typegen` (generates SharedTypes Swift package from domain types), `package` (builds the Shared Swift package via cargo-swift), then `xcode` (generates the `.xcodeproj` via XcodeGen).

### Open in Xcode

```bash
open MyApp.xcodeproj
```

The project name matches the app name declared in `project.yml`. From here you can build, run on a simulator, and use SwiftUI previews.

### Common mistakes to avoid

- Do **not** look for a `.xcworkspace` -- the ios-writer does not generate one. The single `.xcodeproj` references the generated Swift packages as dependencies.
- If Xcode gets into a bad state or creates stray scaffolding files, delete the `.xcodeproj` and regenerate:

```bash
rm -rf MyApp.xcodeproj
make xcode
```

Because the project file is fully derived from `project.yml`, this is always safe.

### Build from the command line

```bash
make build      # builds for iPhone 16 simulator via xcodebuild
make sim-build  # simulator-only build for verification
```

## Reviewing Generated Code

### Core review

The `core-reviewer` skill reviews Crux core (Rust `shared` crate) code for issues that compilers and linters miss. It runs automatically as part of the build phase but can also be invoked standalone:

> Use the core-reviewer skill to review `path/to/my-app`

The skill applies checks across three categories (structural, logic, and quality) and produces a severity-graded report.

### iOS review

The `ios-reviewer` skill reviews iOS shell code for structural and quality issues. It also runs automatically during the build phase and can be invoked standalone:

> Use the ios-reviewer skill to review `path/to/my-app`

## Skills Reference

| Skill | Purpose |
| ----- | ------- |
| `core-writer` | Generate or update the Rust Crux shared crate from Specify artifacts |
| `core-reviewer` | Review Crux core for structural, logic, and quality issues |
| `ios-writer` | Generate or update the SwiftUI iOS shell from the Crux core |
| `ios-reviewer` | Review iOS shell for structural and quality issues |
| `design-system-writer` | Generate VectisDesign Swift package from `tokens.yaml` |

See [plugins.md](plugins.md) for the full plugin and skill reference.
