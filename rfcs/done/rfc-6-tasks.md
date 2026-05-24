# RFC-6 Implementation Tasks

> **Status: Superseded.** The standalone `vectis` binary these tasks built has been folded into the `specify` CLI as the `specify vectis ...` subcommand tree, living in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (`crates/vectis/` library + `templates/vectis/`). The chunk list and per-chunk verification commands below reference the original `crates/vectis-cli/` paths and `./target/release/vectis` binary that no longer exist in this repo; they are preserved as historical record of how RFC-6 was originally executed.

> Source RFC: [rfc-6-vectis-bootstrap.md](rfc-6-vectis-bootstrap.md) Owner: vectis CLI binary and the writer skills it replaces Scope: **vectis only**. A future top-level workspace CLI (RFC-1) will be folded in as a sibling member; nothing in this plan should require touching anything outside the vectis surface.

## How to Use This File

Each chunk is a self-contained agent session. Per session:

1. Read this file's status table.
2. Pick the next `[ ]` chunk whose dependencies are all `[x]`.
3. Read **only** the RFC sections and reference docs listed in that chunk.
4. Work directly on the `vectis-cli` branch. Chunks are constructed in dependency order and land as a single linear commit each -- do **not** create per-chunk branches (multiple branches diverge faster than they can be reconciled). Pull `vectis-cli` first, then rebase if needed.
5. Run the chunk's verification commands. They are gates, not suggestions.
6. Update the status row, append any deviations to the chunk's "Notes", make exactly one commit on `vectis-cli` with subject `RFC-6 chunk <N>: <short title>`, then push.
7. Do **not** expand scope. If you discover work belonging to a later chunk, add it to that chunk's "Notes" column and stop.

## Status

All chunks land as a single linear commit on the `vectis-cli` branch (one commit per chunk).

