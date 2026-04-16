# RFC-5 Implementation Tasks

> Source RFC: [rfc-5-vectis-bootstrap.md](rfc-5-vectis-bootstrap.md)
> Owner: vectis CLI binary and the writer skills it replaces
> Scope: **vectis only**. A future top-level workspace CLI (RFC-1) will be folded in as a sibling member; nothing in this plan should require touching anything outside the vectis surface.

## How to Use This File

Each chunk is a self-contained agent session. Per session:

1. Read this file's status table.
2. Pick the next `[ ]` chunk whose dependencies are all `[x]`.
3. Read **only** the RFC sections and reference docs listed in that chunk.
4. Work on a branch named `vectis-cli/chunk-<N>-<slug>`.
5. Run the chunk's verification commands. They are gates, not suggestions.
6. Update the status row, append any deviations to the chunk's "Notes", commit, push, open PR.
7. Do **not** expand scope. If you discover work belonging to a later chunk, add it to that chunk's "Notes" column and stop.

## Status

| # | Chunk | Status | Branch | Notes |
|---|-------|--------|--------|-------|
| 1 | Workspace + CLI skeleton | [x] | `vectis-cli` | Branch name `vectis-cli/chunk-1-skeleton` collides with existing `vectis-cli` parent branch (git refs are hierarchical); committed on `vectis-cli` instead. Future chunks must use a non-prefix name (e.g. `vectis-cli-chunk-N-slug`) or the parent branch must be renamed/deleted. Dispatcher uses a `CommandOutcome::{Success,Stub}` enum so handlers can stay stubbed without polluting `VectisError`. `VectisError` carries `#[allow(dead_code)]` until chunks 2/5/9 start constructing the unused variants. |
| 2 | Prerequisites detection | [ ] | | When `MissingPrerequisites` is first constructed, narrow the `#[allow(dead_code)]` on `VectisError` (or drop it). |
| 3a | Templates: core extraction | [ ] | | |
| 3b | Templates: iOS extraction | [ ] | | |
| 3c | Templates: Android extraction | [ ] | | |
| 4 | Version resolution + embedded defaults | [ ] | | `embedded/versions.toml` lives at `crates/vectis-cli/embedded/`, so `include_str!("../embedded/versions.toml")` resolves from any file inside `src/`. |
| 5 | `vectis init` core, render-only | [ ] | | Handler must return `Ok(CommandOutcome::Success(value))`, not `Ok(value)`; remove the `Stub` return path. The `Init(_)` arm in `main::main` will need to pass `&InitArgs` through to `init::run` (currently discarded with `_`). |
| 6 | `vectis init` core capability variants | [ ] | | |
| 7 | `vectis init` iOS shell | [ ] | | Same handler-signature note as chunk 5: replace the `Stub` return with `Success` and pass `&InitArgs` through. |
| 8 | `vectis init` Android shell | [ ] | | Same handler-signature note as chunks 5/7. |
| 9 | `vectis verify` | [ ] | | Same handler-signature note as chunk 5; pass `&VerifyArgs`. Will start constructing `VectisError::Verify`. |
| 10 | `vectis add-shell` (incl. `app.rs` parser) | [ ] | | Same handler-signature note as chunk 5; pass `&AddShellArgs`. |
| 11 | `vectis update-versions` | [ ] | | Same handler-signature note as chunk 5; pass `&UpdateVersionsArgs`. |
| 12 | Writer skill rewrites | [ ] | | |
| 13 | `template-updater` skill | [ ] | | |

Dependency graph (chunk N depends on chunks listed):

```
1  ─┬─ 2 ──┐
   │       │
   ├─ 3a ──┼─ 5 ── 6 ─┬─ 7 ─┐
   ├─ 3b ──┘          │     ├─ 9 ── 10 ── 11 ── 12 ── 13
   ├─ 3c ─────────────┴─ 8 ─┘
   └─ 4  ── 5
```

## Folder Convention (set in chunk 1, do not deviate)

This work assumes a top-level CLI sibling will land later. Layout is chosen so that adding `crates/specify-cli/` and `templates/specify/` is purely additive:

```
specify/                          # repo root (existing)
├── Cargo.toml                    # workspace manifest — declares only crates/vectis-cli today
├── Cargo.lock                    # generated
├── crates/
│   └── vectis-cli/               # this work
│       ├── Cargo.toml
│       ├── embedded/
│       │   └── versions.toml     # compiled in via include_str!
│       └── src/
│           ├── main.rs           # clap dispatch
│           ├── error.rs
│           ├── prerequisites.rs
│           ├── versions.rs
│           ├── init/             # subcommand modules as folders once they grow
│           ├── add_shell/
│           ├── verify/
│           ├── update_versions/
│           └── templates/
│               ├── mod.rs        # template engine (placeholder + cap-conditional)
│               ├── core.rs
│               ├── ios.rs
│               └── android.rs
├── templates/
│   └── vectis/                   # raw template files
│       ├── core/
│       ├── ios/
│       └── android/
├── plugins/                      # existing — only chunk 12 touches these
├── schemas/                      # existing — untouched
├── scripts/                      # existing — untouched
├── roadmap/                      # existing — this file lives here
└── Makefile                      # existing — chunk 1 adds build-vectis target
```

**Scoping rules for every chunk in this plan:**

- Files outside `crates/vectis-cli/`, `templates/vectis/`, `roadmap/rfc-5-tasks.md`, the workspace `Cargo.toml`/`Cargo.lock`, and the `Makefile` are off-limits — **except** chunk 12, which intentionally edits `plugins/vectis/skills/{core,ios,android}-writer/`, and chunk 13, which adds `plugins/vectis/skills/template-updater/`.
- Do not edit `scripts/checks.ts`. The new files live under directories `checks.ts` does not enforce schema on.
- Do not introduce shared crates (`crates/common/` etc.) in this phase. Anything reusable later belongs inside `crates/vectis-cli/src/` for now and can be lifted when the workspace CLI lands.

## Initial Version Pins (vetted today against crates.io)

These are the values chunk 4 should embed in `crates/vectis-cli/embedded/versions.toml`. Crux's transitive constraints have been verified by reading each crate's published `Cargo.toml`.

```toml
# Embedded defaults — overridden by ~/.config/vectis/versions.toml or
# project-local versions.toml. See RFC-5 § Version Management.

[crux]
crux_core     = "0.17.0"   # latest stable
crux_http     = "0.16.0"   # latest stable; depends on crux_core ^0.17 + facet =0.31
crux_kv       = "0.11.0"   # latest stable; same constraints
crux_time     = "0.15.0"   # latest stable; same constraints
crux_platform = "0.8.0"    # latest stable; same constraints

# HARD PINS — do not relax. Crux 0.17 specifies these exactly and will not
# build with anything else. The `update-versions` command must lift the entire
# Crux set in lock-step when these change.
facet           = "=0.31"   # required by crux_core 0.17 and every cap crate
facet_generate  = "=0.15"   # codegen feature; only 0.15 is compatible with facet 0.31
serde           = "1.0"
serde_json      = "1.0"

# uniffi pin is coupled to cargo-swift. cargo-swift 0.9.x ships uniffi_bindgen 0.29.1;
# 0.10.x ships 0.30.0; 0.11.x ships 0.31.0. The values below are the known-good
# pair currently used by the writer skills.
uniffi      = "=0.29.4"
cargo_swift = "0.9"        # version range that ships uniffi_bindgen 0.29.x

[android]
# Initial values copied from RFC-5; chunk 11 (`update-versions`) will pull
# fresher values from Google's Maven and prove coherence via `--verify`.
compose_bom = "2025.01.01"
koin        = "4.0.4"
ktor        = "3.1.1"
kotlin      = "2.1.10"
agp         = "8.8.2"
gradle      = "8.13"

[ios]
# No external SPM deps today — VectisDesign and the generated SharedTypes /
# Shared packages are all internal.

[tooling]
cargo_deny = "0.19.4"      # latest
cargo_vet  = "0.10.2"      # latest
xcodegen   = "2.42.0"      # per RFC; not currently queried automatically
```

Notes for future maintainers (not action items):

- `facet 0.46.0` is the latest on crates.io as of Apr 2026 — Crux is two minor versions behind. Bumping `facet` requires either a new Crux release (preferred) or vendored patches.
- `cargo-swift 0.11.0` and `uniffi 0.31.0` are also available; switching requires re-verifying the iOS pipeline end-to-end.
- Android pins above are unverified against current Google Maven. They are placeholders chosen to match the existing skill references; chunk 11 is responsible for replacing them with proven-coherent values.

---

## Chunk 1 — Workspace and CLI skeleton

**Goal:** Establish the Cargo workspace at the repo root and a `vectis-cli` crate whose subcommands all dispatch to handlers that return well-formed "not implemented" JSON.

**RFC sections to read:** Workspace Layout · Dependencies · Makefile Integration · CLI Surface (skim only — implementation lands in later chunks)

**In scope:**

- `Cargo.toml` (workspace manifest, members = `["crates/vectis-cli"]`)
- `Cargo.lock`
- `crates/vectis-cli/Cargo.toml`
- `crates/vectis-cli/src/{main.rs,error.rs}`
- Empty stub modules: `prerequisites.rs`, `versions.rs`, `init/mod.rs`, `add_shell/mod.rs`, `verify/mod.rs`, `update_versions/mod.rs`, `templates/mod.rs`
- `Makefile` — add `build-vectis` target only; do not refactor existing targets
- `.gitignore` — add `/target` and `/vectis` (the copied binary) entries

**Out of scope:** prerequisites logic, version resolution, real subcommand behaviour, templates, skill changes.

**Steps:**