| # | Chunk | Status | Notes |
|---|-------|--------|-------|
| 1 | Workspace + CLI skeleton | [x] | Dispatcher uses a `CommandOutcome::{Success,Stub}` enum so handlers can stay stubbed without polluting `VectisError`. `VectisError` carries `#[allow(dead_code)]` until chunks 2/5/9 start constructing the unused variants. |
| 2 | Prerequisites detection | [x] | Args structs in `main.rs` are now `pub(crate)` and each handler signature is `run(args: &XxxArgs)` -- chunks 5/7/8/9/10/11 no longer need to plumb args through. `#[allow(dead_code)]` on `VectisError` was narrowed to per-variant on `Verify` and `Internal`; `MissingPrerequisites` and `InvalidProject` are now actively constructed. Each Args struct carries per-field `#[allow(dead_code)]` for fields not yet read; later chunks should drop the annotation as they read each field. |
| 3a | Templates: core extraction | [x] | Source filenames are flat (`workspace-cargo.toml`, `shared-cargo.toml`, `gitignore`, `supply-chain-config.toml`, ...); the source→target path mapping lives in `templates/vectis/core/MANIFEST.md`. Chunk 5's engine reads/embeds that mapping rather than reflecting the on-disk layout. Capability-version placeholders (`__CRUX_HTTP_VERSION__`, `__CRUX_KV_VERSION__`, `__CRUX_TIME_VERSION__`, `__CRUX_PLATFORM_VERSION__`) only appear inside CAP markers and are NOT in the RFC's placeholder table -- chunks 5/6 must include them when their cap is selected. `__ANDROID_PACKAGE__` is referenced from `codegen.rs` (Kotlin namespace), so chunk 5 must substitute it for core-only and iOS-only projects too -- default `com.vectis.<lower app name>`. `thiserror = "2"` added as an optional dep gated behind `uniffi` / `wasm_bindgen` features (the existing reference docs omit it but `ffi.rs` requires it). PartialEq/Eq dropped from the `Event` derive (capability payloads like `crux_http::Response<Vec<u8>>` don't impl `Eq`). |
| 3b | Templates: iOS extraction | [x] | Source filenames are flat (`project.yml`, `Makefile`, `App.swift`, `Core.swift`, `ContentView.swift`, `LoadingScreen.swift`, `HomeScreen.swift`); the source→target path mapping lives in `templates/vectis/ios/MANIFEST.md`. Target paths embed `__APP_NAME__` in **directory and file-name positions** (e.g. `iOS/__APP_NAME__/__APP_NAME__App.swift` -> `iOS/Counter/CounterApp.swift`) -- chunk 7's engine must apply placeholder substitution to the path it constructs from MANIFEST, not just to file contents. New placeholder `__APP_NAME_LOWER__` (the lowercase form of `--app-name`, no other transformation) is used for bundle ids in `project.yml`; chunk 7 derives it from `args.app_name` rather than asking the user. Templates intentionally **omit** `VectisDesign` and `Inject` from the SPM dependency list (rationale and reinstate paths in MANIFEST § Design system / Inject); writer skills layer them in during Update Mode. iOS Core.swift carries `<<<CAP:http/kv/time/platform>>>` markers but no `<<<CAP:sse>>>` -- matches today's `app.rs` (no `Effect::Sse` variant). Cap-conditional Swift content in `Core.swift` includes both the `case` arm in `processEffect(_:)` and any helper functions it relies on, all inside the same marker (Swift compiler enforces exhaustive switches; see MANIFEST notes for chunk 7). |
| 3c | Templates: Android extraction | [x] | Source filenames are flat (`Makefile`, `root-build.gradle.kts`, `settings.gradle.kts`, `gradle.properties`, `libs.versions.toml`, `app-build.gradle.kts`, `shared-build.gradle.kts`, `AndroidManifest.xml`, `themes.xml`, `network-security-config.xml`, `Application.kt`, `MainActivity.kt`, `Core.kt`, `LoadingScreen.kt`, `HomeScreen.kt`, `Color.kt`, `Theme.kt`, `Type.kt`, `gitignore`); source→target mapping lives in `templates/vectis/android/MANIFEST.md`. Target paths embed `__APP_NAME__` (e.g. `__APP_NAME__Application.kt`) **and** `__ANDROID_PACKAGE_PATH__` (e.g. `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/core/Core.kt`); chunk 8's engine must apply path-segment substitution. `__ANDROID_PACKAGE_PATH__` is **derived** at file-write time by replacing `.` with `/` in `__ANDROID_PACKAGE__` -- it never appears in file contents and is not a placeholder the engine carries in memory. New version placeholders introduced (NOT in the RFC's placeholder table): `__AGP_VERSION__`, `__KOTLIN_VERSION__`, `__COMPOSE_BOM_VERSION__`, `__KTOR_VERSION__`, `__KOIN_VERSION__` (all in `libs.versions.toml`, only meaningful inside `<<<CAP:http`); chunk 4 must expose these via `Versions::android`. `__ANDROID_NDK_VERSION__` (in `shared-build.gradle.kts`) is also new and not in chunk-4's substruct -- chunk 8 should detect from `$ANDROID_HOME/ndk/<version>/` rather than pin a specific NDK that may not be installed. The "Initial Version Pins" block in this file is **stale for Android**: it lists `agp = "8.8.2"`, `kotlin = "2.1.10"`, `compose_bom = "2025.01.01"`; verification used the reference doc values `8.13.2` / `2.3.0` / `2026.01.01` plus `ktor = "3.4.0"` / `koin = "4.1.1"` (the older pins do not produce a buildable APK on Xcode 16 / Java 21 toolchains). Chunk 4 should bump the Android defaults; chunk 11 must understand the new placeholder names. `network-security-config.xml` is **whole-file conditional** on `http` or `sse` (no CAP markers inside) -- chunk 8 needs a "skip this whole file if cap missing" predicate. `koin-bom`/`ktor-*` deps and `viewModelScope`/coroutine plumbing in `Core.kt` are gated only on `<<<CAP:http`; non-HTTP cap arms in `Core.kt` (`kv`, `time`, `platform`) are TODO stubs that bind `effect.value` to a suppressed-warning local and do nothing else (the render-only baseline never emits them; writer skills replace in Update Mode). The `sse` cap intentionally has no entry in `Core.kt` -- mirrors chunk 3b/3a (`app.rs` has no `Effect::Sse(...)` variant); when chunk 6 adds it, this manifest, `libs.versions.toml`, `AndroidManifest.xml`, and `Core.kt` need matching `<<<CAP:sse` blocks. `gradle.properties` deliberately **omits** `org.gradle.java.home` (per-machine path); chunk 8 should auto-detect Java 21 via `/usr/libexec/java_home -v 21` and write it at scaffold time so the project remains hermetic. Templates intentionally omit `:vectis-design`, the Koin `AppModule.kt`, and per-cap helper classes (`HttpClient.kt`, `SseClient.kt`, `KeyValueClient.kt`); writer skills layer them in during Update Mode (Pattern 1 vs Pattern 2 from `crux-android-shell-pattern.md`). The Gradle wrapper files and `local.properties` are NOT templates -- chunk 8 produces them via `gradle wrapper --gradle-version <pin>` and by writing `sdk.dir=$ANDROID_HOME` after the Gradle config files exist. Verification: cargo check on staged paired core ✅; `gradle wrapper --gradle-version 8.13` ✅ (must use Gradle 8.x to bootstrap; `rust-android-gradle = 0.9.6` calls `setFileMode(Integer)` which Gradle 9.x removed -- chunk 8 should bundle or download an 8.x gradle to bootstrap if the developer's system gradle is 9.x); `make build` (codegen) ✅; `./gradlew :app:assembleDebug` ✅. |
| 4 | Version resolution + embedded defaults | [x] | `embedded/versions.toml` lives at `crates/vectis-cli/embedded/`; `include_str!("../embedded/versions.toml")` from `src/versions.rs` resolves correctly. `Versions` exposes all five Crux crate versions plus the hard-pin set (`facet`, `facet_generate`, `serde`, `serde_json`, `uniffi`, `cargo_swift`); `Versions::android` carries `compose_bom`/`koin`/`ktor`/`kotlin`/`agp`/`gradle` plus `ndk: Option<String>` (omitted from embedded defaults so chunk 8 detects from `$ANDROID_HOME/ndk/<version>/`). Android pins bumped per chunk-3c verification: `agp = "8.13.2"`, `kotlin = "2.3.0"`, `compose_bom = "2026.01.01"`, `ktor = "3.4.0"`, `koin = "4.1.1"`. **`--version-file` semantics differ across subcommands**: on `init`/`verify`/`add-shell` it is a resolution override (the file MUST exist) and is added by this chunk; on `update-versions` it is the *write target* (already wired in chunk 1, kept as-is). Only `init::run` actually calls `Versions::resolve` today (the smoke-test gate); chunks 5/6/7/8 plumb it through their templates, chunks 9/10/11 plumb it through their handlers -- the per-handler `version_file` Args fields carry `#[allow(dead_code)] // consumed by chunk N` until then. Resolver factored as `Versions::resolve(project, override)` (public, reads `$HOME` from env) delegating to `resolve_with(project, override, home)` (test-only injection) so unit tests don't have to mutate process-global env vars. **Stale pins discovered during chunk-7 verification** (NOT patched here -- chunk-7 stayed in scope; chunk 11 owns the bump): `cargo_swift = "0.9"` ships uniffi_bindgen 0.29.1 and produces a `Package.swift` whose binary target is named `RustFramework` while the inner module is `sharedFFI`; Xcode 26 / Swift 6 fails to resolve `import sharedFFI` against that mismatch (every `RustBuffer`/`ForeignBytes`/`RustCallStatus` reference becomes "cannot find type ... in scope"). cargo-swift 0.11.0 fixes this by renaming the binary target to match the module. Chunk 11 must bump the pair to `cargo_swift = "0.11"` and `uniffi = "=0.31.0"` (uniffi_bindgen 0.31.0 is what 0.11 ships) and re-run the chunk-7 verification matrix end-to-end before committing. The runtime `uniffi` macros (`uniffi::Object`, `uniffi::export`, `uniffi::constructor`, `uniffi::Error`, `uniffi(flat_error)`) used in `templates/vectis/core/ffi.rs` are stable across 0.29 → 0.31 -- no template surgery expected, but verify with `cargo check --features uniffi` after the bump. |
| 5 | `vectis init` core, render-only | [x] | Handler returns `Ok(CommandOutcome::Success(value))`; the `Stub` return path is removed for `init`. `&InitArgs` is consumed (`app_name`, `dir`, `version_file` actively used; `caps` and `android_package` read for guardrails / default Android package -- their `#[allow(dead_code)]` annotations dropped in `main::InitArgs`). `init::run` calls `Versions::resolve` once and threads the result into `init::core::scaffold`. The chunk-3a/3b/3c MANIFESTs are encoded as the embedded slice in `templates/core.rs::TEMPLATES` (one row per file, target path declared inline) rather than as a parsed copy of `MANIFEST.md` -- the manifest stays the source of truth; `templates::core::tests::registry_matches_rfc_core_file_count` enforces parity (currently 13 files). `__ANDROID_PACKAGE__` is always substituted (default `com.vectis.<lower app name>`) even with no `--shells android`, so `codegen.rs` compiles for core-only / iOS-only projects. The render-only cap stripper drops marker lines + content; chunk 6's evaluator (already wired with a `Capability` enum + `cap_selected` predicate) drops only the marker lines when the cap is selected. **Chunk-5 verification surfaced a chunk-3a/4 template gap**: the workspace `[workspace.lints.clippy]` block uses `cargo = "warn"` plus per-lint group entries at the same priority -- under `cargo clippy --all-targets -- -D warnings` (a chunk-5 gate, not run during chunk 3a's verification) this fires `clippy::cargo_common_metadata` (the scaffolded `shared` crate has no description/license/repository -- it isn't published to crates.io), `clippy::multiple_crate_versions` (Crux's transitive deps bring `syn 1` + `syn 2` and similar duplicates the user can't dedupe), and `clippy::lint_groups_priority` (groups must carry an explicit lower priority to allow per-lint overrides). Patched in `templates/vectis/core/workspace-cargo.toml`: lint groups now use `{ level = "warn", priority = -1 }` and `cargo_common_metadata` / `multiple_crate_versions` are `allow`. **Chunk 6 needs to know**: the placeholder substitution order in `templates::mod.rs::substitute_placeholders` substitutes `__APP_NAME_LOWER__` *before* `__APP_NAME__` (`__APP_NAME_LOWER__` is a strict superstring of `__APP_NAME__`); when chunk 6/7 add new placeholders they must be slotted into the same chain in superstring-first order. **Render-only `--caps`/`--shells` handling**: chunk 5 accepts an empty `--caps` / `--shells` and rejects any non-empty value with a structured `InvalidProject` error pointing at chunks 6/7/8; chunk 6 should replace the `--caps` guard with the comma-split parser, and chunks 7/8 should replace the `--shells` guard with their scaffold dispatch. **MANIFEST self-check command**: must use `command ls -1 templates/vectis/core` (the user's interactive `ls` alias adds `-lpa` rows that pollute the diff; the chunk-3a recipe needs `command ls -1` to be hermetic) and must filter `awk` output through `grep -v -E '^(http|kv|time)$'` (the Cap-marker reference table also matches the `^\| \`[a-z]` row pattern). |
| 6 | `vectis init` core capability variants | [x] | `init::run` now parses `--caps` via a sibling helper to `parse_shells` (`init::mod::parse_caps`, comma-split + trim + dedupe-in-input-order); not lifted into a single generic helper because the typed enums and error messages diverge. `Capability::from_tag` was added; `#[allow(dead_code)]` annotations on `Capability` and `marker_tag` are dropped. JSON output's `capabilities` field now reflects the parsed set in input order. **`sse` decision: NOT added to `app.rs` Effect/Event enum.** It remains a deps-only cap (`async-sse`, `futures` in `shared-cargo.toml`) -- adding `Effect::Sse(...)` would cascade into chunks 7/8 (Core.swift / Core.kt sse cap blocks, exhaustive switches), widening scope for no observable user benefit. The existing chunk-7/8/3a/3b/3c notes that say "when chunk 6 adds Sse, do X" still apply conditionally; chunk 12's writer skills can introduce it later if a real sse app needs it. **Two template lint-hygiene patches were required to make `cargo clippy --all-targets -- -D warnings` pass for every cap combo** -- both went into `templates/vectis/core/app.rs`: (1) `#[allow(dead_code)]` on each capability type alias (`Http`, `KeyValue`, `Time`, `Platform`) because the render-only baseline never references them (writer skills wire them into update arms during Update Mode); (2) `#[allow(clippy::match_same_arms)]` on the `update()` fn because every per-cap arm shares a `render()` body, which fires `clippy::match_same_arms` when more than one cap is selected. Both are render-only-baseline scaffolding and should be dropped by chunk-12's writer skills as soon as they replace the placeholder bodies (recorded in chunk 12's notes). Within a single cap's match arms, the `FetchData|Fetched(_)` and `LoadData|Loaded(_)` patterns are already merged with `\|` to silence the same-cap variant of `match_same_arms`. **Verification matrix all green**: cargo check + `cargo clippy --all-targets -- -D warnings` for caps in `""`, `"http"`, `"kv"`, `"http,kv"`, `"http,kv,time,platform,sse"`. **Stale-cwd hazard for the next agent**: the recommended verification loop in this chunk does `cd /tmp/vectis-6-check && cargo ...`; if the loop body deletes the dir before the next iteration, the shell sits in a deleted cwd and subsequent commands fail with `Unable to proceed. Could not locate working directory.` Use `cargo check --manifest-path /tmp/vectis-6-check/Cargo.toml` (or re-`cd` to a stable dir each iteration) to keep the loop hermetic. |
| 7 | `vectis init` iOS shell | [x] | `init::run` now dispatches to `init::ios::scaffold` for `--shells ios` and merges its files + per-make-step results into `assemblies.ios` (sibling of `assemblies.core`). `--shells android` still returns `InvalidProject` pointing at chunk 8. The placeholder map (`Params`) is built once via a private `init::mod::build_params` and threaded into both core and iOS scaffolds -- chunk 8 should reuse this helper rather than duplicating it inside `init::android`. New public engine helper `templates::substitute_path` applies `__APP_NAME_LOWER__`-then-`__APP_NAME__` substitution to target path strings (the order matters; chunk 8 must extend this for `__ANDROID_PACKAGE_PATH__`). The iOS scaffold refuses to overwrite a pre-existing `iOS/` directory before any byte is written (atomic refusal mirrors `init::core::scaffold`). Build pipeline (`make typegen && make package && make xcode`) runs from `iOS/` after files are written; each step's pass/fail (and on failure, combined stdout/stderr) is captured into `BuildStep` and serialized into `assemblies.ios.build_steps`. The `run_build` flag on `ios::scaffold` is `true` from `init::run` and `false` from unit tests so the test suite stays hermetic. **Tooling drift resolution: bumped cargo-swift to 0.11 locally (chunk-4 cascade)**: cargo-swift 0.9.0 produces a `Package.swift` whose binary target is named `RustFramework` while the inner module is `sharedFFI`, and Xcode 26 / Swift 6 cannot resolve `import sharedFFI` against that mismatch (every type from `RustBuffer` down comes back "cannot find type ... in scope"). cargo-swift 0.11.0 emits the binary target as `sharedFFI` (matching the modulemap), which fixes the import without any patch on the generated swift. The chunk-4 embedded `cargo_swift = "0.9"` pin and `uniffi = "=0.29.4"` runtime pair is now stale -- chunks 4 and 11 must bump to `cargo_swift = "0.11"` + `uniffi = "=0.31"` (uniffi_bindgen 0.31.0 is what cargo-swift 0.11 ships). Did NOT change embedded/versions.toml in this chunk (out of scope per the chunk-7 in-scope list); chunk 11 owns the version-bump cascade. **`make package` failure mode for the next agent**: if a downstream user has only cargo-swift 0.9 installed, the `make package` step succeeds (exit 0) but the Swift package is unbuildable -- chunk 9's verify pipeline will fail at `xcodebuild` rather than at `make package`. Either (a) chunk 4's pin bump fixes this for new installs, or (b) chunk 2's prereq check needs a min-version floor on `cargo swift --version` (>= 0.10). **Verification deviations from the chunk's command list:**  (1) `iPhone 15` is no longer auto-installed under Xcode 26.4 (only iPhone 16 and 16 Pro variants ship); created a custom simulator (`xcrun simctl create "iPhone 15 RFC5" "iPhone 16" com.apple.CoreSimulator.SimRuntime.iOS-26-0`) and used `-destination 'id=<UDID>'` instead of the chunk's `name=iPhone 15`. Chunk 9 should use `-destination 'generic/platform=iOS Simulator'` (no name) to be hermetic across Xcode versions. (2) The chunk-7 verification ran while `/tmp/vectis-7-check` was the shell's cwd -- macOS rotated `/tmp` mid-session and the shell hung in a deleted cwd; absolute paths to `xcodebuild -project` avoid this. **`Info.plist` is materialized by xcodegen** from the `info.properties` block in `project.yml`; chunk 7 deliberately does not pre-write a static `Info.plist` (xcodegen would overwrite it anyway). |
| 8 | `vectis init` Android shell | [x] | `init::run` now dispatches `--shells android` to `init::android::scaffold` and merges its files + per-step build results into `assemblies.android` (sibling of `assemblies.core` and `assemblies.ios`). Reused the existing `init::mod::build_params` helper from chunk 7 -- chunks 9/10 should keep using it rather than re-deriving placeholder maps per scaffold. New public engine helper `templates::substitute_path_with(target, params, android_package_path: Option<&str>)` extends chunk 7's `substitute_path` with an optional `__ANDROID_PACKAGE_PATH__` substitution applied **before** `__APP_NAME_LOWER__` / `__APP_NAME__`; iOS callers pass `None`, Android callers compute the package path (`.` -> `/`) and pass `Some(...)`. The chunk-3c MANIFEST is encoded as a `templates::android::TEMPLATES` slice with one row per file plus an `IncludeWhen::{Always, AnyOf(&[Capability])}` predicate -- `network_security_config.xml` is the only `AnyOf(&[Http, Sse])` row today (whole-file conditional). The Android version placeholders (`agp`, `kotlin`, `compose_bom`, `ktor`, `koin`) were added to `Params` and substituted unconditionally for every scaffold (no-op in core/iOS templates because none reference them); `__ANDROID_NDK_VERSION__` is **not** substituted at scaffold time -- `init::android::run_pipeline` resolves it from `versions.android.ndk` or by scanning `$ANDROID_HOME/ndk/<version>/` (highest lexical version wins) and patches `Android/shared/build.gradle.kts` just before the build runs. `init::android::scaffold` writes `local.properties` from `$ANDROID_HOME` (returns `Internal` if unset, which prereq check should prevent) and appends `org.gradle.java.home=<path>` to `gradle.properties` when `/usr/libexec/java_home -v 21` succeeds (macOS-only; no-op elsewhere). The pipeline (`gradle wrapper --gradle-version <pin>` -> `make build` -> `./gradlew :app:assembleDebug`) reuses chunk-7's `BuildStep` shape (`{name, passed, error?}`) so `assemblies.android.build_steps` matches `assemblies.ios`'s shape; chunk 9 should extract this into `verify::android::run_pipeline` rather than duplicating the loop (mirrors the chunk-9 note for iOS). **Gradle 9.x incompatibility worked around inside the CLI (no PATH munging required).** The developer's system `gradle` was 9.4.1 (Homebrew default on macOS today). `rust-android-gradle = 0.9.6` calls `setFileMode(Integer)` which Gradle 9.x removed, so invoking `gradle wrapper --gradle-version 8.13` from inside the scaffolded `Android/` directory fails *during project evaluation* -- Gradle loads `settings.gradle.kts` and the root/app `build.gradle.kts` plugin classpath before the wrapper task runs, and the `rust-android-gradle` plugin then explodes. Fix: `init::android::bootstrap_wrapper` creates a fresh empty scratch dir under `std::env::temp_dir()`, drops a blank `settings.gradle.kts` in it, runs `gradle wrapper --gradle-version <pin>` *there* (no plugins to evaluate), then copies `gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar`, and `gradle/wrapper/gradle-wrapper.properties` into the project. Sets the unix exec bit on `gradlew` post-copy and best-effort cleans the scratch. After the wrapper is in place every subsequent step (`make build`, `./gradlew :app:assembleDebug`) uses Gradle 8.13 from the wrapper distribution and the rust-android-gradle plugin loads cleanly. This means chunk 2 does **not** need to gate on `gradle < 9.0.0` -- the system Gradle just has to be runnable enough to emit a wrapper, which 9.x is. **Verification command outcomes**: (1) `vectis init Counter --dir /tmp/vectis-8-check --caps http,kv --shells android` -- all three `build_steps` passed (`gradle wrapper`, `make build`, `gradlew assembleDebug`). (2) `cd /tmp/vectis-8-check/Android && ./gradlew :shared:cargoBuild :app:assembleDebug` -- `BUILD SUCCESSFUL in 1s` (warm cache; the init step already exercised `:app:assembleDebug` end-to-end). No deviations from the chunk's command list. **Test isolation hazard**: the unit tests that mutate `ANDROID_HOME` race under `cargo test`'s default parallel runner; serialised via a static `OnceLock<Mutex<()>>` (`android_home_lock`) inside `init::android::tests`. **All chunk-8 verification commands passed end-to-end**: `cargo build --release -p vectis-cli`, `cargo clippy --release -p vectis-cli --all-targets -- -D warnings`, `cargo test -p vectis-cli` (84 tests, 0 failed), `vectis init Counter --caps http,kv --shells android` (every `build_steps[*].passed` is `true`), and `./gradlew :shared:cargoBuild :app:assembleDebug` (BUILD SUCCESSFUL). **For chunk 9**: extract `init::android::run_pipeline` into `verify::android::run_pipeline` and have both `init::android::scaffold` and `verify::run` call it -- the loop will drift if duplicated (mirrors the chunk-7 -> chunk-9 extraction note for iOS). The `BuildStep` shape is already RFC-compliant. **For chunk 10**: `init::android::scaffold` already takes a `run_pipeline: bool` flag (`true` from `init::run`, `false` from tests) so `add-shell android` can call it directly with `run_pipeline=true` -- no further extraction needed. **The Sse cap still has no Android footprint** -- when chunk 12 (or a writer skill) introduces `Effect::Sse(...)` to `app.rs`, this MANIFEST, `libs.versions.toml`, `AndroidManifest.xml`, and `Core.kt` all need matching `<<<CAP:sse` blocks (`network_security_config.xml`'s predicate already includes `Sse` so no change there). |
| 9 | `vectis verify` | [x] | **Chunk 9 complete.** `verify::run` now auto-detects assemblies (`shared/src/app.rs` for core; `iOS/` and `Android/` dirs for shells), runs `prerequisites::check` scoped to that set, validates `args.version_file` via `Versions::resolve` (failures surface as structured `InvalidProject`), creates a scratch codegen sink (see `VerifyCache` below), and runs each assembly's pipeline to completion even if a prior assembly failed (per RFC: "continue independently across assemblies"). Final JSON is `{project_dir, passed, assemblies: {core?, ios?, android?}}` with each assembly's value `{passed, steps: [{name, passed, error?}]}` -- exact RFC shape, validated in both happy (`passed: true` across core+android) and sad (`core.passed=false`, `steps[0] = {name: "cargo check", passed: false, error: "..."}`) runs against `/tmp/vectis-8-check`. **Pipeline extraction landed** (forward impact for chunks 10 and any future skill that runs builds): new `verify::pipeline` module owns `BuildStep {name, passed, error?}` + `run_step(name, &mut Command) -> Result<BuildStep>`; `verify::{core,ios,android}::run_pipeline` are the per-assembly executors. `init::ios::scaffold` now calls `verify::ios::run_pipeline(&ios_root, /*include_xcodebuild=*/ false)` instead of its private `run_make_pipeline`; `init::android::scaffold` calls `verify::android::run_pipeline(&android_root)` for the make + gradlew steps (its `bootstrap_wrapper` + NDK injection stay in `init::android` because they are scaffold-time concerns the RFC explicitly excludes from verify). Chunk 10's `add-shell` should just call `verify::run` for the newly-added assembly -- do NOT re-implement the pipeline loop. **Core pipeline** runs the six RFC steps in order, stopping at the first failure: `cargo check` -> `cargo clippy --all-targets -- -D warnings` -> `cargo deny check` -> `cargo vet` -> `cargo run -p shared --bin codegen --features codegen,facet_typegen -- --language {swift,kotlin} --output-dir <cache>/{swift,kotlin}`. **iOS pipeline** runs `make typegen` -> `make package` -> `make xcode` -> `xcodebuild -project <found>.xcodeproj -scheme <AppName> -destination 'generic/platform=iOS Simulator' build` from the `iOS/` dir (init skips the final `xcodebuild` by passing `include_xcodebuild=false` so `init` stays fast; verify always runs it). The `-destination 'generic/platform=iOS Simulator'` choice (vs the chunk's suggested `name=iPhone 15`) is hermetic across Xcode versions -- resolves the chunk-7 finding that `iPhone 15` no longer auto-installs under Xcode 26.4. **Android pipeline** runs `make build` -> `./gradlew :app:assembleDebug` from the `Android/` dir -- deliberately drops the `gradle wrapper` step (init already ran it; re-bootstrapping on verify would clobber any user-edited `gradle-wrapper.properties`). **Scratch-dir decision for codegen output (`VerifyCache`)**: prefers `$HOME/.cache/vectis/verify-<pid>/` when `$HOME` is set; falls back to `std::env::temp_dir()/vectis-verify-<pid>/` only when `$HOME` is unset. The `Drop` impl best-effort-removes the cache. This works around the macOS `/tmp` rotation the chunk notes warned about: a long-running verify that happens to span a rotation window won't have its open `shared/target/.../deps/` handles yanked. **Template patches required for `cargo deny check` to pass on a freshly scaffolded project (forward impact for chunk 12's writer skills and chunk 13's template-updater):** (1) `templates/vectis/core/deny.toml` now sets `[licenses] private = { ignore = true }` and `[advisories] ignore = ["RUSTSEC-2024-0370", "RUSTSEC-2025-0141"]` (the unmaintained `proc-macro-error` + `bincode` are transitive deps of Crux 0.17 with no safe upgrade today -- chunk 13's `template-updater` skill prunes these when Crux moves away). (2) `templates/vectis/core/shared-cargo.toml` now carries `publish = false` so the `[licenses] private = { ignore = true }` rule applies (without `publish = false`, cargo-deny still flags the crate as `unlicensed`). If a writer skill or user later decides to publish the shared crate, they must remove `publish = false` AND add a `license = "..."` field together. **`cargo vet` first-run bootstrap** (new behaviour inside `verify::core`, not a separate step): the template ships `supply-chain/imports.lock` as header-only and `supply-chain/config.toml` with no `[[exemptions.*]]` block, which makes `cargo vet` fail immediately on a greenfield project. `verify::core::run_pipeline` now detects this pristine state (`imports.lock` has no non-comment non-empty lines AND `config.toml` contains no `[[exemptions.`) and runs `cargo vet regenerate exemptions` once before the actual `cargo vet` step; subsequent verifies skip the regen so real supply-chain drift surfaces as a failure. Both the regen and the real `cargo vet` appear in the JSON as a single `"name": "cargo vet"` step (per the RFC's step-name spec), so if the regen fails the error is attributed to cargo vet -- acceptable because the user's recovery step is the same (run `cargo vet` manually). **JSON shape matches RFC exactly** (spot-checked against § Output Format § `vectis verify` output: top-level `{project_dir, passed, assemblies}`; each assembly `{passed, steps: [{name, passed, error?}]}`; `error` is `skip_serializing_if = "Option::is_none"` so passing steps never carry an `error: null`). **All chunk-9 verification commands passed end-to-end**: `cargo build --release -p vectis-cli`, `cargo clippy --release -p vectis-cli --all-targets -- -D warnings`, `cargo test -p vectis-cli` (87 tests, 0 failed), `vectis init Counter --caps http,kv --shells android` (fresh scaffold with the patched `deny.toml` + `publish = false`), `vectis verify --dir /tmp/vectis-8-check` (happy: `passed: true` with all core/android steps green), syntax-error injected into `shared/src/app.rs` + `vectis verify` (sad: `passed: false`, `assemblies.core.passed: false`, `steps[0] = {name: "cargo check", passed: false, error: "... expected one of \`!\` or \`::\`, found \`error\` ..."}`) -- matches the RFC's negative-path expectation exactly. **Original chunk notes preserved below for history:** Same `Stub` -> `Success` transition; `&VerifyArgs` is already plumbed through. `args.version_file` is plumbed but unread (chunk 4); drop its `#[allow(dead_code)]` when you call `Versions::resolve(&dir, args.version_file.as_deref())`. The on-disk assembly detection (`dir.join("iOS").is_dir()` / `Android`) is already implemented in `verify::run` for prereq scoping -- reuse it (or extract it alongside the per-assembly pipeline). Will start constructing `VectisError::Verify`; drop the per-variant `#[allow(dead_code)]` on `VectisError::Verify` then. **Chunk 7 already started constructing `VectisError::Verify`** for `make {typegen,package,xcode}` failures, so the per-variant `#[allow(dead_code)]` annotation has effectively been retired -- when chunk 9 lands, just remove the line in `error.rs`. **iOS verify pipeline already lives in `init::ios::run_make_pipeline`** (per-step name/passed/error captured into `BuildStep`); chunk 9 should extract it (rename to `verify::ios::run_pipeline`) and have both `init::ios::scaffold` and `verify::run` call it -- duplicating the loop will drift. **Android verify pipeline already lives in `init::android::run_pipeline`** (chunk 8 -- per-step name/passed/error captured into the same `BuildStep` shape); same extraction story (`verify::android::run_pipeline`). The `BuildStep` JSON shape (`{name, passed, error?}`) already matches the RFC's per-step output; reuse it for core too. **Android pipeline ordering**: chunk 8 calls `gradle wrapper -> make build -> ./gradlew :app:assembleDebug`; the chunk-9 verify equivalent should drop the `gradle wrapper` step (the wrapper already exists from init) and run `make build -> ./gradlew :app:assembleDebug` -- re-bootstrapping the wrapper on verify would clobber any patched `gradle-wrapper.properties` the user has hand-edited. **`-destination` for `xcodebuild`**: use `-destination 'generic/platform=iOS Simulator'` rather than the chunk's `name=iPhone 15`. Under Xcode 26.4 the `iPhone 15` simulator is no longer auto-installed (only iPhone 16 / 16 Pro variants ship), and any specific name will rot; `generic/platform=iOS Simulator` is hermetic across Xcode versions. **`make package` failure mode for old cargo-swift installs**: with cargo-swift 0.9 the step exits 0 but the resulting Swift package fails to compile under Xcode 16+ (`cannot find type 'RustBuffer' in scope`). chunk-9 verify's `xcodebuild` step will surface this, but the failure is misleading because `make package` reported success. Either chunk 4 + 11 bump the cargo-swift pin (preferred, see chunk-4 notes) or chunk 2's prereq check needs a min-version floor on `cargo swift --version` (>= 0.10). **Gradle 9.x failure mode for Android verify**: the `:app:assembleDebug` step (and any `gradle wrapper` re-bootstrap) requires Gradle 8.x because `rust-android-gradle = 0.9.6` calls `setFileMode(Integer)` which Gradle 9.x removed. Once the wrapper exists, `./gradlew` uses the project-pinned 8.13 distribution so this is moot for the build itself, but chunk 2's prereq check should add a max-version floor (`gradle < 9.0.0`) so `gradle wrapper` (in init / `add-shell` / `update-versions --verify`) doesn't fail with a confusing Groovy stack trace on a freshly-installed Homebrew machine. **Stable scratch dir**: `/tmp/vectis-verify-<pid>/{swift,kotlin}` is fine on Linux but macOS rotates `/tmp` aggressively (chunk 6/7/8 all hit this). Either resolve `/tmp` once via `std::env::temp_dir()` and keep an open file handle so the directory survives, or use `$HOME/.cache/vectis/verify-<pid>/` as the codegen sink. |
| 10 | `vectis add-shell` (incl. `app.rs` parser) | [x] | **Chunk 10 complete.** `add_shell::run` parses `shared/src/app.rs` with `syn` (new `syn = "2"` dep; see `add_shell::parser`), recovers `{app_name, capabilities, unrecognized_capabilities}`, resolves `Versions` via `Versions::resolve(&dir, args.version_file.as_deref())` (bad `--version-file` fails before any byte is written), and then calls `init::ios::scaffold(.., run_build=true)` or `init::android::scaffold(.., run_build=true)` for the requested shell. The `#[allow(dead_code)]` annotations on `AddShellArgs::{dir, android_package, version_file}` are dropped. JSON output is `{app_name, project_dir, platform, source: "app.rs", detected_capabilities, unrecognized_capabilities, assembly: {status: "created", files, build_steps}, passed}`. **Verification matrix all green** end-to-end: `cargo build --release -p vectis-cli` ✅, `cargo clippy --release -p vectis-cli --all-targets -- -D warnings` ✅, `cargo test -p vectis-cli` (103 tests, 0 failed) ✅; `vectis init Counter --caps http,kv` ✅ (core-only scaffold), `vectis add-shell ios --dir <tmp>` ✅ (all three iOS steps pass: `make typegen`, `make package`, `make xcode`), `vectis add-shell android --dir <tmp>` ✅ (all three Android steps pass: `gradle wrapper`, `make build`, `gradlew assembleDebug`), `vectis verify --dir <tmp> | jq .passed` → `true` ✅ (core/ios/android all pass end-to-end). **Parser design**: key off RHS crate root, not alias name. `type KeyValue = crux_kv::KeyValue<..>` resolves to `Capability::Kv` even though the scaffold uses the alias name `KeyValue` (not `Kv`). Nested `crux_http::sse::Sse` is detected before the bare `crux_http` arm so it maps to `Capability::Sse`, not `Http`. Unknown `crux_*` crates land in `unrecognized_capabilities` as verbatim crate-path strings (non-fatal warnings, per the RFC). Non-`crux_*` type aliases (e.g. `type Foo = Vec<u8>;`) are ignored silently. **Design deviation from the chunk's advisory (intentional)**: chunk-10 Notes said to call `verify::run(&VerifyArgs {..})` for the just-added assembly. We call `init::{ios,android}::scaffold(run_build=true)` instead because (a) `init::android::scaffold` bootstraps the Gradle wrapper *and* runs the full build pipeline in one pass -- calling `verify::run` afterwards would require running prereq + Gradle bootstrap a second time, or bypassing scaffold's build loop (which would then silently double-run everything), and (b) the `BuildStep` shape is identical to what `verify::run` would emit because chunk 8/9 already extracted `verify::android::run_pipeline` (and `init::android::run_pipeline` delegates to it). So the JSON in `assembly.build_steps` matches what `vectis verify` would record for the same assembly, minus the iOS `xcodebuild` step (init's scaffold passes `include_xcodebuild=false`; a consumer wanting that step runs `vectis verify` afterwards). This decision is documented in the `add_shell/mod.rs` module comment. `init::mod::build_params` was promoted to `pub(crate)` (not relocated to `init::params::build` -- the helper is trivial and the one additional call site doesn't justify a new module). `CommandOutcome` now `#[derive(Debug)]` so `add_shell` tests can pattern-match outcomes in error paths. **`verify::core::run_pipeline` gained a 7th step** (forward impact for chunks 11 + 13): `cargo build shared cdylib` runs between `codegen swift` and `codegen kotlin`. Rationale: the Kotlin codegen step runs `uniffi::generate_bindings(..)` which requires `libshared.{dylib,so}` under `target/{debug,release}/`, but `cargo run --bin codegen` only builds the binary + the `shared` *rlib* -- not the cdylib. Chunk 9's happy-path test worked only because it scaffolded `--shells android` first, which pre-built the cdylib via `rust-android-gradle`; chunk-10 flows (init core-only, then add-shell ios/android) exposed the gap. Adding `cargo build -p shared --features uniffi` makes verify hermetic regardless of which shells exist. Chunk 11's `update-versions --verify` will exercise this through each cap matrix combo. Chunk 13 should know about the new step name when writing template-updater validation. **For chunk 11**: no changes to the overall approach, but `update-versions --verify` incremental matrix builds can skip full `vectis init` per cap and just swap in new pins via `init + add-shell` sequences. **For chunk 12**: writer skills should continue to use `vectis add-shell` (not re-implement scaffolding) in greenfield add-shell flows; the JSON shape is stable. **For chunk 13**: `add_shell::parser` is an AST parser, not a regex -- when Crux bumps rename capability crates (e.g. `crux_http` -> `crux_network_http`), the template-updater skill must update both the template (the `type Http = ...;` line) and the parser's `classify_cap_path` switch in `add_shell/parser.rs`. Today's parser recognises exactly five crates (`crux_http`, `crux_http::sse`, `crux_kv`, `crux_time`, `crux_platform`). **`android_package` for add-shell**: derived from `--android-package` if supplied, else `com.vectis.<app_name_lower>` via `init::core::default_android_package`. The parsed `app.rs` does **not** carry the original package choice (no marker comment today); a project that used a custom package at init time must re-pass `--android-package` to `add-shell android`. If this becomes painful, a minimal `// @vectis-package = "..."` marker in `app.rs` is a trivial extension (the parser already walks items). **Hazards observed during verification**: (1) `rm -rf /tmp/vectis-10-check` frequently returns `Directory not empty` on macOS due to /tmp rotation racing with concurrent target/ writes; used `mktemp -d /tmp/vectis-10-check-XXXXXX` instead to sidestep. (2) The user's interactive `ls` alias emits `-lpa`-style rows; verification commands must use `/bin/ls -d <glob>` when capturing paths into shell variables. |
| 11 | `vectis update-versions` | [x] | **Chunk 11 complete.** `update_versions::run` now queries registries, computes a coherent proposal, prints the diff (or writes atomically), and -- when `--verify` is passed -- scaffolds a 4-combo cap matrix (`""`, `"http"`, `"http,kv"`, `"http,kv,time,platform,sse"`) and runs `vectis verify` on each before committing the new pins. New deps: `ureq = "2"` (sync HTTP) + `roxmltree = "0.20"` (Google Maven XML). The `#[allow(dead_code)]` annotations on `UpdateVersionsArgs::{version_file, dry_run}` and `Versions::embedded` are dropped; `VectisError` now has every variant actively constructed (chunk 4's `Internal` covered malformed embedded defaults, chunk 11 adds registry-failure construction). **JSON shape**: `{version_file, dry_run, verify, passed, written, changes[], unchanged[], errors?[], verification?}`; each `changes` entry is `{key, current, proposed}`, each `unchanged` is `{key, value}`. `verification` (when `--verify`) is `{passed, combos: [{caps, passed, verify?, error?}]}` where `verify` is the full `vectis verify` JSON for that combo. **Write semantics**: `--dry-run` never writes; without `--dry-run`, writes atomically (write to `<path>.toml.tmp`, rename over target); with `--verify` and no `--dry-run`, writes only on matrix pass. **The chunk-7 prescribed embedded bump (`cargo_swift = "0.11"` + `uniffi = "=0.31.0"`) was attempted and REVERTED** -- the bump breaks `verify::core::run_pipeline`'s `codegen kotlin` step on a fresh scaffold. Root cause: `crux_core = "0.17.0"`'s `cli` feature transitively pins `uniffi_bindgen = "=0.29.4"`, and `templates/vectis/core/codegen.rs` calls `crux_core::cli::bindgen(...)` to generate Kotlin bindings; mixing a 0.31 runtime (in `shared`) with a 0.29 bindgen fails with `unresolved module path shared::ffi`. There is no Crux release today that tracks uniffi_bindgen 0.31. **This pair must stay at `0.29.4` / `0.9` in the embedded defaults until either (a) a newer crux_core release bumps bindgen to 0.31 or (b) chunk 13's `template-updater` skill rewrites `codegen.rs` to call `uniffi_bindgen = "=0.31.0"` directly, decoupled from `crux_core::cli::bindgen`.** The chunk-7 iOS concern (`import sharedFFI` broken under Xcode 26+/Swift 6) is handled by the *local* cargo-swift install -- every chunk-7/8/9/10 verification passed on a machine with `cargo-swift 0.11` via Homebrew even with the embedded runtime pin at `0.29.4`, because cargo-swift 0.11's bindgen can read 0.29.4's metadata (uniffi metadata format is stable across 0.29 → 0.31). Treat cargo-swift like xcodegen / xcbeautify: per-developer prereq, not vendored runtime pin. Rationale is inlined as a multi-line comment in `embedded/versions.toml` so the next maintainer doesn't rediscover it. **Network-resilience**: registry probes are per-field isolated -- a failed query keeps the current pin and appends a diagnostic to `errors[]` rather than aborting the whole command. Observed during verification: `maven central: ktor-bom` and `maven central: kotlin-stdlib` occasionally time out on the search.maven.org solr endpoint; the field falls back to current value and the command completes normally. **Coherence gate working as intended**: the `--verify --dry-run` command run against today's registries correctly reports `passed: false` for every combo because the proposed pins include `uniffi = "=0.31.0"` (latest stable) which trips the same `codegen kotlin` failure described above. This is exactly the intended `--verify` behaviour -- surface drift before writing -- and it validates that chunk 13 has concrete, reproducible cases to work from. **Gradle pin NOT auto-bumped**: `android.gradle` stays pinned to the value in the current pins file. Gradle major-version bumps (e.g. 8.x → 9.x) break `rust-android-gradle 0.9.6` (see chunk 8 notes) and gradle pairs with AGP on a discrete compatibility table. A human owns this pin. **Observed drift in today's registries (all deferred to chunk 13):** (1) `crux.facet_generate` proposed `^0.15` (from `crux_core 0.17.0`'s declared dep req) vs current `=0.15` -- cargo treats them as equivalent but the string differs; chunk-13 should normalise to `=x.y.z`. (2) `android.agp` latest is `9.1.1` which needs Gradle 9.x; blocked by `rust-android-gradle 0.9.6` -- chunk 13 either pins AGP <9 via a capped coordinate or drives a rust-android-gradle update. (3) Combo-3 (full caps) trips `cargo deny check` on `RUSTSEC-2024-0384 (instant unmaintained)` + `RUSTSEC-2026-0097 (rand unsound)` -- both pulled in by `async-sse` when `sse` is in the cap set. Chunk 13 should extend `templates/vectis/core/deny.toml`'s `[advisories] ignore` list to include these two RUSTSEC IDs (same pattern as the two chunk-9 added), or push Crux upstream to drop the `async-sse` transitive chain. **Matrix implementation notes for future maintainers**: `update_versions::matrix::CAP_MATRIX` is a `&[&str]` constant -- each entry is a `--caps` string. The loop is core-only on purpose (shells vendor the same pins; multiplying by ios+android for little coverage and a lot of build time). Each combo writes a sibling `pins.toml` into its scratch dir, invokes `init::run` programmatically with `version_file: Some(...)` pointing at that file, then `verify::run` with the same override. Scratch dirs live under `$HOME/.cache/vectis/update-versions-<pid>/combo-<N>/` (same rationale as `verify::VerifyCache` -- macOS `/tmp` rotation). Each combo's dir is removed after use so the matrix peak disk usage is ~225MB (one combo's target/), not ~900MB. **Forward impact for chunk 12 (writer skills)**: nothing new. **Forward impact for chunk 13 (template-updater)**: the three drift items above (facet_generate string normalisation, AGP >=9 gate, new RUSTSEC advisories in full-caps combo) are concrete tasks; the uniffi/cargo-swift bump decoupling is the biggest structural job, summarised in chunk 13's Notes as well. **Verification commands outcome**: `./target/release/vectis update-versions --dry-run --version-file /tmp/pins.toml` -- exits 0, emits `{changes: [10 items], unchanged: [10 items], errors: [0-1 transient]}` ✅. `./target/release/vectis update-versions --dry-run --verify --version-file /tmp/pins.toml` -- exits 0, emits `{verification: {passed: false, combos: [4 items, each with full verify JSON]}}` ✅ (matrix correctly reports incoherence in latest pins; this is the gate doing its job). `cargo build --release -p vectis-cli` ✅, `cargo clippy --release -p vectis-cli --all-targets -- -D warnings` ✅, `cargo test -p vectis-cli` (118 tests, 0 failed) ✅. Chunk-9 regression smoke: fresh `vectis init Counter --caps http` + `vectis verify` with the embedded defaults -- `passed: true` across all seven core steps including `codegen kotlin` ✅. |
| 12 | Writer skill rewrites | [x] | **Chunk 12 complete.** Rewrote `core-writer`, `ios-writer`, `android-writer` SKILL.md. Each skill's Create Mode now collapses to: (1) read Specify artifacts / `app.rs` for the derived App name + capability set, (2) invoke the CLI (`vectis init {AppName} --dir {project-dir} --caps {caps}` + `vectis verify` for core; `vectis add-shell ios --dir {app-dir}` for iOS; `vectis add-shell android --dir {app-dir} [--android-package ...]` for Android), (3) switch to Update Mode. Update Mode sections + Preservation Rules + Artifact-to-Code mapping + Update Change Patterns are unchanged. Deleted 5 reference files whose content is now embodied in the CLI's embedded templates: `core-writer/references/{crux-versions,crux-project-config,crux-ffi-scaffolding}.md`, `ios-writer/references/ios-project-config.md`, `android-writer/references/android-project-config.md`. Updated each SKILL.md's Reference Documentation table to point at `crates/vectis-cli/embedded/` and `crates/vectis-cli/src/{scaffold/core,init/ios,init/android}.rs` for boilerplate ownership, and at `vectis update-versions` for version-pin refreshes. Skill frontmatter has no `references:` array (verified via grep); `plugins/vectis/README.md` only links `review-checks.md`, so no index update was required. **Deviations**: (a) the chunk's verification includes an *interactive* Cursor smoke test ("in Cursor with Specify wired up: `/spec:init && /spec:define "Counter app" && /spec:build`") that cannot be executed by a non-interactive agent -- the automated `make checks` gate passed (`All checks passed.`); the interactive smoke test is deferred to the first human user who drives the writer skills end-to-end. (b) `ios-writer/SKILL.md` and `android-writer/SKILL.md` originally carried "Internal Sub-Agent Decomposition" sections that decomposed Create Mode into Scaffold / Bridge / UI phases and enumerated per-phase reference dependencies (including the now-deleted `{ios,android}-project-config.md`). With the CLI owning the entire scaffold phase, the phase split no longer reflects the shape of the work; those sections have been replaced with a single "Verification ownership" subsection under Mode Detection that preserves the orchestrator's `skip_verification` contract (writer stops at code-gen, dedicated verify sub-agent runs the build) without referencing the removed phase/reference decomposition. **Forward-impact carried into the rewritten skills** (all still true under the new text): (1) core-writer still has instructions to drop the render-only-baseline `#[allow(dead_code)]` on capability `type` aliases and `#[allow(clippy::match_same_arms)]` on `update()` once per-cap arms get real bodies (now lives in the core-writer "Switch to Update Mode" section). (2) `publish = false` stays in the scaffolded `shared/Cargo.toml` unless the user explicitly opts to publish (must pair with `license = "..."` in the same edit). (3) Writer skills must not hand-seed `supply-chain/config.toml` exemptions -- first `vectis verify` bootstraps them. **Forward impact for chunk 13 (`template-updater`)**: the writer skills now route all version-pin refreshes through `vectis update-versions`, so any Crux bump that renames a capability crate (e.g. `crux_http` -> `crux_network_http`) requires `template-updater` to edit both the templates and, indirectly, the `add_shell/parser.rs` crate-path classifier -- the writer-skill SKILL.md files don't enumerate crate names, so `template-updater`'s scope is unchanged from chunk 11's note but is now the *only* place those crate names are hard-coded in writer-visible docs. When chunk 6's SSE decision is ever revisited (add `Effect::Sse(...)` to `app.rs`), the writer skills will need `Sse` added to each file's Capability Detection / CAP-block enumeration -- but that edit is mechanical and ride-alongs with the template change. When the core-writer skill rewrites the render-only `app.rs`, it should drop the chunk-6 lint-hygiene scaffolding once the placeholder bodies are replaced: (a) the `#[allow(dead_code)]` on each capability `type` alias (the alias becomes a real call site in `update()`), and (b) the `#[allow(clippy::match_same_arms)]` on `update()` (each arm gets a distinct body). Leaving them in place is harmless under `-D warnings` but masks future regressions. **Chunk 9 added `publish = false` to the scaffolded `shared/Cargo.toml`** so `cargo deny check` exempts the unlicensed shared crate via `[licenses] private = { ignore = true }`. If the writer skill ever adds a `license = "..."` field to `shared/Cargo.toml` (e.g. when the user decides to publish), it must remove `publish = false` at the same time -- leaving both fields fights itself (crate is nominally licensed but still treated as private). Safer policy: leave `publish = false` in place unless the user explicitly asks to publish. **Chunk 9 also added a first-run supply-chain bootstrap inside `verify::core`** -- writer skills should NOT try to hand-seed `supply-chain/config.toml` exemptions themselves; the first `vectis verify` after scaffolding does it automatically via `cargo vet regenerate exemptions`. |
| 13 | `template-updater` skill | [x] | **Chunk 13 complete.** Added `plugins/vectis/skills/template-updater/SKILL.md` plus `plugins/vectis/skills/template-updater/references/known-drift.md`. The SKILL.md mirrors the core-reviewer / design-system-writer layout (frontmatter, `## Arguments`, `## Process` broken into D1--D5 matching the RFC's Detect/Diagnose/Update/Validate/Report flow, worked example, anti-patterns, reference table). Frontmatter carries `name: template-updater` + a 10+-character description with "Use when ..." as the schema requires; no `references:` array is used (same convention as the other Vectis skills). Editable surface is scoped to `templates/vectis/**`, `crates/vectis-cli/src/templates/**`, `crates/vectis-cli/src/add_shell/parser.rs`, `crates/vectis-cli/embedded/versions.toml`, and `templates/vectis/core/deny.toml`; `init/verify/update_versions/prerequisites/main.rs` are explicitly off-limits (those are CLI orchestration, not templates -- if a fix needs them, the skill stops and flags it for a new chunk). Worked example is the chunk-13-specified `crux_core 0.17 -> 0.18` / `Effect::Render` -> `Effect::View` rename, walked step-by-step with the three template-file touch points (`app.rs`, `Core.swift`, `Core.kt`) and an explicit "no parser edit needed (crate root unchanged)" justification so the next maintainer has a concrete micro-scoped pattern to reuse. The `known-drift.md` reference doc enumerates the four backlog items chunks 9/11 left for this skill -- item 1 is the `uniffi` / `crux_core::cli::bindgen` decoupling (structural rewrite of `templates/vectis/core/codegen.rs`), item 2 is the AGP >= 9.0 / `rust-android-gradle 0.9.6` cap (option: teach `update_versions::query::google_maven_latest_stable` to filter on max-version), item 3 is `RUSTSEC-2024-0384` + `RUSTSEC-2026-0097` ignore entries under the `sse`-cap transitive chain, item 4 is the cosmetic `facet_generate ^0.15` -> `=0.15.0` req-string normalisation. Each item has a reproducible trigger (`vectis update-versions --dry-run --verify` against today's registries) and a fix path with option (a) / option (b) where applicable. Updated `docs/plugins.md` Vectis section (enforced by `scripts/checks.ts` §11 `checkPluginsDocInventory` -- every skill on disk must be listed) and `plugins/vectis/README.md` skills table (not enforced but keeps the plugin's own landing page consistent). **Marketplace manifest update**: `plugins/vectis/.cursor-plugin/plugin.json` does NOT enumerate skills (skills are auto-discovered from `skills/*/SKILL.md`), so no edit there. The chunk text said "`.cursor-plugin/plugin.json` (or whatever marketplace manifest enumerates skills) -- add the new skill"; the conditional ("or whatever") applies -- the actual enforcement lives in `docs/plugins.md` + `scripts/checks.ts`, which I updated. **Verification**: `make checks` green end-to-end (`All checks passed.`) on both the pre-edit baseline and after the skill + references + docs edits landed. This covers frontmatter schema validation, skill-reference link resolution, marketplace consistency, `docs/plugins.md` inventory parity, markdown link targets, and skill variable consistency -- the six SKILL-level checks from `scripts/checks.ts`. **Deviations from the chunk's verification list**: the chunk's second bullet says "Manually invoke the skill in Cursor against a synthetic broken bump (edit templates to introduce a known compile error, run the skill)." This is an interactive-only end-to-end smoke test; the automated agent session cannot drive a human-initiated Cursor skill invocation against a CLI that is also the system-under-test. Deferred to the first human who drives a real upstream bump -- the four items in `known-drift.md` are already concrete, reproducible trigger points for that run. Same class of deviation that chunk 12's Notes recorded for the Cursor writer-skill smoke test. **Forward impact**: none -- chunk 13 is the terminal chunk in the dependency graph. If chunk 6's Sse decision is ever revisited (`Effect::Sse(...)` added to `app.rs`), the template-updater skill does not need to change: the skill's process operates on template files + the `add_shell/parser.rs` crate-path classifier, and the latter already tries `crux_http::sse::Sse` before the bare `crux_http` arm so Sse is correctly classified today. **Preserved forward-notes from chunks 9/11 below** (retained for history -- they are the input that chunk 13 was wired to resolve): **Chunk 9 patched `templates/vectis/core/deny.toml`** with a hardcoded `[advisories] ignore = ["RUSTSEC-2024-0370", "RUSTSEC-2025-0141"]` list suppressing the unmaintained `proc-macro-error` (transitive via `crux_macros` -> `crux_core`) and `bincode 1.x` (transitive via `crux_core`). When `template-updater` detects a Crux bump that drops either dependency, it must remove the corresponding `RUSTSEC-*` entry from `deny.toml` -- leaving a stale `ignore` entry makes `cargo deny check` softer than intended. Conversely, when new upstream advisories land on transitive Crux deps that can't be fixed immediately, the skill should add a fresh `RUSTSEC-*` entry with a rationale comment (same pattern as the two listed today). Keep `[licenses] private = { ignore = true }` and the `[advisories] ignore` list's preamble comment untouched -- they are project-policy, not version-bumpable. **Chunk 11 surfaced three concrete drift items for this skill** (all reproduced deterministically via `vectis update-versions --dry-run --verify` against today's registries): **(1) `uniffi` / `cargo-swift` decoupling from crux_core::cli::bindgen.** `crux_core 0.17.0`'s `cli` feature pins `uniffi_bindgen = "=0.29.4"`; the embedded runtime pin can't bump to `=0.31.0` while the template's `codegen.rs` calls `crux_core::cli::bindgen(...)` because mixing a 0.31 runtime with a 0.29 bindgen fails at `codegen kotlin` with `unresolved module path shared::ffi`. The fix is to either (a) rewrite `templates/vectis/core/codegen.rs` to call `uniffi_bindgen 0.31` directly (adding `uniffi_bindgen = "=0.31.0"` to the shared crate's `[dependencies]`, dropping the `crux_core::cli::{BindgenArgsBuilder, bindgen}` imports, and reconstructing the argument builder against uniffi_bindgen's public API), or (b) wait for a crux_core release that tracks 0.31 bindgen and bump the pair in lockstep. Once decoupled, chunk 11's matrix combos should all go green. **(2) AGP 9.x vs rust-android-gradle 0.9.6.** `update-versions` auto-proposes `android.agp = "9.1.1"` (latest on Google Maven), but `rust-android-gradle 0.9.6` calls `setFileMode(Integer)` which Gradle 9.x removed (see chunk-8 notes); using AGP 9.x transitively requires Gradle 9.x, which bricks `gradle wrapper` bootstrap. Fix: either pin AGP below 9.0 (teach `update_versions::query::google_maven_latest_stable` to filter on a max-version constraint) or drive a rust-android-gradle update upstream so Gradle 9.x works. **(3) RUSTSEC-2024-0384 (`instant` unmaintained) + RUSTSEC-2026-0097 (`rand 0.7.3` unsound).** Both fire in the `http,kv,time,platform,sse` matrix combo because `async-sse 5.1.0` transitively pulls `http-types 2.12.0` -> `rand 0.7.3` + `futures-lite 1.13.0` -> `instant 0.1.13`. Neither `async-sse` nor `http-types` has a maintained successor that drops these. Add both RUSTSEC IDs to `templates/vectis/core/deny.toml`'s `[advisories] ignore` list with a rationale comment ("transitive via async-sse when sse cap is enabled; no maintained upstream replacement"). **(4) facet_generate req-string normalisation (cosmetic).** `crux_core 0.17.0`'s published Cargo.toml declares `facet_generate = "^0.15"` but the RFC's "hard pin" policy wants `=0.x.y`. chunk 11's registry query faithfully copies the req string; chunk 13 should either normalise (`^0.15` -> `=0.15.0`) or document that facet_generate is semver-minor-pinned and drop the `=` prefix convention. |

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

- Files outside `crates/vectis-cli/`, `templates/vectis/`, `rfcs/rfc-6-tasks.md`, the workspace `Cargo.toml`/`Cargo.lock`, and the `Makefile` are off-limits — **except** chunk 12, which intentionally edits `plugins/vectis/skills/{core,ios,android}-writer/`, and chunk 13, which adds `plugins/vectis/skills/template-updater/`.
- Do not edit `scripts/checks.ts`. The new files live under directories `checks.ts` does not enforce schema on.
- Do not introduce shared crates (`crates/common/` etc.) in this phase. Anything reusable later belongs inside `crates/vectis-cli/src/` for now and can be lifted when the workspace CLI lands.

## Initial Version Pins (vetted today against crates.io)

These are the values chunk 4 should embed in `crates/vectis-cli/embedded/versions.toml`. Crux's transitive constraints have been verified by reading each crate's published `Cargo.toml`.

```toml
# Embedded defaults — overridden by ~/.config/vectis/versions.toml or
# project-local versions.toml. See RFC-6 § Version Management.

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
# Initial values copied from RFC-6; chunk 11 (`update-versions`) will pull
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
- `cargo-swift 0.11.0` and `uniffi 0.31.0` are also available; switching requires re-verifying the iOS pipeline end-to-end. **Chunk 11 attempted this bump and reverted it**: `crux_core 0.17.0`'s `cli` feature pins `uniffi_bindgen = "=0.29.4"`, so mixing a 0.31 runtime with a 0.29 bindgen breaks `codegen kotlin` (`unresolved module path shared::ffi`). The pair stays at `uniffi = "=0.29.4"` / `cargo_swift = "0.9"` until chunk 13 (`template-updater`) rewrites `templates/vectis/core/codegen.rs` to call `uniffi_bindgen 0.31` directly, or a crux_core release tracks 0.31 bindgen. The Xcode 26+ iOS concern is handled by the *local* cargo-swift install (chunk-7 Notes); cargo-swift 0.11's bindgen can read uniffi 0.29.4's metadata.
- Android pins above are superseded by the chunk-3c bumps recorded in the status table's Note column for chunk 3c. Chunk 11 wires up `update-versions` which can query Google Maven / Maven Central live, but the automatic proposals have to be sanity-checked: AGP currently latest is `9.1.1` which would require Gradle 9.x and break `rust-android-gradle 0.9.6` -- see chunk 11 + chunk 13 Notes.

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

Committed on `vectis-cli` (single-branch policy -- all chunks land linearly on this branch).

Verification (all four subcommands exit 1 with valid JSON; `make checks` and `cargo clippy --release -p vectis-cli --all-targets -- -D warnings` both pass):

```text
$ ./target/release/vectis --help
Vectis CLI -- scaffolds the deterministic 'Hello World' starting point for Crux apps (core + optional iOS/Android shells) and verifies that every assembly compiles. See RFC-6.

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

Committed on `vectis-cli` (originally on a per-chunk branch; later consolidated -- see Decision Log).

Verification (all clean):

```text
$ cargo build --release -p vectis-cli            # ok
$ cargo clippy --release -p vectis-cli --all-targets -- -D warnings   # ok
$ cargo test -p vectis-cli prerequisites
running 19 tests
test prerequisites::tests::assembly_tag_strings ... ok
test error::tests::missing_prerequisites_json_shape ... ok
test prerequisites::tests::extract_from_cargo_swift_output ... ok
test prerequisites::tests::extract_from_gradle_output ... ok
test prerequisites::tests::extract_from_modern_java_output ... ok
test prerequisites::tests::extract_from_old_java_output ... ok
test prerequisites::tests::extract_from_cargo_deny_output ... ok
test prerequisites::tests::extract_from_xcodegen_output ... ok
test prerequisites::tests::extract_returns_none_when_no_version ... ok
test prerequisites::tests::extract_skips_year_like_tokens ... ok
test prerequisites::tests::version_display ... ok
test prerequisites::tests::version_ordering ... ok
test prerequisites::tests::version_parse_basic ... ok
test prerequisites::tests::version_parse_rejects_garbage ... ok
test prerequisites::tests::version_parse_strips_suffix ... ok
test prerequisites::tests::cmd_check_missing_program_fails ... ok
test prerequisites::tests::env_check_empty_var_is_failure ... ok
test prerequisites::tests::env_check_unset_var_is_failure ... ok
test prerequisites::tests::cmd_check_min_version_too_low_fails ... ok
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
$ make checks                                    # All checks passed.
```

Smoke tests against the binary (with all workstation tools installed):

```text
$ ./target/release/vectis init Counter --shells ios,android   # exit 1 (not_implemented; prereqs ok)
$ ./target/release/vectis verify                              # exit 1 (not_implemented; prereqs ok)
$ ./target/release/vectis add-shell ios                       # exit 1 (not_implemented; prereqs ok)
$ ./target/release/vectis update-versions --dry-run           # exit 1 (not_implemented; prereqs ok)
```

Missing-prereq path (sanitised PATH / unset env var):

```text
$ PATH=/usr/bin:/bin:/usr/sbin:/sbin:/Users/goldie/.cargo/bin \
    ./target/release/vectis init Counter --shells ios
{
  "error": "missing_prerequisites",
  "message": "Install the missing tools above and re-run the command.",
  "missing": [
    { "tool": "xcodegen",   "assembly": "ios", "check": "xcodegen --version",   "install": "brew install xcodegen" },
    { "tool": "xcbeautify", "assembly": "ios", "check": "xcbeautify --version", "install": "brew install xcbeautify" }
  ]
}    # exit 2

$ env -u ANDROID_HOME ./target/release/vectis init Counter --shells android
{
  "error": "missing_prerequisites",
  ...
  "missing": [
    { "tool": "android-sdk", "assembly": "android", ... },
    { "tool": "android-ndk", "assembly": "android", ... }
  ]
}    # exit 2
```

InvalidProject path for unknown shell platforms:

```text
$ ./target/release/vectis init Counter --shells nonsense
{ "error": "invalid_project", "message": "unknown shell platform: \"nonsense\" ..." }    # exit 1
```

Implementation deviations and discoveries (all minor):

- **Tool registry as a `static`, not a function.** The chunk text just says "Hard-code the table from RFC § Workstation Requirements"; I used a `static TOOLS: &[Tool] = &[...]` (with `all_tools()` returning it) so the slice has `'static` lifetime. Building the slice inside a function returns a reference to a temporary -- doesn't compile.
- **Per-tool stable name, not the RFC's check command.** The RFC's table sometimes labels a row as "Xcode + Command Line Tools" or "Android SDK (`$ANDROID_HOME`)". I used flat identifiers (`xcode`, `android-sdk`, `rustup-android-targets`, `android-ndk`) for the `tool` field of the JSON payload so it's machine-readable. The `check` field carries the exact RFC string (`xcode-select -p`, `echo $ANDROID_HOME`, etc.) so users still see the same display.
- **Args structs are now `pub(crate)` and passed to handlers.** Chunks 5/7/8/9/10/11 all carried a deferred note that said "the `Init(_)` arm in `main::main` will need to pass `&InitArgs` through to `init::run`". Doing this in chunk 2 was unavoidable because the prerequisite check needs `args.shells` (init), `args.dir` (verify), `args.platform` (add-shell), and `args.verify` (update-versions). I propagated this to the status row notes for those chunks so they aren't redoing the work.
- **Per-field `#[allow(dead_code)]` on Args structs.** clap's derive populates every field, but fields not yet read by the (still-stub) handlers trigger dead-code warnings under `-D warnings`. Each unused field carries `#[allow(dead_code)] // consumed by chunk N` so the lint stays on by default. Later chunks should remove these as they start reading the fields.
- **`VectisError` dead-code attribute narrowed.** Per the chunk 2 status-row instruction, the crate-wide `#[allow(dead_code)]` on `VectisError` was replaced with per-variant attributes on only the still-unused `Verify` and `Internal`. `MissingPrerequisites` is now constructed by `prerequisites::check`; `InvalidProject` is now constructed by `init::run` and `add_shell::run` for unknown platform inputs.
- **Java version floor enforced; nothing else has one.** The RFC says "validates version minimums where they matter (e.g., Java 21, not Java 17)". Only Java has a `min_version` set today (21.0.0). Other tools just need to invoke successfully. The `extract_version` helper is generic enough that adding more floors is one literal per tool.
- **`extract_version` is a tiny string scanner, not a regex.** Splits on any non-`[0-9.]` character and keeps the first dot-containing token that parses as `M.m[.p]`. Tested against real output samples from `cargo-swift`, `cargo-deny`, modern Java (21.0.10), legacy Java (1.8.0_221), Gradle, and XcodeGen. Year-like tokens (`2026`) are filtered because they have no dot.
- **`run_check` swallows the per-tool reason.** The reason a check failed (e.g. "found 17.0.0 but need >= 21.0.0") is computed but discarded -- only the tool's name/install hint surface in the JSON. The RFC's example payload has the same shape (no per-tool diagnostic), so this is intentional. If we ever want richer diagnostics, the `MissingTool` struct can grow a `reason: Option<String>` field without breaking JSON consumers.
- **`verify::run` does on-disk assembly detection inline.** Looking for `iOS/` and `Android/` directories under `--dir` was the simplest way to scope the prereq check correctly (per the RFC: "verify auto-detects which assemblies exist"). Chunk 9 will need the same detection for the actual pipeline; reuse or extract is an option then.
- **Two `unsafe` blocks in tests.** `std::env::set_var` / `remove_var` are `unsafe` in Rust 2024 edition. They're scoped to the env-check tests and use a reserved variable name (`VECTIS_TEST_EMPTY`) to avoid colliding with anything real.

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

Committed on `vectis-cli` (originally on a per-chunk branch; later consolidated -- see Decision Log).

The 13 template files and their `MANIFEST.md` live at `templates/vectis/core/`. Source filenames are flat (`workspace-cargo.toml`, `shared-cargo.toml`, `gitignore`, `lib.rs`, `app.rs`, `ffi.rs`, `codegen.rs`, `clippy.toml`, `rust-toolchain.toml`, `deny.toml`, `supply-chain-config.toml`, `supply-chain-audits.toml`, `supply-chain-imports.lock`); target paths (`Cargo.toml`, `shared/Cargo.toml`, `.gitignore`, `shared/src/lib.rs`, `shared/src/app.rs`, ...) are recorded in `MANIFEST.md`.

Verification (`cargo check` on a Counter render-only render):

```text
$ rm -rf /tmp/vectis-3a-check && mkdir /tmp/vectis-3a-check && cd /tmp/vectis-3a-check
$ # Stage templates per MANIFEST.md path mapping (see "Implementation deviations" below):
$ mkdir -p shared/src/bin supply-chain
$ cp $REPO/templates/vectis/core/workspace-cargo.toml      Cargo.toml
$ cp $REPO/templates/vectis/core/clippy.toml               clippy.toml
$ cp $REPO/templates/vectis/core/rust-toolchain.toml       rust-toolchain.toml
$ cp $REPO/templates/vectis/core/gitignore                 .gitignore
$ cp $REPO/templates/vectis/core/shared-cargo.toml         shared/Cargo.toml
$ cp $REPO/templates/vectis/core/lib.rs                    shared/src/lib.rs
$ cp $REPO/templates/vectis/core/app.rs                    shared/src/app.rs
$ cp $REPO/templates/vectis/core/ffi.rs                    shared/src/ffi.rs
$ cp $REPO/templates/vectis/core/codegen.rs                shared/src/bin/codegen.rs
$ cp $REPO/templates/vectis/core/deny.toml                 deny.toml
$ cp $REPO/templates/vectis/core/supply-chain-config.toml  supply-chain/config.toml
$ cp $REPO/templates/vectis/core/supply-chain-audits.toml  supply-chain/audits.toml
$ cp $REPO/templates/vectis/core/supply-chain-imports.lock supply-chain/imports.lock
$ find . -type f \( -name '*.toml' -o -name '*.rs' -o -name '*.lock' -o -name '.gitignore' \) -exec sed -i '' \
    -e 's/__APP_NAME__/Counter/g' \
    -e 's/__APP_STRUCT__/Counter/g' \
    -e 's/__CRUX_CORE_VERSION__/0.17.0/g' \
    -e 's/__FACET_VERSION__/=0.31/g' \
    -e 's/__UNIFFI_VERSION__/=0.29.4/g' \
    -e 's/__SERDE_VERSION__/1.0/g' \
    -e 's/__ANDROID_PACKAGE__/com.vectis.counter/g' {} \;
$ find . -type f \( -name '*.toml' -o -name '*.rs' -o -name '*.lock' \) -exec sed -i '' '/<<<CAP:/,/CAP:.*>>>/d' {} \;
$ cargo check
    Updating crates.io index
    ... (250 packages locked)
    Checking shared v0.1.0 (/private/tmp/vectis-3a-check/shared)
    Finished `dev` profile in 11.91s
```

Bonus checks (not required by the chunk verification, but they exercise paths chunks 5 and 9 will need):

```text
$ cargo check --features codegen,facet_typegen
    Finished `dev` profile in 21.63s
$ cargo run --bin codegen --features codegen,facet_typegen -- --language swift --output-dir /tmp/vectis-3a-codegen-swift
     Running `target/debug/codegen --language swift ...`
$ ls /tmp/vectis-3a-codegen-swift
SharedTypes/
$ cargo build --features uniffi --lib && cargo run --bin codegen --features codegen,facet_typegen -- --language kotlin --output-dir /tmp/vectis-3a-codegen-kotlin
     Running `target/debug/codegen --language kotlin ...`
Code generation complete, formatting with ktlint (use --no-format to disable)
$ ls /tmp/vectis-3a-codegen-kotlin
com/  uniffi/
$ ls /tmp/vectis-3a-codegen-kotlin/com/vectis/counter
Counter.kt  Requests.kt
```

(The ktlint formatting warning is benign -- ktlint is not on `$PATH` on the build host. The Kotlin codegen output is well-formed and namespaced under `com/vectis/counter` as expected from `__ANDROID_PACKAGE__` substitution.)

Implementation deviations and discoveries (all minor; status-row notes for chunks 4-8 updated to absorb them):

- **Verification recipe in the chunk text is under-specified.** The chunk says `cp -r $REPO/templates/vectis/core/* .` and then `cargo check`, but the source filenames are flat (`workspace-cargo.toml`, `shared-cargo.toml`, `lib.rs`, ...) while cargo expects nested target paths (`Cargo.toml`, `shared/Cargo.toml`, `shared/src/lib.rs`, ...). Followed `MANIFEST.md`'s source→target mapping when staging the verification project. Recommend tightening 3b/3c verification recipes the same way -- iOS templates land under `iOS/{AppName}/...`, Android under `Android/...`, neither matches the flat templates layout.
- **Capability-version placeholders are not in the RFC's placeholder table.** `templates/vectis/core/workspace-cargo.toml` uses `__CRUX_HTTP_VERSION__`, `__CRUX_KV_VERSION__`, `__CRUX_TIME_VERSION__`, `__CRUX_PLATFORM_VERSION__` inside `<<<CAP:...>>>` blocks. They are stripped along with the cap markers in the render-only verification, so chunk 3a's `cargo check` doesn't exercise them. Chunk 6 (capability variants) must substitute them from chunk 4's `Versions` struct. The RFC's Initial Version Pins block already pins all five Crux crate versions, so no upstream RFC change is required -- only the placeholder table needs the additions.
- **`__ANDROID_PACKAGE__` lives in core, not just Android.** `shared/src/bin/codegen.rs` uses it as the Kotlin package namespace (the equivalent of Swift's `"SharedTypes"` constant). For core-only or iOS-only renders, the codegen binary still has to compile, so chunk 5 must substitute `__ANDROID_PACKAGE__` with the default `com.vectis.<lower app name>` even when `--shells android` is absent. The default is unambiguous because the placeholder text is namespacing the generated Kotlin tree, not declaring a package the user runs against.
- **`thiserror = "2"` added as an optional dep.** The chunk's reference docs (`crux-ffi-scaffolding.md`) use `#[derive(thiserror::Error)]` on `CoreError` but never list `thiserror` as a `[dependencies]` row. The previous skills must have been adding it ad hoc. Pinned `thiserror = "2"` in `shared-cargo.toml` and gated it behind both `uniffi` and `wasm_bindgen` features so the dependency only enters the dep graph when `ffi.rs` is actually compiled.
- **`Event` derives drop `PartialEq, Eq`.** The reference example derives them, but the http capability variant carries a `crux_http::Response<Vec<u8>>` payload that isn't `Eq`. Render-only would compile either way; dropping these derives keeps the template forward-compatible with chunks 6+.
- **`Effect` variants per-cap are placement-only.** Render-only's `Effect` is just `Render(RenderOperation)`. The CAP-fenced `Http(HttpRequest)` / `KeyValue(KeyValueOperation)` / `Time(TimeRequest)` / `Platform(PlatformRequest)` variants are present in the file but stripped for the render-only verification. Chunk 6 will exercise them. The `sse` cap has no Effect variant today (only a `[dependencies]` block in `shared-cargo.toml`); chunk 6 should decide whether to add `Sse(...)` and a matching `app.rs` block.
- **CAP marker semantics fixed.** Markers must each occupy their own line (`<<<CAP:foo` opens, `CAP:foo>>>` closes). The `sed` recipe uses `/<<<CAP:/,/CAP:.*>>>/d` which deletes the entire fenced range inclusive of both markers. Chunk 5's engine should mirror that semantic when stripping; chunk 6's engine should drop only the marker lines (preserving content) when the cap is selected. Indentation inside markers is preserved verbatim, which matters for the `codegen` and `facet_typegen` feature arrays in `shared-cargo.toml` where retained CAP content becomes inline list elements.
- **`MANIFEST.md` includes a CI self-check.** A short `diff` snippet at the bottom of the manifest validates that every file in `templates/vectis/core/` is listed exactly once. Worth wiring into `make checks` later, but out of scope for this chunk (the chunk text restricts this work to `templates/vectis/core/` only).

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

Committed on `vectis-cli` (single-branch policy).

The 7 template files and their `MANIFEST.md` live at `templates/vectis/ios/`. Source filenames are flat (`project.yml`, `Makefile`, `App.swift`, `Core.swift`, `ContentView.swift`, `LoadingScreen.swift`, `HomeScreen.swift`); target paths (`iOS/project.yml`, `iOS/__APP_NAME__/__APP_NAME__App.swift`, ...) are recorded in `MANIFEST.md`.

Verification (paired-core staging + xcodegen + Swift typecheck against synthetic SharedTypes/Shared stubs, exercised across three cap profiles):

```text
$ # render-only (no caps), HTTP cap, all four caps (http+kv+time+platform).
$ # Each variant: stage chunk-3a + chunk-3b templates per MANIFEST source→target
$ # mapping, sed-substitute placeholders, strip / unwrap CAP markers.
$ cargo check                                          # core compiles in all 3 variants
    Finished `dev` profile in ~15s
$ make typegen                                         # SharedTypes Swift package generated
    INFO  crux_core::type_generation::facet > Generating Swift types
$ make package                                         # cargo-swift produces Shared/RustFramework.xcframework
    Building Shared Swift package...
$ make xcode                                           # xcodegen produces Counter.xcodeproj
    Created project at /tmp/vectis-3b-check/iOS/Counter.xcodeproj
$ swiftc -typecheck \                                  # iOS shell sources type-check vs. stubs
    -target arm64-apple-ios17.0-simulator -sdk $(xcrun --sdk iphonesimulator --show-sdk-path) \
    -I /tmp/.../modules-{render,http,allcaps} \
    CounterApp.swift Core.swift ContentView.swift Views/LoadingScreen.swift Views/HomeScreen.swift
    # exit=0 for all three cap profiles
```

The chunk text's `xcodebuild ... build` step is **partial** by the chunk's own admission ("full pipeline verification arrives in chunk 7"), and on this host it currently fails inside the `Shared` Swift package generated by `cargo swift package` -- the package's `shared.swift` fails to resolve `import sharedFFI` ("cannot find type 'RustBuffer' in scope") under Xcode 16 / Swift 6 with the chunk-3a/4 pinned versions. This is a tooling-version drift issue (cargo-swift 0.9.0 ships uniffi_bindgen 0.29.1, runtime is `=0.29.4`), not a problem with the iOS templates -- the templates themselves type-check cleanly against synthetic stubs of the FFI surface, and xcodegen consumes `project.yml` without complaint. Captured as a chunk-7 to-do in that chunk's status row; the chunk-3b deliverable (templates + MANIFEST) is unaffected.

Implementation deviations and discoveries (all minor; status-row notes for chunks 5/6/7 updated to absorb them):

- **`__APP_NAME__` substitution applies to target paths, not just file contents.** The MANIFEST records targets like `iOS/__APP_NAME__/__APP_NAME__App.swift`, which substitute to `iOS/Counter/CounterApp.swift` (two `__APP_NAME__` occurrences in the path itself, one in the directory segment and one in the file-name prefix). Chunk 7's engine must run the placeholder substitution over the constructed target path before opening the file for write -- the on-disk template layout deliberately stays flat. Chunk 3a's core targets don't exercise this (their target paths are static), so this is the first time path-level substitution lands. Status-row note on chunk 7 updated.

- **New placeholder `__APP_NAME_LOWER__`.** The RFC's placeholder table lists it (`__APP_NAME_LOWER__` example value `counter`), but chunk 3a didn't have to use it (core templates don't carry per-app bundle/package strings). The iOS `project.yml` uses it for `bundleIdPrefix` and per-config `PRODUCT_BUNDLE_IDENTIFIER`. The engine derives it from `args.app_name` via `to_lowercase()` -- it is not a CLI flag. Chunk 7 must add the field to the `Params` struct alongside `__APP_NAME__` / `__APP_STRUCT__`.

- **VectisDesign and Inject deliberately omitted.** The `ios-writer` reference docs (`ios-project-config.md`, `swiftui-view-patterns.md`, `crux-ios-shell-pattern.md`) reference both. The chunk-3b templates omit them so the deterministic baseline always compiles -- `VectisDesign` lives at `design-system/ios/` and is produced by a separate skill, and `Inject` pulls a network SPM dep + requires the developer to install InjectionIII. Both can come back as cap-style toggles in a future RFC. Rationale and re-enable instructions captured in `templates/vectis/ios/MANIFEST.md` § Design system / Inject.

- **No `<<<CAP:sse>>>` block in `Core.swift`.** Mirrors today's `app.rs` (chunk 3a deferred to chunk 6 the question of whether to add an `Effect::Sse(...)` variant -- the `sse` cap currently only changes `shared-cargo.toml`). When chunk 6 lands the Rust-side variant, chunk 7 (or a follow-up to 3b) needs to add the matching Swift case to the template's `processEffect(_:)` switch, gated by `<<<CAP:sse`.

- **Cap markers carry both case arms and helpers.** Swift enforces exhaustive switches on enums, so each cap-conditional region in `Core.swift` includes both the matching `case .http(...):` arm in `processEffect(_:)` and any helper functions it relies on (e.g. `performHttpRequest`), all inside the same `<<<CAP:http ... CAP:http>>>` block. The engine must not split or reorder marker contents. Chunk 3a's marker semantics already cover this (whole-region drop / marker-line strip); restated in the iOS MANIFEST so chunk 7 doesn't have to derive it.

- **Cargo-swift / uniffi version drift surfaced during verification.** With chunk-3a/4's pinned `cargo_swift = "0.9"` (uniffi_bindgen 0.29.1) + `uniffi = "=0.29.4"` runtime, `xcodebuild` against the cargo-swift-produced `Shared` package fails at `import sharedFFI` ("cannot find type 'RustBuffer' in scope") under Xcode 16 / Swift 6. The iOS templates themselves are clean (verified by typechecking against synthetic stubs in three cap profiles); the failure is in the generated `shared.swift` upstream of any vectis code. Documented as a chunk-7 to-do; possible resolutions are bumping cargo-swift to 0.10/0.11 (cascades into chunks 4 and 11), tightening the `uniffi` pin to `=0.29.1` to match the bundled bindgen, or carrying a post-package patch on `shared.swift`. None of the options need to land before chunk 7 starts work, but chunk 7's first task should be picking one.

- **MANIFEST self-check tightened to filter the cap-marker reference table.** Chunk 3a's awk pattern (`/^\| `[a-z]/`) matched lowercase identifiers, which conflicted with the iOS file names (`App.swift`, `Core.swift` -- PascalCase). The iOS self-check accepts mixed-case identifiers but filters the right-hand side to tokens that look like file names (`\.[A-Za-z]+$|^Makefile$`), so cap names (`http`, `kv`, ...) from the cap-marker table don't pollute the diff. `command ls -1` (instead of bare `ls`) sidesteps the ambient `ls='ls -lpa'` alias that would otherwise inject `./` / `../` rows. Same diff structure as chunk 3a.

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

- Verification recipe is under-specified the same way 3a/3b were. The chunk text above stages into `/tmp/vectis-3c-check/Android` but template filenames are flat (`root-build.gradle.kts`, `app-build.gradle.kts`, `Application.kt`, ...) while gradle expects nested target paths (`build.gradle.kts`, `app/build.gradle.kts`, `app/src/main/java/<pkg>/CounterApplication.kt`, ...). Used `templates/vectis/android/MANIFEST.md`'s source→target mapping when staging. Path-segment substitution must replace `__APP_NAME__` and `__ANDROID_PACKAGE_PATH__` (the latter is `__ANDROID_PACKAGE__` with `.` -> `/`) in directory and file-name positions, not just file contents.
- `gradle wrapper --gradle-version 8.13` cannot be run with a system-installed Gradle 9.x: `rust-android-gradle = 0.9.6` calls `AbstractCopyTask.setFileMode(Integer)` which Gradle 9 removed, so plugin resolution fails before the wrapper is generated. Bootstrap with an 8.x gradle binary instead. Verification used a downloaded `gradle-8.13` distribution.
- `gradle.properties` template omits `org.gradle.java.home` (per-machine path); Java 21 is required and was provided via `JAVA_HOME=$(/usr/libexec/java_home -v 21)`. Chunk 8 should write this line at scaffold time.
- Initial Version Pins block is stale for Android. Verified-working values used during chunk 3c: `agp = "8.13.2"`, `kotlin = "2.3.0"`, `compose_bom = "2026.01.01"`, `ktor = "3.4.0"`, `koin = "4.1.1"` (the older block values do not produce a buildable APK against Xcode 16 / Java 21 toolchains). Chunk 4 must bump the embedded defaults; chunk 11 must understand the new placeholder names (`__AGP_VERSION__`, `__KOTLIN_VERSION__`, `__COMPOSE_BOM_VERSION__`, `__KTOR_VERSION__`, `__KOIN_VERSION__`).
- `__ANDROID_NDK_VERSION__` is a new placeholder (in `shared-build.gradle.kts`) not in the RFC's placeholder table or chunk-4's `Versions::android` substruct. Verification substituted from `$ANDROID_HOME/ndk/<version>/` at staging time. Chunk 4 should add an `ndk` field; chunk 8 should fall back to local detection if the field is absent.
- `network-security-config.xml` has no internal CAP markers -- the whole file is conditional on `http` or `sse`. Engine needs a "skip this whole file if cap missing" predicate.
- `koin-bom`/`ktor-*` deps in `libs.versions.toml` and `app-build.gradle.kts`, plus the `viewModelScope`/coroutine plumbing in `Core.kt`, are gated only on `<<<CAP:http`. The non-HTTP cap arms in `Core.kt` (`kv`, `time`, `platform`) are TODO stubs that bind `effect.value` to a `@Suppress("UNUSED_VARIABLE")` local and do nothing else (the render-only baseline never emits these effects). The `sse` cap intentionally has no entry in `Core.kt` -- `app.rs` has no `Effect::Sse` variant in the render-only baseline (mirrors chunk 3a/3b). When chunk 6 adds the Rust-side variant, this manifest, `libs.versions.toml`, `AndroidManifest.xml`, and `Core.kt` need matching `<<<CAP:sse` blocks.
- Templates intentionally omit `:vectis-design`, the Koin `AppModule.kt`, and per-cap helper classes (`HttpClient.kt`, `SseClient.kt`, `KeyValueClient.kt`). Mirrors chunk 3b. Writer skills layer them in during Update Mode (Pattern 1 baseline vs Pattern 2 in `crux-android-shell-pattern.md`).
- Verification: `cargo check` on staged paired core ✅; `gradle wrapper --gradle-version 8.13` ✅ (via downloaded 8.x bootstrap); `make build` (codegen) ✅; `./gradlew :app:assembleDebug` ✅.

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

Committed on `vectis-cli` (single-branch policy).

Verification (all four gates green):

```text
$ cargo build --release -p vectis-cli                            # ok
$ cargo clippy --release -p vectis-cli --all-targets -- -D warnings   # ok
$ cargo test -p vectis-cli versions
running 9 tests
test versions::tests::missing_override_returns_invalid_project_error ... ok
test versions::tests::embedded_defaults_parse_and_match_initial_pins ... ok
test versions::tests::directory_passed_as_override_returns_invalid_project_error ... ok
test versions::tests::no_home_falls_through_to_embedded ... ok
test versions::tests::malformed_override_returns_invalid_project_error_with_path ... ok
test versions::tests::embedded_layer_used_when_no_files_or_overrides ... ok
test versions::tests::override_layer_takes_precedence_over_everything ... ok
test versions::tests::user_layer_takes_precedence_over_embedded ... ok
test versions::tests::project_layer_takes_precedence_over_user_and_embedded ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out
$ cargo test -p vectis-cli                                       # 29 passed; 0 failed
$ make checks                                                    # All checks passed.
```

Smoke tests against the binary:

```text
$ ./target/release/vectis init Counter --version-file /nonexistent.toml ; echo "exit=$?"
{
  "error": "invalid_project",
  "message": "version file not found: /nonexistent.toml"
}
exit=1

$ ./target/release/vectis init --help | head -20
Scaffold a new Crux project (core, plus optional shells)
Usage: vectis init [OPTIONS] <APP_NAME>
...
      --version-file <VERSION_FILE>
          Override version pins file. When set, the file MUST exist; resolution
          otherwise falls back to `<project>/versions.toml`,
          `~/.config/vectis/versions.toml`, then the embedded defaults

# All four subcommands accept --version-file (verify/add-shell/update-versions
# parse it but do not yet act on it -- chunks 9/10/11 will).
```

Implementation deviations and discoveries (all minor; status-row notes for chunks 5-11 updated):

- **Android pins bumped vs. the Initial Version Pins block.** The chunk text says "use the Initial Version Pins block at the top of this file verbatim", but chunk 3c's verification proved the original Android values do not produce a buildable APK on Xcode 16 / Java 21 toolchains. Embedded defaults use the chunk-3c verified set (`agp = "8.13.2"`, `kotlin = "2.3.0"`, `compose_bom = "2026.01.01"`, `ktor = "3.4.0"`, `koin = "4.1.1"`). All other pins (Crux, hard pins, `gradle`, tooling) match the Initial Version Pins block verbatim. The chunk 4 status row already directed this bump.
- **`Versions::android.ndk` is `Option<String>` and omitted from embedded defaults.** Chunk 3c introduced `__ANDROID_NDK_VERSION__` in `shared-build.gradle.kts`. The status row left it as a choice between "field present" or "chunk 8 detects from disk". Chose both: the field exists on the struct (so projects that want to pin can do so via project/user `versions.toml`), but is `Option<String>` and absent from the embedded defaults so chunk 8 detects from `$ANDROID_HOME/ndk/<version>/` for the default scaffold (avoids confusing "NDK not found" errors for developers who installed a different version).
- **`--version-file` semantics differ across subcommands -- two different intents on the same flag name.** The chunk text says "A `--version-file <path>` global flag on every subcommand". On `init`/`verify`/`add-shell` the flag is a resolution override (the file must exist; missing → `InvalidProject`). On `update-versions` the flag is the *write target* (the file may not exist yet; chunk 11 will create/update it). I kept the same flag name on all four because that is what the chunk text and the RFC's `update-versions` Options block both call it; the help text disambiguates. Each handler decides whether to call `Versions::resolve(..., args.version_file.as_deref())`.
- **Only `init::run` calls `Versions::resolve` in this chunk.** The chunk verification only exercises init's path; wiring resolve into verify/add-shell/update-versions in chunk 4 would either (a) require also implementing each handler's real flow (out of scope) or (b) eagerly fail commands that currently work because the user has no `~/.config/vectis/versions.toml` and would otherwise fall through to embedded defaults harmlessly. The other handlers' `version_file` fields carry `#[allow(dead_code)] // consumed by chunk N` and the resolution call lands when each handler's chunk does.
- **`Versions::resolve_with(project, override, home)` factored out of `resolve(project, override)`.** Avoids `unsafe { std::env::set_var("HOME", ...) }` in the unit tests (Rust 2024 makes `set_var` `unsafe` and parallel tests would race on it anyway). Public surface is the chunk-specified `Versions::resolve(project_dir, override_path)`; the inner helper is `pub(super)`-shaped (`fn`, not `pub fn`) so future callers cannot accidentally bypass `$HOME` resolution.
- **Resolver returns `VectisError::Internal` for malformed embedded defaults, `VectisError::InvalidProject` for malformed user/project/override files.** A malformed embedded blob is a build/release-time bug in vectis itself; a malformed user file is a configuration error. Different error variants make the distinction visible in the JSON output. The existing chunk-2 narrowing of `VectisError::Internal`'s `#[allow(dead_code)]` was loosened (now `Internal` is constructed by chunk 4, not just chunk 11); kept the per-variant attribute on `Verify` only.
- **`Versions::embedded()` is public for chunk 11.** `update-versions --dry-run` will need to compare "current" (resolved chain) against "proposed" (registry queries), and when no user/project file exists yet "current" should be the embedded baseline, not whatever `resolve` produced. Carries `#[allow(dead_code)] // consumed by chunk 11` until chunk 11 lands.
- **Two field-name conventions in the wild for some Android pins (`compose_bom` vs `compose-bom`).** The Initial Version Pins block uses underscores; the RFC § Version Management example uses hyphens. Chose underscores (matching the Initial Version Pins block, since the chunk says "verbatim") because TOML field-name → Rust struct-field mapping is identity for underscores but requires `#[serde(rename = "compose-bom")]` for hyphens. If we want to accept the hyphenated form too, that is an additive change in chunk 11 (add `#[serde(alias = "compose-bom")]` etc.); not doing it now keeps chunk 4 minimal.
- **`toml = "0.8"`** added to `crates/vectis-cli/Cargo.toml`. No other deps changed; the `dirs` crate was deemed unnecessary since `std::env::var_os("HOME")` plus `PathBuf::join` is sufficient on macOS / Linux (Windows is not a target per the prereqs table).

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

Committed on `vectis-cli` (single-branch policy).

Verification (all four chunk-5 gates green):

```text
$ rm -rf /tmp/vectis-5-check && \
    ./target/release/vectis init Counter --dir /tmp/vectis-5-check | jq .
{
  "app_name": "Counter",
  "app_struct": "Counter",
  "assemblies": {
    "core": {
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
      ],
      "status": "created"
    }
  },
  "capabilities": [],
  "project_dir": "/tmp/vectis-5-check",
  "shells": []
}                                                # exit 0

$ cd /tmp/vectis-5-check && cargo check
    Checking shared v0.1.0 (/private/tmp/vectis-5-check/shared)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.64s    # exit 0

$ cargo clippy --all-targets -- -D warnings
    Checking shared v0.1.0 (/private/tmp/vectis-5-check/shared)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.97s    # exit 0

$ cargo run --bin codegen --features codegen,facet_typegen -- \
    --language swift --output-dir /tmp/vectis-5-codegen-swift
    ... (250 packages)
    Running `target/debug/codegen --language swift --output-dir /tmp/vectis-5-codegen-swift`
    # exit 0; SharedTypes/ produced under output dir
```

Bonus checks (not required by the chunk gates):

```text
$ cargo test -p vectis-cli                                           # 50 passed; 0 failed
$ cargo clippy --release -p vectis-cli --all-targets -- -D warnings  # exit 0
$ make checks                                                        # All checks passed.

# Refusal-to-overwrite (rerun against the populated /tmp/vectis-5-check):
$ ./target/release/vectis init Counter --dir /tmp/vectis-5-check
{ "error": "invalid_project",
  "message": "refusing to overwrite existing file at /tmp/vectis-5-check/Cargo.toml ..." }    # exit 1
$ ls /tmp/vectis-5-check/Cargo.toml.bak                                                       # never written
ls: cannot access ...

# PascalCase validation:
$ ./target/release/vectis init counter --dir /tmp/vectis-5-bad
{ "error": "invalid_project",
  "message": "app name \"counter\" must start with an ASCII uppercase letter (PascalCase, ...)" }    # exit 1
```

Implementation deviations and discoveries (status-row notes for chunks 6/7/8 updated to absorb them):

- **Chunk-3a/4 template gap surfaced by the chunk-5 clippy gate.** Chunk 3a's verification ran `cargo check` only, so it never exercised `cargo clippy --all-targets -- -D warnings`. The chunk-5 clippy gate exposes three `cargo`-group lints that fire on every render-only scaffold: `clippy::cargo_common_metadata` (the scaffolded `shared` crate has no description/license/repository because it isn't published to crates.io), `clippy::multiple_crate_versions` (Crux's transitive deps bring `syn 1` + `syn 2` and similar duplicates the user cannot deduplicate), and `clippy::lint_groups_priority` (lint groups and per-lint overrides at the same priority is a hard error under `-D warnings` since clippy 1.74). Patched in-place in `templates/vectis/core/workspace-cargo.toml`: the four lint *groups* (`all`, `nursery`, `pedantic`, `cargo`) now use `{ level = "warn", priority = -1 }`, and `cargo_common_metadata` / `multiple_crate_versions` are explicitly `allow`. Chunks 7/8 should run their respective `clippy --all-targets -- -D warnings` gates expecting the same group-priority hygiene to be needed for any new lints they introduce.
- **`templates::core::TEMPLATES` is the embedded source-of-truth, not a parsed `MANIFEST.md`.** The chunk text said "read `templates/vectis/core/MANIFEST.md`'s source→target mapping (or an embedded copy of it)". Parsing the markdown table at runtime would be brittle (table format breaks change behaviour) and would require shipping the manifest into the binary -- there is no compelling reason to do so since the engine just needs `(target_path, contents)` pairs. Encoded as a `&'static [CoreTemplate]` slice with `include_str!` per entry; a unit test (`registry_matches_rfc_core_file_count`) pins the size so any new template forces a manifest update at the same time. Chunks 7/8 should follow the same pattern (`templates::ios::TEMPLATES`, `templates::android::TEMPLATES`) -- they do not need to re-parse their MANIFESTs either.
- **Placeholder substitution is superstring-first.** `__APP_NAME_LOWER__` is a strict superstring of `__APP_NAME__`; the `replace()` chain in `templates::mod.rs::substitute_placeholders` runs the lowercase form *first*, otherwise `__APP_NAME__` would corrupt `__APP_NAME_LOWER__` into `Counter_LOWER__`. Captured as a regression test (`substitutes_app_name_lower_before_app_name`). Chunks 6/7 adding new placeholders must slot them into the same chain in superstring-first order.
- **Render-only `--caps` / `--shells` are guarded with structured errors instead of silently ignored.** Chunk 5's scope is core, render-only. The handler accepts empty `--caps "" --shells ""` (so build orchestration that always passes the flags doesn't break) but rejects any non-empty value with a `VectisError::InvalidProject` pointing at the chunk that owns the work (6 for `--caps`, 7/8 for `--shells`). Chunk 6 should replace the `--caps` guard with the comma-split parser; chunks 7/8 should replace the `--shells` guard with their scaffold dispatch (and at that point lift the parser from `init::run::parse_shells` -- it already returns `Vec<AssemblyKind>` in scaffold order).
- **`Capability` enum landed in chunk 5, not chunk 6.** The engine's signature is `render(template, params, caps: &[Capability])`, so the enum has to exist now even though chunk 5 only ever passes `&[]`. Variants and `marker_tag()` carry `#[allow(dead_code)] // populated by chunk 6` until chunk 6's CLI parser starts constructing them. The two-mode evaluator (drop-region vs. drop-markers) is fully implemented and unit-tested in chunk 5, so chunk 6 only has to wire the parser.
- **`process_caps` is a streaming line-walker.** Avoids regex / a real templating engine (RFC § Dependencies forbids both). Newline detection is one-shot (`\r\n` if present anywhere in the input, else `\n`) and the trailing newline is preserved iff the input had one. Mismatched markers (nested opens, orphaned closes, inline content) are emitted verbatim so downstream compilers fail loudly rather than silently producing partial files -- captured as design notes in the function docs.
- **Atomic refusal-to-overwrite.** `init::core::scaffold` walks every target path *before* creating any directory or writing any byte; if any target exists it returns `InvalidProject` immediately. This guarantees the "rerun produces no half-baked project" property the RFC's "one command, working project" promise depends on. Verified by the `scaffold_refuses_to_overwrite_existing_files` test (asserts the pre-existing colliding file is untouched and no other files were written).
- **`scaffold` creates the project dir if missing.** `vectis init Counter --dir /tmp/scratch/new-project` should bring `new-project` into being, not require the user to `mkdir` it first. `fs::create_dir_all(project_dir)` runs after the existence check (so the existence check runs against a path that may not exist yet -- harmless because `Path::exists()` returns false for non-existent paths) and before any per-file write. PascalCase validation runs even earlier so an invalid name doesn't create the directory.
- **MANIFEST self-check fragility.** The chunk-3a recipe (`diff <(ls templates/vectis/core | grep -v ^MANIFEST.md$ | sort) <(awk -F'\`' '/^\| \`[a-z]/ { print $2 }' MANIFEST.md | sort)`) breaks twice on this workstation: (1) the user's `ls` alias adds `-lpa`, polluting the LHS with file metadata rows -- worked around with `command ls -1`; (2) the awk pattern matches the cap-marker reference table rows (`http`, `kv`, `time`, `platform`, `sse`) along with the file-mapping rows -- worked around with `grep -v -E '^(http|kv|time|platform|sse)$'`. Captured here for chunks 6+ that touch the MANIFEST. The actual diff passes with both fixes.

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

Verification result (this chunk's commit):

- `vectis init Counter --dir /tmp/vectis-7-check --caps http --shells ios` exited 0; `assemblies.ios.build_steps` shows `typegen`, `package`, `xcode` all `passed: true`.
- `xcodebuild ... build` "Build Succeeded" against a created simulator (`xcrun simctl create "iPhone 15 RFC5" "iPhone 16" com.apple.CoreSimulator.SimRuntime.iOS-26-0` -> `id=AF07D1F0-57B1-4248-A551-76876983CF77`); chunk's `name=iPhone 15` does not resolve under Xcode 26.4 because that simulator no longer ships in the default install (chunk 9 should use `generic/platform=iOS Simulator`).
- Required environment fix: bumped local cargo-swift install from 0.9.0 to 0.11.0 to make `make package` produce a Swift package whose binary target matches the modulemap module name. Embedded `cargo_swift = "0.9"` pin in `embedded/versions.toml` was NOT touched (out of scope for chunk 7); chunks 4 + 11 own the bump.
- `cargo build --release -p vectis-cli` ✅, `cargo test -p vectis-cli` ✅ (68 tests, +5 vs chunk 6), `cargo clippy --all-targets -- -D warnings` ✅, `make checks` ✅.

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

- `init::run` dispatches `--shells android` to `init::android::scaffold` and merges its files + `BuildStep`s into `assemblies.android` (same envelope shape as `assemblies.ios`).
- New engine helper `templates::substitute_path_with(target, params, android_package_path: Option<&str>)` extends chunk-7's `substitute_path` with optional `__ANDROID_PACKAGE_PATH__` substitution (applied **before** `__APP_NAME_LOWER__` / `__APP_NAME__`). iOS callers pass `None`; Android callers compute the package path (`.` -> `/`) and pass `Some(...)`.
- `templates::android::TEMPLATES` mirrors the chunk-3c MANIFEST: 19 rows, each carrying an `IncludeWhen::{Always, AnyOf(&[Capability])}` whole-file predicate. Today only `network_security_config.xml` is `AnyOf(&[Http, Sse])`.
- Android version placeholders (`__AGP_VERSION__`, `__KOTLIN_VERSION__`, `__COMPOSE_BOM_VERSION__`, `__KTOR_VERSION__`, `__KOIN_VERSION__`) substituted unconditionally for every scaffold; `__ANDROID_NDK_VERSION__` is patched in by `init::android::run_pipeline` after disk detection (`$ANDROID_HOME/ndk/<version>/`, highest lexical version) so unit tests don't depend on a per-machine NDK install.
- Hermetic post-processing: `local.properties` is written from `$ANDROID_HOME` (or `Internal` error if unset); `org.gradle.java.home=<path>` is appended to `gradle.properties` when `/usr/libexec/java_home -v 21` succeeds (macOS-only; no-op elsewhere).
- Build pipeline (`gradle wrapper --gradle-version <pin>` -> `make build` -> `./gradlew :app:assembleDebug`) reuses chunk-7's `BuildStep` shape.
- Test isolation: unit tests that mutate `ANDROID_HOME` are serialised via a static `OnceLock<Mutex<()>>` (`android_home_lock`) inside `init::android::tests` to avoid races under `cargo test`'s parallel runner.
- **Gradle 9.x worked around inside the CLI** (no PATH munging required): `init::android::bootstrap_wrapper` runs `gradle wrapper --gradle-version <pin>` in a fresh empty scratch dir (containing only an empty `settings.gradle.kts`) and copies the resulting `gradlew{,.bat}` + `gradle/wrapper/*` files into the project. Avoids loading the `rust-android-gradle = 0.9.6` plugin (which calls `setFileMode(Integer)`, removed in Gradle 9.x) during the wrapper task. Verified end-to-end against system Gradle 9.4.1.
- **Chunk 8 verification ran from the repo-root cwd** (no `cd /tmp/...` mid-flow), so the macOS `/tmp`-rotation hazard that bit chunks 6/7 did not occur. All commands -- `cargo build/clippy/test`, `vectis init` (every `build_steps[*].passed = true`), and `./gradlew :shared:cargoBuild :app:assembleDebug` (`BUILD SUCCESSFUL in 1s`) -- passed end-to-end.

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

- **Branching policy — single linear branch.** Initial chunks experimented with one branch per chunk (`vectis-cli-chunk-2-prerequisites`, `vectis-cli-chunk-3a-templates-core`). After three chunks this had already produced three live branches and was on track to produce ~13, with no realistic merge story (each chunk depends on the previous, so they would have been a linear stack of PRs that re-rebase on every push). Consolidated all completed work onto `vectis-cli` via fast-forward and deleted the per-chunk branches. Going forward every chunk commits directly to `vectis-cli` (one commit per chunk, ordered by the dependency graph). "How to Use This File" step 4 was rewritten to match. The original Chunk 1 ref-name collision (`vectis-cli/chunk-1-skeleton` vs. the `vectis-cli` parent ref) is now moot under this policy.
- **Chunk 1 — Stub return shape.** The chunk text specified the `VectisError` variant list (no `NotImplemented`) and the stub JSON shape (`{"error": "not_implemented", "command": "<name>"}`). To reconcile these without duplicating `println!`/`exit` in each handler, introduced `CommandOutcome::{Success, Stub}` in `main.rs`. Stubs return `Ok(CommandOutcome::Stub { command })`; the dispatcher renders the JSON and exits 1. Real handlers in later chunks switch to `CommandOutcome::Success(value)` with no other dispatch changes.
- **Chunk 2 — Args plumbing brought forward.** Chunks 5/7/8/9/10/11 each carried a deferred note to "pass `&XxxArgs` through to the handler". The chunk 2 prereq check needs `args.shells` (init), `args.dir` (verify), `args.platform` (add-shell), and `args.verify` (update-versions), so this plumbing had to land now: the Args structs in `main.rs` were promoted to `pub(crate)` and every handler now accepts `args: &XxxArgs`. Status-row notes on the affected later chunks were updated to reflect that only the `Stub` -> `Success` transition remains for them.
- **Chunk 2 — Per-field `#[allow(dead_code)]` over crate-wide.** clap derive populates every Args field, but only a subset are read by the chunk-2 handlers. Under the existing `-D warnings` clippy gate, the unused fields fail the build. Chose per-field `#[allow(dead_code)] // consumed by chunk N` annotations over a blanket `#![allow(dead_code)]` so the lint stays effective elsewhere; later chunks remove the annotation as they start reading each field.
- **Chunk 2 — Tool name vs check-command labelling.** RFC § Workstation Requirements names some tools by category (e.g. "Xcode + Command Line Tools", "Android SDK (`$ANDROID_HOME`)"). The `tool` field of the JSON payload uses flat machine identifiers (`xcode`, `android-sdk`, `rustup-android-targets`, `android-ndk`); the `check` field carries the exact RFC display string. This keeps `tool` parseable while preserving the RFC's user-visible commands.
- **Chunk 3a — Flat source filenames + MANIFEST.md mapping.** Templates under `templates/vectis/core/` use flat filenames (`workspace-cargo.toml`, `shared-cargo.toml`, `lib.rs`, ...), with `MANIFEST.md` recording the source→target path mapping. This avoids a duplicate folder hierarchy under `templates/`, keeps `include_str!` paths short for chunk 5, and lets the engine treat target paths as data rather than as walked directory layout. Chunks 3b and 3c should adopt the same convention.
- **Chunk 3a — Capability-version placeholders.** RFC's placeholder table covers the always-on placeholders (`__CRUX_CORE_VERSION__`, `__FACET_VERSION__`, `__UNIFFI_VERSION__`, `__SERDE_VERSION__`) but not the per-capability versions. Added `__CRUX_HTTP_VERSION__`, `__CRUX_KV_VERSION__`, `__CRUX_TIME_VERSION__`, `__CRUX_PLATFORM_VERSION__` inside CAP-fenced regions of `workspace-cargo.toml`. Chunks 4-6 absorb the impact: chunk 4's `Versions` struct already exposes all five Crux crate fields; chunk 6's engine substitutes them when their cap is selected.
- **Chunk 3a — `__ANDROID_PACKAGE__` referenced from core's codegen binary.** `shared/src/bin/codegen.rs` uses the placeholder as the Kotlin package namespace. Chunk 5 (core, render-only) must substitute it for every render even when no Android shell is requested. The default `com.vectis.<lower app name>` (per RFC § CLI Surface § `vectis init`) is unambiguous: it labels generated Kotlin types, never an installed Android package.
- **Chunk 3a — `thiserror = "2"` is now an optional dep behind `uniffi`/`wasm_bindgen`.** The pre-RFC reference docs use `#[derive(thiserror::Error)]` on `CoreError` without listing `thiserror` in `[dependencies]`. The skill agents must have been adding it ad hoc on each scaffold. Pinning it here removes a recurring agent error mode.
- **Chunk 3b — Path-segment placeholder substitution.** iOS targets like `iOS/__APP_NAME__/__APP_NAME__App.swift` substitute to `iOS/Counter/CounterApp.swift` -- the placeholder appears twice in a single path, once as a directory segment and once as a file-name prefix. Chunk 3a's core targets were all static (`Cargo.toml`, `shared/src/app.rs`, ...) so this didn't surface there. The convention adopted for chunk 7's engine: run placeholder substitution over the constructed target path (string-level) before opening the file for write. The on-disk template layout stays flat regardless. Status row on chunk 7 carries the explicit instruction.
- **Chunk 3b — `__APP_NAME_LOWER__` is engine-derived, not a CLI flag.** The RFC's placeholder table lists it but its only consumers in the iOS templates (`bundleIdPrefix`, per-config `PRODUCT_BUNDLE_IDENTIFIER`) want the lowercase form of `--app-name` with no other transformation. Rather than adding a `--app-name-lower` flag, chunk 7 derives it via `args.app_name.to_lowercase()` and adds it to the `Params` struct. Chunk 3a's core templates didn't use it; chunk 3c's Android templates will (Android package, etc.) so the field stays in `Params` for both shells.
- **Chunk 3b — VectisDesign / Inject deliberately left out of the iOS baseline.** Both appear in the `ios-writer` reference docs but pose problems for a "one command, working project" guarantee: `VectisDesign` lives at `design-system/ios/` and is produced by a separate writer skill (may not exist when `vectis init` runs); `Inject` requires a network SPM resolve plus a per-developer InjectionIII install. The chunk-3b templates ship without either; the writer skills layer them in during Update Mode when they detect them. Re-instate paths and rationale captured in `templates/vectis/ios/MANIFEST.md` § Design system / Inject.
- **Chunk 5 — Workspace lint priorities + cargo-group allowlist.** Chunk 3a's verification ran `cargo check` only; chunk 5's added `cargo clippy --all-targets -- -D warnings` gate exposes that the workspace `[workspace.lints.clippy]` block (lifted verbatim from the original skill reference) puts lint *groups* (`all`, `nursery`, `pedantic`, `cargo`) at the same priority as per-lint overrides, which is itself a hard error under `-D warnings` (`clippy::lint_groups_priority`). The same gate fires `cargo_common_metadata` (the scaffolded `shared` crate is internal -- no description/license/repository) and `multiple_crate_versions` (Crux's transitive deps include `syn 1` + `syn 2` and similar duplicates the user can't dedupe). Patched in `templates/vectis/core/workspace-cargo.toml`: lint groups now use `{ level = "warn", priority = -1 }`, and `cargo_common_metadata` / `multiple_crate_versions` are explicit `allow`. The fix lives in chunk 3a's territory but blocks chunk 5's gate, so it lands here -- chunks 6/7/8 inherit the patched lints automatically.
- **Chunk 5 — Embedded `TEMPLATES` slice instead of runtime MANIFEST parsing.** The chunk text said the engine could "read `templates/vectis/core/MANIFEST.md`'s source→target mapping (or an embedded copy of it)". Encoded as a `&'static [CoreTemplate]` slice with `include_str!` per entry (one row per file, target path declared inline). The MANIFEST stays the human-facing source of truth; `templates::core::tests::registry_matches_rfc_core_file_count` pins the slice size at 13 so any new template forces a manifest update at the same time. Same convention recommended for chunks 7/8 (`templates::ios::TEMPLATES`, `templates::android::TEMPLATES`).
- **Chunk 5 — Render-only guards `--caps` and `--shells` rather than silently dropping them.** The chunk-5 scaffold is core, render-only. The handler accepts an empty `--caps "" --shells ""` (so build orchestration that always passes the flags doesn't break) but rejects any non-empty value with a structured `InvalidProject` pointing at the owning chunk (6 for `--caps`, 7/8 for `--shells`). This is preferred over silent drop because the user otherwise gets a project missing the assemblies they explicitly requested with no diagnostic. Chunks 6/7/8 should *replace* the guard with their respective dispatch -- not add a new branch alongside it.
- **Chunk 3b — Cargo-swift / uniffi tooling drift caught during verification.** The chunk-3a/4 pinned versions (`cargo_swift = "0.9"` shipping uniffi_bindgen 0.29.1, runtime `uniffi = "=0.29.4"`) produce a Swift `Shared` package whose `import sharedFFI` fails to resolve `RustBuffer` and friends under Xcode 16 / Swift 6. The iOS templates themselves are clean (typecheck against synthetic FFI stubs in render-only / HTTP-only / all-caps profiles all pass). Logged as a chunk-7 first-task to-do because it blocks `make package` -> `xcodebuild` end-to-end; possible fixes (bump cargo-swift, tighten uniffi pin, or patch the generated `shared.swift`) all live under chunk 7's scope. Not blocking chunk 3b because the chunk's verification is explicitly partial ("full pipeline verification arrives in chunk 7").
- **Chunk 7 — Cargo-swift fix landed as a tooling bump, not a versions.toml bump.** The chunk-3b drift turned out to be a binary-target/module-name mismatch in cargo-swift 0.9.0's bindgen output: the binary target is named `RustFramework` while the inner modulemap declares `sharedFFI`, and Xcode 26 / Swift 6 cannot resolve `import sharedFFI` against that. cargo-swift 0.11.0 emits the binary target as `sharedFFI` (matching the module), which fixes the import without any patch on the generated swift -- so option (a) from the chunk-7 status row (bump cargo-swift to ≥ 0.10) is the right choice. The bump itself touches `embedded/versions.toml` (chunk 4) and the `update-versions` registry plumbing (chunk 11), which are explicitly outside chunk 7's in-scope list ("How to Use This File" rule 7: "do not expand scope"). So chunk 7 stayed in scope by upgrading the *local cargo-swift install* (which is a per-developer prereq, like xcodegen or xcbeautify) and recording the embedded-pin bump as a required action under the chunk 4 + chunk 11 notes. The chunk-7 verification command therefore passes today on any machine with cargo-swift ≥ 0.10 and fails (at `xcodebuild`, not `make package`) on machines with cargo-swift 0.9; this is the same situation as a missing xcodegen install -- prereq-shaped, not template-shaped.
- **Chunk 7 — `iPhone 15` is no longer a default simulator under Xcode 26.4.** The chunk-7 verification command pins `-destination 'platform=iOS Simulator,name=iPhone 15'`, but the iPhone 15 runtime is no longer auto-installed (only iPhone 16 / 16 Pro variants ship in the default Xcode 26.4 download). Worked around by creating a custom simulator (`xcrun simctl create "iPhone 15 RFC5" "iPhone 16" com.apple.CoreSimulator.SimRuntime.iOS-26-0`) and using `-destination 'id=<UDID>'` instead. Chunk 9's verify pipeline should switch to `-destination 'generic/platform=iOS Simulator'` so the build is hermetic across Xcode/simulator combinations (no name-based pin to rot).
- **Chunk 11 — Registry probes are keep-on-failure, not abort-on-failure.** The RFC prose on `vectis update-versions` is silent on how to handle a single registry failure inside an otherwise-healthy run. The obvious options are (a) abort the whole command with `VectisError::Internal` or (b) keep the current pin for the failing field, record a diagnostic, and continue. Chose (b): a flaky Maven Central mirror (observed twice during chunk-11 verification: ktor-bom + kotlin-stdlib timeouts on `search.maven.org/solrsearch`) should not force the user to re-run the whole command. Each per-field failure appends a string to the output's `errors[]` array and the field surfaces as "unchanged" in the diff. The user can re-run to retry the failing queries. A whole-command abort (`VectisError::Internal`) still happens for unrecoverable states -- `$HOME` unset with no `--version-file`, or a filesystem write failure.
- **Chunk 11 — `--verify` matrix is core-only, not core+ios+android.** The RFC's `--verify` prose ("scaffold a temporary project with the proposed pins") doesn't specify shell coverage. Ran the matrix core-only for two reasons: (1) shells vendor the same Crux/uniffi pins the core does, so multiplying by 3 for ios+android gives little additional coverage of the pin set; (2) each shell build is ~60-120s on first run, so core-only (~4 combos × 30s) fits inside a reasonable interactive budget. If a future Crux release changes shell-specific pins (e.g. a new Android Koin API surface), extend `update_versions::matrix::CAP_MATRIX` to include shell variants.
- **Chunk 11 — Embedded uniffi/cargo-swift pair NOT bumped.** Both chunks 4 and 7 flagged that the embedded `uniffi = "=0.29.4"` + `cargo_swift = "0.9"` pair should bump to `=0.31.0` + `0.11` because cargo-swift 0.9's bindgen names the binary target `RustFramework` while the modulemap declares the module `sharedFFI` (Xcode 26+/Swift 6 cannot resolve the import). Chunk 11 attempted the bump, re-ran the chunk-9 verification matrix (per that chunk's Notes instruction), and discovered it breaks `codegen kotlin` on a fresh scaffold: `crux_core 0.17.0`'s `cli` feature pins `uniffi_bindgen = "=0.29.4"`, and `templates/vectis/core/codegen.rs` calls `crux_core::cli::bindgen(...)` for Kotlin binding generation. Mixing a 0.31 runtime in `shared` with a 0.29 bindgen fails with `unresolved module path shared::ffi`. There is no version of Crux that ships bindgen 0.31 today, so the bump is impossible without a template surgery on `codegen.rs` that belongs in chunk 13. Reverted the embedded pin back to `0.29.4` / `0.9`; the Xcode 26+ iOS concern is addressed by the *local* cargo-swift install (see chunk-7 Notes) -- a per-developer prereq like xcodegen / xcbeautify. The rationale is inlined as a multi-line comment in `crates/vectis-cli/embedded/versions.toml` so this does not need rediscovering.