1. Workspace `Cargo.toml` with `resolver = "3"` (Rust 2024 default) and `members = ["crates/vectis-cli"]`. No workspace-level `[dependencies]` yet — keep deps in the crate to avoid forcing them on future siblings.
2. `crates/vectis-cli/Cargo.toml`: `clap = { version = "4", features = ["derive"] }`, `serde`, `serde_json`, `thiserror = "2"`. Edition `2024`.
3. Clap `#[derive(Parser)]` with four subcommands. Each handler returns a fixed JSON shape `{ "error": "not_implemented", "command": "<name>" }` and exits non-zero.
4. `error.rs` with a `VectisError` enum (variants: `MissingPrerequisites`, `Io`, `InvalidProject`, `Verify`, `Internal`) and a `to_json` helper that produces the RFC's structured error shape. Used by every handler.
5. Makefile `build-vectis` target as in RFC § Makefile Integration.

**Verification:**

```bash
cargo build --release -p vectis-cli
./target/release/vectis --help                        # shows all 4 subcommands
./target/release/vectis init Counter                  # exits non-zero, valid JSON
./target/release/vectis verify                        # exits non-zero, valid JSON
./target/release/vectis add-shell ios                 # exits non-zero, valid JSON
./target/release/vectis update-versions --dry-run     # exits non-zero, valid JSON
make checks                                           # still passes
```

Capture the JSON outputs in the PR description.

**Notes:**

Completed on branch `vectis-cli` (see status row for the branch-naming rationale).

Verification (all four subcommands exit 1 with valid JSON; `make checks` and `cargo clippy --release -p vectis-cli --all-targets -- -D warnings` both pass):

```text
$ ./target/release/vectis --help
Vectis CLI -- scaffolds the deterministic 'Hello World' starting point for Crux apps (core + optional iOS/Android shells) and verifies that every assembly compiles. See RFC-5.

Usage: vectis <COMMAND>

Commands:
  init             Scaffold a new Crux project (core, plus optional shells)
  verify           Verify that the project's assemblies compile
  add-shell        Add a platform shell to an existing project
  update-versions  Resolve and pin coherent dependency versions
  help             Print this message or the help of the given subcommand(s)

$ ./target/release/vectis init Counter   ; echo "exit=$?"
{
  "command": "init",
  "error": "not_implemented"
}
exit=1

$ ./target/release/vectis verify   ; echo "exit=$?"
{
  "command": "verify",
  "error": "not_implemented"
}
exit=1

$ ./target/release/vectis add-shell ios   ; echo "exit=$?"
{
  "command": "add-shell",
  "error": "not_implemented"
}
exit=1

$ ./target/release/vectis update-versions --dry-run   ; echo "exit=$?"
{
  "command": "update-versions",
  "error": "not_implemented"
}
exit=1
```

Implementation deviations from the chunk text, all minor:

- Stub modules are folder modules (`init/mod.rs`, `add_shell/mod.rs`, …) rather than flat `*.rs` files. The chunk text lists folder paths and the RFC § Workspace Layout shows flat files — I followed the chunk's structure since later chunks (5-11) will grow these modules.
- Added a `CommandOutcome::{Success, Stub { command }}` enum in `main.rs` so stub handlers can stay typed (`Result<CommandOutcome, VectisError>`) without inventing a `NotImplemented` variant on `VectisError` (the chunk pinned the error variant list). Future implementing chunks just return `Ok(CommandOutcome::Success(value))`.
- Added a `[[bin]]` entry naming the binary `vectis` so `cargo build --release -p vectis-cli` produces `target/release/vectis` (matching the Makefile's `cp` and the verification commands). Without it, the binary would be `vectis-cli`.
- Added `publish = false` and a license expression to `crates/vectis-cli/Cargo.toml` to silence packaging warnings — the crate is internal.
- `VectisError` carries `#[allow(dead_code)]` (with a comment) because chunk 1 only constructs `Io` (transitively, via `#[from]`) and exercises the rest only in unit tests. Chunks 2/5/9 will start using the remaining variants and should narrow or drop the attribute.

---

## Chunk 2 — Prerequisites detection

**Goal:** Every subcommand performs a scoped prerequisite check before any other work and returns the RFC's `missing_prerequisites` JSON shape on failure.

**RFC sections to read:** Prerequisite Detection (entire section, including the workstation requirements table)

**In scope:** `crates/vectis-cli/src/prerequisites.rs` and the call site in each subcommand handler. Subcommands still return "not implemented" after a successful check.

**Out of scope:** any actual scaffolding or verification work.

**Steps:**

1. Define a `Tool` struct (name, check command, version regex, min version, install hint, assembly tag) and an `AssemblyKind` enum (`Core`, `Ios`, `Android`).
2. Hard-code the table from RFC § Workstation Requirements. Use `std::process::Command` to run the `--version` checks. Parse versions with a small `semver` shim or string compare for the simple cases (avoid pulling in `semver` crate unless necessary).
3. Each subcommand declares which assemblies it cares about:
   - `init` → core + whatever `--shells` lists
   - `add-shell` → core + the named shell
   - `verify` → core + every assembly directory present in `--dir`
   - `update-versions` → core only by default; `--verify` upgrades to all
4. On any missing tool, emit the RFC's JSON shape and exit `2` (distinct from generic failure `1`).

**Verification:**

```bash
# With all tools installed:
./target/release/vectis init Counter --shells ios,android
# → exits with "not_implemented" (prereqs passed)

# Simulate missing xcodegen:
PATH_BACKUP=$PATH; PATH=$(echo $PATH | tr ':' '\n' | grep -v xcodegen | paste -sd:) \
  ./target/release/vectis init Counter --shells ios
# → exits 2 with missing_prerequisites JSON listing xcodegen
PATH=$PATH_BACKUP

cargo test -p vectis-cli prerequisites    # unit tests cover version comparisons
```

**Notes:**

---

## Chunk 3a — Templates: core extraction

**Goal:** Lift the deterministic file templates for the core assembly out of the existing skill references and into `templates/vectis/core/` with `__PLACEHOLDER__` markers in place of variable values.

**RFC sections to read:** File Manifests § Core Assembly · Workspace Layout (the `templates/vectis/core/` enumeration) · the placeholder table at the end of Workspace Layout

**Reference docs to read:**

- `plugins/vectis/skills/core-writer/references/crux-project-config.md`
- `plugins/vectis/skills/core-writer/references/crux-ffi-scaffolding.md`
- `plugins/vectis/skills/core-writer/references/crux-app-pattern.md`
- `plugins/vectis/skills/core-writer/references/crux-versions.md`

**In scope:** files under `templates/vectis/core/` only. No Rust code changes.

**Out of scope:** capability conditional logic (handled in chunk 6), template engine (chunk 5), iOS/Android templates (3b/3c).

**Steps:**

1. Create one file per row of the RFC's Core Assembly table.
2. Render each file as the version a render-only `Counter` app would have, then substitute placeholders per the RFC's placeholder table (`__APP_NAME__`, `__APP_STRUCT__`, `__CRUX_CORE_VERSION__`, `__FACET_VERSION__`, `__UNIFFI_VERSION__`, `__SERDE_VERSION__`).
3. Where capability-dependent sections appear (e.g. extra `[dependencies]` rows, extra `Effect` variants), wrap them in fenced markers like `<<<CAP:http\n...\nCAP:http>>>`. Chunk 6 will turn these into Rust-side conditional logic; chunk 3a is just placement.
4. Add a `templates/vectis/core/MANIFEST.md` listing every file and its required placeholders, kept in sync as a self-check during 3a.

**Verification:**

```bash
# Manual substitution test — proves a render-only core compiles from these templates.
mkdir /tmp/vectis-3a-check && cd /tmp/vectis-3a-check
cp -r $REPO/templates/vectis/core/* .
# Substitute placeholders by hand or with `sed -i`:
find . -type f -exec sed -i '' \
  -e 's/__APP_NAME__/Counter/g' \
  -e 's/__APP_STRUCT__/Counter/g' \
  -e 's/__CRUX_CORE_VERSION__/0.17.0/g' \
  -e 's/__FACET_VERSION__/=0.31/g' \
  -e 's/__UNIFFI_VERSION__/=0.29.4/g' \
  -e 's/__SERDE_VERSION__/1.0/g' {} \;
# Strip the cap markers (no caps for render-only):
find . -type f -exec sed -i '' '/<<<CAP:/,/CAP:.*>>>/d' {} \;
cargo check                                # MUST pass
```

Capture the output of `cargo check` in the PR.

**Notes:**

---

## Chunk 3b — Templates: iOS extraction

**Goal:** Same as 3a, for the iOS assembly.

**RFC sections to read:** File Manifests § iOS Assembly · placeholder table

**Reference docs to read:**

- `plugins/vectis/skills/ios-writer/references/ios-project-config.md`
- `plugins/vectis/skills/ios-writer/references/crux-ios-shell-pattern.md`
- `plugins/vectis/skills/ios-writer/references/swiftui-view-patterns.md`

**In scope:** `templates/vectis/ios/` only.

**Steps:** mirror 3a. Each file from the RFC's iOS table becomes a placeholder template under `templates/vectis/ios/`. Note that several files live at paths containing `{AppName}` — keep template filenames flat (e.g. `App.swift`) and store the on-disk target path in `MANIFEST.md` so the engine in chunk 7 knows where to write each one.

**Verification:**

```bash
# Substitute and run xcodegen + xcodebuild against a paired core (use a previously
# scaffolded core checkout, or hand-substitute the chunk-3a templates first).
# This is a partial check — full pipeline verification arrives in chunk 7.
xcodegen --spec /tmp/vectis-3b-check/iOS/project.yml
xcodebuild -project /tmp/vectis-3b-check/iOS/Counter.xcodeproj -scheme Counter \
  -destination 'platform=iOS Simulator,name=iPhone 15' build
```

**Notes:**

---

## Chunk 3c — Templates: Android extraction

**Goal:** Same as 3a/3b, for the Android assembly.

**RFC sections to read:** File Manifests § Android Assembly · placeholder table

**Reference docs to read:**

- `plugins/vectis/skills/android-writer/references/android-project-config.md`
- `plugins/vectis/skills/android-writer/references/crux-android-shell-pattern.md`
- `plugins/vectis/skills/android-writer/references/compose-view-patterns.md`

**In scope:** `templates/vectis/android/` only.

**Steps:** mirror 3a/3b. Pay attention to the `__ANDROID_PACKAGE_PATH__` placeholder — Kotlin source files live at paths derived from the package name and the engine will need to translate `com.vectis.counter` → `com/vectis/counter`. Capture this in `MANIFEST.md`. The Gradle wrapper files (`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.{jar,properties}`) are **not** templates — they are produced by `gradle wrapper` in chunk 8.

**Verification:**

```bash
# Substitute placeholders and run a full Android build against a paired core.
# Partial check — full pipeline verification arrives in chunk 8.
cd /tmp/vectis-3c-check/Android
gradle wrapper --gradle-version 8.13
./gradlew :app:assembleDebug
```

**Notes:**

---

## Chunk 4 — Version resolution and embedded defaults

**Goal:** Working `versions.rs` module that loads pins per the RFC's resolution order and exposes a typed `Versions` struct to the rest of the CLI.

**RFC sections to read:** Version Management § versions.toml · § Resolution order

**In scope:**

- `crates/vectis-cli/embedded/versions.toml` (use the "Initial Version Pins" block at the top of this file verbatim)
- `crates/vectis-cli/src/versions.rs`
- A `--version-file <path>` global flag on every subcommand
- Unit tests

**Out of scope:** querying registries (chunk 11), mutating any versions file (chunk 11).

**Steps:**

1. Define a `Versions` struct with nested `Crux`, `Android`, `Ios`, `Tooling` substructs, all `Deserialize`. Use `toml = "0.8"`.
2. Resolution order, top to bottom, first hit wins:
   1. `--version-file <path>` (explicit override)
   2. `<project>/versions.toml`
   3. `~/.config/vectis/versions.toml` (use the `dirs` crate or `std::env::var("HOME")`)
   4. embedded defaults via `include_str!("../embedded/versions.toml")`
3. Every layer is a complete `Versions` document — no partial/merge semantics in this chunk. Adding merge later is feasible; not doing it now keeps the surface tight.
4. Expose `Versions::resolve(project_dir, override_path) -> Result<Versions, VectisError>`.

**Verification:**

```bash
cargo test -p vectis-cli versions    # covers all four resolution paths

# Smoke test against the binary:
./target/release/vectis init Counter --version-file /nonexistent.toml
# → exits with InvalidProject error mentioning the missing file
```

**Notes:**

---

## Chunk 5 — `vectis init` core, render-only

**Goal:** `vectis init <Name>` (no `--caps`, no `--shells`) produces a complete, `cargo check`-passing core assembly using the chunk-3a templates and chunk-4 versions.

**RFC sections to read:** CLI Surface § `vectis init` · Output Format § `vectis init` output · § Template parameterization for `app.rs` (only the render-only baseline — capabilities arrive in chunk 6)

**In scope:**

- `crates/vectis-cli/src/templates/{mod.rs,core.rs}` — placeholder substitution and the cap-marker stripper (capabilities pass-through is empty for now)
- `crates/vectis-cli/src/init/{mod.rs,core.rs}` — orchestration
- Embed core templates via `include_str!`

**Out of scope:** `--caps` flag handling beyond accepting an empty list (chunk 6), iOS shell (chunk 7), Android shell (chunk 8).

**Steps:**

1. Template engine API: `render(template: &str, params: &Params, caps: &[Capability]) -> String`. For chunk 5, `caps` is always empty and the engine simply strips every `<<<CAP:...CAP:...>>>` block.
2. Embed each `templates/vectis/core/*` file via `include_str!` keyed by its target path. A simple `&[(&str, &str)]` slice is enough; no template registry trait yet.
3. `init::core::run(opts) -> InitResult`: validate app name (PascalCase regex), build `Params`, render every embedded template, write to `<project_dir>/<target_path>`, return the RFC's JSON.
4. Refuse to overwrite: if any target file exists, fail with a structured `InvalidProject` error before writing anything.

**Verification:**

```bash
rm -rf /tmp/vectis-5-check && \
  ./target/release/vectis init Counter --dir /tmp/vectis-5-check | jq .
cd /tmp/vectis-5-check && cargo check && cargo clippy --all-targets -- -D warnings
cargo run --bin codegen --features codegen,facet_typegen -- \
  --language swift --output-dir /tmp/vectis-5-codegen-swift
```

All four commands must exit zero. The `init` JSON output must list every file that was created.

**Notes:**

---

## Chunk 6 — `vectis init` core capability variants

**Goal:** `--caps` flag accepts any combination of `http,kv,time,platform,sse` and produces a `cargo check`-passing core.

**RFC sections to read:** § Template parameterization for `app.rs` · CLI Surface § `vectis init` (`--caps` flag)

**In scope:** `crates/vectis-cli/src/templates/{mod.rs,core.rs}` (extend the engine to evaluate cap markers), and the `app.rs`/`Cargo.toml`/`shared/Cargo.toml` core templates updated with the cap-marker fenced sections.

**Out of scope:** iOS/Android shell handling (later chunks).

**Steps:**

1. Define `Capability` enum with the five variants. Parse from comma-separated CLI input.
2. Replace the chunk-5 cap stripper with a real evaluator: `<<<CAP:http\n...\nCAP:http>>>` is included iff `http ∈ caps`.
3. Update the core templates to wrap capability-dependent regions: extra `[dependencies]` rows, extra `Event` variants, extra `Effect` variants, extra type aliases, extra `update()` arms.
4. Apply the RFC's per-capability `app.rs` rules (HTTP gets `FetchData`/`Fetched`, KV gets `LoadData`/`Loaded`, Time and Platform contribute only Effect variants and type aliases).

**Verification:**

```bash
for caps in "" "http" "kv" "http,kv" "http,kv,time,platform,sse"; do
  rm -rf /tmp/vectis-6-check
  ./target/release/vectis init Counter --dir /tmp/vectis-6-check \
    ${caps:+--caps "$caps"} || exit 1
  (cd /tmp/vectis-6-check && cargo check && cargo clippy --all-targets -- -D warnings) || exit 1
done
```

All combinations must pass.

**Notes:**

---

## Chunk 7 — `vectis init` iOS shell

**Goal:** `vectis init <Name> --shells ios` produces a core + iOS shell that builds in the iOS Simulator.

**RFC sections to read:** File Manifests § iOS Assembly · Verify Pipeline § iOS

**In scope:** `crates/vectis-cli/src/templates/ios.rs`, `crates/vectis-cli/src/init/ios.rs`, embed the chunk-3b templates.

**Out of scope:** `verify` command (chunk 9 calls these same build steps; doing it here would duplicate work).

**Steps:**

1. iOS file targets are nested under `iOS/<AppName>/...` — derive the target path per file from the chunk-3b `MANIFEST.md`.
2. Generate `iOS/project.yml` with `cargo-swift`-produced `Shared` and `SharedTypes` packages declared as local SPM dependencies. The packages don't exist on disk yet — they're produced by `make package` and `make typegen` during build/verify.
3. After writing files, run `make typegen && make package && make xcode` from `iOS/` (gated on prerequisites). Treat any non-zero exit as a hard failure with a structured error.

**Verification:**

```bash
rm -rf /tmp/vectis-7-check && \
  ./target/release/vectis init Counter --dir /tmp/vectis-7-check \
    --caps http --shells ios
cd /tmp/vectis-7-check/iOS
xcodebuild -project Counter.xcodeproj -scheme Counter \
  -destination 'platform=iOS Simulator,name=iPhone 15' build | xcbeautify
```

**Notes:**

---

## Chunk 8 — `vectis init` Android shell

**Goal:** `vectis init <Name> --shells android` produces a core + Android shell where `./gradlew :app:assembleDebug` succeeds.

**RFC sections to read:** File Manifests § Android Assembly · Verify Pipeline § Android · CLI Surface § `vectis init` (`--android-package` flag)

**In scope:** `crates/vectis-cli/src/templates/android.rs`, `crates/vectis-cli/src/init/android.rs`, embed the chunk-3c templates.

**Steps:**

1. Resolve `__ANDROID_PACKAGE__` from `--android-package` or default `com.vectis.<lower app name>`. Derive `__ANDROID_PACKAGE_PATH__` by replacing `.` with `/`. Apply when writing Kotlin files into `Android/app/src/main/java/<pkg-path>/...`.
2. After writing files: `cd Android && gradle wrapper --gradle-version <pinned>` (so the wrapper exists), then write `local.properties` with `sdk.dir=$ANDROID_HOME`. Both gated on prerequisite check.
3. Cap-conditional Kotlin files (`HttpClient.kt`, `SseClient.kt`, `KeyValueClient.kt`, `di/AppModule.kt`) — engine includes them only when their cap predicate matches. The cap-marker mechanism from chunk 6 needs a "whole file is conditional" variant; add it here.

**Verification:**

```bash
rm -rf /tmp/vectis-8-check && \
  ./target/release/vectis init Counter --dir /tmp/vectis-8-check \
    --caps http,kv --shells android
cd /tmp/vectis-8-check/Android
./gradlew :shared:cargoBuild :app:assembleDebug
```

**Notes:**

---

## Chunk 9 — `vectis verify`

**Goal:** `vectis verify` auto-detects assemblies, runs the RFC's per-assembly pipelines, returns the structured JSON output, and is callable from chunks 10 and 11.

**RFC sections to read:** Verify Pipeline (entire section) · Output Format § `vectis verify` output

**In scope:** `crates/vectis-cli/src/verify/`. May extract a small `assembly_detection.rs` shared with `init`/`add-shell` if duplication is obvious — keep it inside vectis-cli.

**Steps:**

1. Detect assemblies: core if `shared/src/app.rs` exists, ios if `iOS/` exists, android if `Android/` exists.
2. Per-assembly pipelines exactly as the RFC enumerates. Each step is a `(name, command, args, cwd)` tuple. Stop at the first failure within an assembly; continue independently across assemblies.
3. Codegen verification (steps 5/6 of core) writes to `/tmp/vectis-verify-<pid>/{swift,kotlin}` and cleans up on exit.
4. JSON output exactly matches the RFC shape, including per-step `passed` booleans and inline `error` strings.
5. `passed: true` overall iff every assembly passed.

**Verification:**

```bash
# Happy path:
./target/release/vectis verify --dir /tmp/vectis-8-check | jq .
# → "passed": true

# Negative path: inject a syntax error and re-run.
echo 'syntax error' >> /tmp/vectis-8-check/shared/src/app.rs
./target/release/vectis verify --dir /tmp/vectis-8-check | jq .
# → core.passed=false, first failed step "cargo check" with error string
```

**Notes:**

---

## Chunk 10 — `vectis add-shell` (incl. `app.rs` parser)

**Goal:** `vectis add-shell {ios,android}` reads an existing `shared/src/app.rs`, infers the app name and capabilities, and scaffolds the requested shell using chunk 7/8's logic.

**RFC sections to read:** CLI Surface § `vectis add-shell` · Output Format § `vectis add-shell` output

**In scope:** `crates/vectis-cli/src/add_shell/`, including a `parser.rs` for the limited `app.rs` parser.

**Steps:**

1. Parser is **structural, not syntactic**. Use `syn` to parse `shared/src/app.rs` and walk the AST for:
   - `impl App for <AppName>` → app name
   - `type Http = crux_http::Http<...>;` → http capability
   - `type Kv = crux_kv::KeyValue<...>;` → kv capability
   - `type Time = crux_time::Time<...>;` → time capability
   - `type Platform = crux_platform::Platform<...>;` → platform capability
   - `type Sse = crux_http::sse::Sse<...>;` → sse capability
2. Anything else recognized by name pattern (`type Xxx = some_crux::Crate<...>;`) goes into `unrecognized_capabilities` as a warning. Do not fail.
3. Refuse if `iOS/` or `Android/` (matching the requested platform) already exists.
4. Reuse `init::ios::scaffold(...)` / `init::android::scaffold(...)` — extract them from chunk 7/8's `run` so both `init` and `add-shell` can call them.
5. Run `vectis verify` for the just-added assembly and include results in the output.

**Verification:**

```bash
# Build a core-only project, then add iOS, then add Android.
rm -rf /tmp/vectis-10-check
./target/release/vectis init Counter --dir /tmp/vectis-10-check --caps http,kv
./target/release/vectis add-shell ios --dir /tmp/vectis-10-check | jq .
# → detected_capabilities: ["http","kv"]; verify passes
./target/release/vectis add-shell android --dir /tmp/vectis-10-check | jq .
./target/release/vectis verify --dir /tmp/vectis-10-check | jq .passed
# → true
```

**Notes:**

---

## Chunk 11 — `vectis update-versions`

**Goal:** `vectis update-versions` queries registries, computes a coherent bump, and writes/diffs the target versions file.

**RFC sections to read:** Version Management § `vectis update-versions`

**In scope:**

- `crates/vectis-cli/src/update_versions/`
- `Cargo.toml` deps: add `ureq = "2"` (small sync HTTP client) and `roxmltree = "0.20"` for Maven Central XML parsing. Avoid `reqwest` and `tokio` to keep the binary small and fast.

**Steps:**

1. **Crux block**: GET `https://crates.io/api/v1/crates/crux_core` for latest stable. GET `https://crates.io/api/v1/crates/crux_core/<version>/dependencies` and extract the pinned `facet`, `facet_generate`, `serde`, `serde_json` requirements. Capability crate versions: GET each one's latest stable and reject if its dep on `crux_core` doesn't match the chosen version.
2. **uniffi/cargo-swift**: GET `https://crates.io/api/v1/crates/cargo-swift` for latest, then read its `uniffi_bindgen` requirement to derive the `uniffi` pin. Move them as a pair.
3. **Android**: query Google Maven (`https://maven.google.com/androidx/compose/compose-bom/maven-metadata.xml`, etc.) — Maven Central does not host these. Use `roxmltree` to read `<versioning><latest>`. For Koin and ktor, query `https://search.maven.org/solrsearch/select?...&core=gav&rows=10&wt=json` and filter to versions matching `^\d+\.\d+\.\d+$` (no RC/Beta).
4. **Tooling**: `cargo-deny`, `cargo-vet` from crates.io. `xcodegen` from `https://api.github.com/repos/yonaskolb/XcodeGen/releases/latest`.
5. `--dry-run` prints the diff (no writes). Without `--dry-run`, write atomically (write to `.tmp`, rename) to `--version-file` (default `~/.config/vectis/versions.toml`).
6. `--verify`: after computing the new pins, scaffold a temp project per cap combination and run `vectis verify` against each. Only commit on success.

**Verification:**

```bash
./target/release/vectis update-versions --dry-run | jq .
# → emits diff JSON with current vs proposed values

./target/release/vectis update-versions --dry-run --verify
# → scaffolds temp projects, runs verify for each cap combo
```

**Notes:**

---

## Chunk 12 — Writer skill rewrites

**Goal:** Update `core-writer`, `ios-writer`, `android-writer` to invoke the CLI in greenfield, deleting the Create Mode steps the CLI now owns. Update Mode behaviour is unchanged.

**RFC sections to read:** Skill Integration · § Greenfield Detection in Build Orchestration

**In scope:**

- `plugins/vectis/skills/core-writer/SKILL.md`
- `plugins/vectis/skills/ios-writer/SKILL.md`
- `plugins/vectis/skills/android-writer/SKILL.md`
- Their `references/` — delete files no longer referenced anywhere; trim files that are still partly relevant.

**Out of scope:** `template-updater` (chunk 13).

**Steps:**

1. For each skill, replace the Create Mode section with the RFC's new Mode Detection prose. Keep the Update Mode section verbatim.
2. Audit `references/` per skill: any file whose entire content is now embodied in templates (e.g. `crux-project-config.md` boilerplate, `ios-project-config.md` build flags) is deleted. Files that contain pattern guidance still relevant to Update Mode (e.g. `crux-app-pattern.md`, `swiftui-view-patterns.md`, `compose-view-patterns.md`) stay.
3. Update the skill frontmatter's `references:` array to match.
4. Update `plugins/vectis/SKILL.md` (or equivalent index) if it lists per-skill references.

**Verification:**

```bash
make checks                          # markdown link checks must pass
# Smoke test on a downstream project:
cd /tmp && rm -rf smoke && mkdir smoke && cd smoke
# Then in Cursor with Specify wired up:
#   /spec:init && /spec:define "Counter app" && /spec:build
# Confirm the writer skills invoke `vectis init` / `vectis verify` and skip
# the deleted Create Mode steps.
```

**Notes:**

---

## Chunk 13 — `template-updater` skill

**Goal:** New agent skill that closes the loop on version bumps by detecting template breakage, fixing it, and proving the fix.

**RFC sections to read:** Template Maintenance § `template-updater` Agent Skill

**In scope:**

- `plugins/vectis/skills/template-updater/SKILL.md`
- `plugins/vectis/skills/template-updater/references/` (as needed)
- `.cursor-plugin/plugin.json` (or whatever marketplace manifest enumerates skills) — add the new skill.

**Steps:**

1. Mirror the structure of an existing vectis skill (e.g. `core-reviewer/SKILL.md`) for frontmatter and section layout.
2. Encode the five-step flow from the RFC: detect (scratch scaffold + `verify`), diagnose (read errors + changelog), update (edit templates + cap-conditional logic), validate (cap matrix), report.
3. Reference `vectis update-versions --verify` as the primary tooling.
4. Include a worked example (e.g. "when `crux_core` bumps from 0.17 → 0.18 and renames `Effect::Render` to `Effect::View`").

**Verification:**

```bash
make checks                          # frontmatter schema, link targets, marketplace consistency
# Manually invoke the skill in Cursor against a synthetic broken bump
# (edit templates to introduce a known compile error, run the skill).
```

**Notes:**

---

## Decision Log

Append entries here when a chunk uncovers a question that needed a judgement call.

- **Chunk 1 — Branch naming.** The pre-existing `vectis-cli` branch (which carries the RFC and this tasks file) blocks creation of any `vectis-cli/<sub>` branch because git treats refs hierarchically. Chose to commit chunk 1 directly on `vectis-cli` rather than rename the parent. Future chunks should adopt a flat naming scheme like `vectis-cli-chunk-N-slug`. Updating the convention in "How to Use This File" can wait until chunk 2 starts and confirms the new pattern.
- **Chunk 1 — Stub return shape.** The chunk text specified the `VectisError` variant list (no `NotImplemented`) and the stub JSON shape (`{"error": "not_implemented", "command": "<name>"}`). To reconcile these without duplicating `println!`/`exit` in each handler, introduced `CommandOutcome::{Success, Stub}` in `main.rs`. Stubs return `Ok(CommandOutcome::Stub { command })`; the dispatcher renders the JSON and exits 1. Real handlers in later chunks switch to `CommandOutcome::Success(value)` with no other dispatch changes.
