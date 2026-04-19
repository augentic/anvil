# Android Assembly Template Manifest

Source-of-truth mapping for the chunk-3c templates. The Rust template engine
arriving in chunk 8 reads this list (or an equivalent embedded copy) to know
which template file goes to which on-disk path, and which placeholders /
capability markers it must process.

## Path mapping

Source filenames are flat -- no nested directories under
`templates/vectis/android/`. Nested target paths (especially the
`Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/...` segment) are produced
by the template engine, never by the on-disk layout of the templates directory.
This keeps `include_str!` paths short and matches the convention established
in `templates/vectis/{core,ios}/MANIFEST.md`.

| Source (this dir)              | Target (rendered project)                                                           |
| ------------------------------ | ----------------------------------------------------------------------------------- |
| `Makefile`                     | `Android/Makefile`                                                                  |
| `gitignore`                    | `Android/.gitignore`                                                                |
| `root-build.gradle.kts`        | `Android/build.gradle.kts`                                                          |
| `settings.gradle.kts`          | `Android/settings.gradle.kts`                                                       |
| `gradle.properties`            | `Android/gradle.properties`                                                         |
| `libs.versions.toml`           | `Android/gradle/libs.versions.toml`                                                 |
| `app-build.gradle.kts`         | `Android/app/build.gradle.kts`                                                      |
| `shared-build.gradle.kts`      | `Android/shared/build.gradle.kts`                                                   |
| `AndroidManifest.xml`          | `Android/app/src/main/AndroidManifest.xml`                                          |
| `themes.xml`                   | `Android/app/src/main/res/values/themes.xml`                                        |
| `network-security-config.xml`  | `Android/app/src/main/res/xml/network_security_config.xml` (only when `http`/`sse`) |
| `Application.kt`               | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/__APP_NAME__Application.kt`     |
| `MainActivity.kt`              | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/MainActivity.kt`                |
| `Core.kt`                      | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/core/Core.kt`                   |
| `LoadingScreen.kt`             | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/screens/LoadingScreen.kt`    |
| `HomeScreen.kt`                | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/screens/HomeScreen.kt`       |
| `Color.kt`                     | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Color.kt`              |
| `Theme.kt`                     | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Theme.kt`              |
| `Type.kt`                      | `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Type.kt`               |

Total: 19 files (matches RFC § File Manifests § Android Assembly).

The `__APP_NAME__` and `__ANDROID_PACKAGE_PATH__` segments in target paths are
substituted by the engine when writing each file, the same as inside file
contents. Path-segment substitution applies to both directory and file-name
positions (e.g. `__APP_NAME__Application.kt` becomes `CounterApplication.kt`).

`__ANDROID_PACKAGE_PATH__` is **derived**, not supplied -- the engine in
chunk 8 computes it by replacing `.` with `/` in `__ANDROID_PACKAGE__` at
file-write time. It does not appear in the placeholder substitution table the
engine carries in memory; it only ever appears in target-path strings.

The Gradle wrapper files (`gradlew`, `gradlew.bat`,
`gradle/wrapper/gradle-wrapper.jar`, `gradle/wrapper/gradle-wrapper.properties`)
are intentionally **not** templates. They are produced by chunk 8 invoking
`gradle wrapper --gradle-version <pin>` after the Gradle config files exist.
The same applies to `local.properties` (per-developer; carries `sdk.dir` from
`$ANDROID_HOME`).

## Placeholder reference

Always present in the Android templates:

| Placeholder              | Example value         | Files                                                                                                             |
| ------------------------ | --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `__APP_NAME__`           | `Counter`             | `Makefile`, `settings.gradle.kts`, `AndroidManifest.xml`, `themes.xml`, `Application.kt`, `HomeScreen.kt` (preview), and the `__APP_NAME__Application.kt` file-name path |
| `__ANDROID_PACKAGE__`    | `com.vectis.counter`  | `Makefile`, `app-build.gradle.kts`, `shared-build.gradle.kts`, every Kotlin source `package` line, and every cross-package `import`                                |

`__APP_STRUCT__` is not used in the Android templates today (the generated
Kotlin types are namespaced by `__ANDROID_PACKAGE__`, not by the app struct
name). `__APP_NAME_LOWER__` is not used either; `__ANDROID_PACKAGE__` already
encodes the lowercase form.

`__ANDROID_PACKAGE_PATH__` only appears in the target-path column above; it is
not substituted into file contents.

Capability-version placeholders -- only meaningful when their CAP marker is
kept (see "Cap-marker reference" below):

| Placeholder              | Example value | Files                  |
| ------------------------ | ------------- | ---------------------- |
| `__AGP_VERSION__`        | `8.13.2`      | `libs.versions.toml`   |
| `__KOTLIN_VERSION__`     | `2.3.0`       | `libs.versions.toml`   |
| `__COMPOSE_BOM_VERSION__`| `2026.01.01`  | `libs.versions.toml`   |
| `__KTOR_VERSION__`       | `3.4.0`       | `libs.versions.toml`   |
| `__KOIN_VERSION__`       | `4.1.1`       | `libs.versions.toml`   |
| `__ANDROID_NDK_VERSION__`| `30.0.14904198` | `shared-build.gradle.kts` |

Notes for chunk 4 / 8 / 11:

- **`__AGP_VERSION__` / `__KOTLIN_VERSION__` / `__COMPOSE_BOM_VERSION__` /
  `__KTOR_VERSION__` / `__KOIN_VERSION__` are not in the RFC's placeholder
  table.** They mirror the chunk-3a additions for `__CRUX_*_VERSION__` and
  cover the same need: chunk 4's `Versions::android` struct already carries
  `compose_bom`, `koin`, `ktor`, `kotlin`, `agp`, so chunk 8 substitutes them
  from there. Update the placeholder table in any future RFC revision.
- **The "Initial Version Pins" block in `rfcs/rfc-6-tasks.md` is stale
  versus the values that actually compile today.** The block lists
  `agp = "8.8.2"`, `kotlin = "2.1.10"`, `compose_bom = "2025.01.01"` -- the
  reference doc and the verified-working values used by chunk 3c are
  `8.13.2` / `2.3.0` / `2026.01.01`. Chunk 4 should either bump those defaults
  (preferred) or chunk 11 must do it before any project actually scaffolds.
  Documented further under "Verification deviations" below.
- **`__ANDROID_NDK_VERSION__` is also not in the RFC placeholder table or in
  the chunk-4 `Versions::android` substruct.** The chunk-8 engine should
  detect the installed NDK from `$ANDROID_HOME/ndk/<version>/` and substitute
  it (or fall back to a default pin and require the developer to install that
  NDK). This avoids hard-coding an NDK version that may not be installed.

## Cap-marker reference

Capability-conditional regions follow the same convention as core/iOS (paired
`<<<CAP:<name>` / `CAP:<name>>>>` lines, each on their own line). The engine
treats the entire region (markers and content inclusive) as removable when the
cap is not selected, and drops only the marker lines (preserving content) when
the cap is selected.

| Cap        | Files                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| `http`     | `libs.versions.toml`, `app-build.gradle.kts`, `AndroidManifest.xml`, `Core.kt` |
| `kv`       | `Core.kt`                                                                      |
| `time`     | `Core.kt`                                                                      |
| `platform` | `Core.kt`                                                                      |

Notes for chunk 8:

- **`network-security-config.xml` is whole-file conditional on `http` or
  `sse`.** It has no CAP markers inside; its inclusion in the rendered
  project is decided by chunk 8 outside the file. The MANIFEST records this
  in the path-mapping column. Chunk 8's engine needs a "skip this whole file
  if cap missing" predicate (the chunk-8 status note already calls this out).
- **`koin-bom` and `ktor-*` lines are gated only on `<<<CAP:http`.** The
  reference docs use them for any non-render cap, but the deterministic
  baseline only wires the HTTP path. Chunk 6 / chunk 8 / writer skills layer
  in DI for kv/time/platform/sse during Update Mode. Documented to keep the
  cap-marker semantics from chunk 3a/3b unchanged (no "any non-render cap"
  marker variant).
- **Cap arms inside `Core.kt` carry both the matching `is Effect.X -> ...`
  arm in `processRequest` and any helper functions / coroutine plumbing that
  arm needs**, all inside the same CAP marker. Kotlin `when` over a sealed
  interface is exhaustive, so adding a cap arm without the corresponding
  Effect variant in `app.rs` (or vice versa) is a compile error. The CAP
  markers in `Core.kt` mirror the ones in `templates/vectis/core/app.rs`
  exactly. The `http` block additionally adds `viewModelScope` / coroutine
  imports and the `resolveAndHandleEffects` helper -- they are not generic
  enough to live outside the marker.
- **`kv`, `time`, `platform` baseline arms are TODO stubs.** They bind
  `effect.value` to a suppressed-warning local and do nothing else (no
  `coreFfi.resolve(...)` call, no async plumbing). The deterministic baseline
  never emits these effects (the render-only update path only fires
  `render()`), so this is safe. The writer skills replace the stubs with real
  handlers in Update Mode. If chunk 6 wires `Event` variants that emit
  non-HTTP effects on init, those stubs will need to grow `coreFfi.resolve`
  plumbing similar to the HTTP arm.
- **The `sse` cap intentionally has no entry in `Core.kt` today.** Same
  reasoning as chunk 3b: `app.rs` doesn't declare an `Effect::Sse(...)`
  variant in the render-only baseline, so the Kotlin `Effect` enum has no
  `.sse` case to handle. When chunk 6 adds the Rust-side variant, this
  manifest, `libs.versions.toml`, `AndroidManifest.xml` (cleartext for SSE
  endpoints), and `Core.kt` all need matching `<<<CAP:sse` blocks.

## Design system / Koin DI

The chunk-3c templates deliberately omit `:vectis-design`, the Koin
`AppModule.kt`, and any `HttpClient.kt` / `SseClient.kt` / `KeyValueClient.kt`
classes from the deterministic baseline. Reasons mirror chunk 3b:

- **`:vectis-design`** lives at `design-system/android/` and is produced by
  the separate `design-system-writer` skill. It is not guaranteed to exist
  when `vectis init` runs, and the Android reference docs say "When
  `design-system/tokens.yaml` exists, ... include the generated Compose
  library" -- i.e. additive, not baseline.
- **Koin DI / per-cap helper classes** (Pattern 2 Core in
  `crux-android-shell-pattern.md`) introduce non-trivial dependencies and
  multi-file structure that the deterministic baseline does not need. The
  baseline uses Pattern 1 (Core extends `androidx.lifecycle.ViewModel`,
  `mutableStateOf` for view state) and inlines a stub HTTP handler inside
  `Core.kt` when the `http` cap is selected.

The writer skills layer in `:vectis-design`, Koin, and the per-cap helper
classes during Update Mode when they detect them. Chunk 12's skill rewrite
should keep that path working.

## Verification deviations

The chunk-3c chunk text gates this work on
`./gradlew :app:assembleDebug` against a render-only paired core. This is
expensive (NDK cross-compile across four ABIs) but it is the only assertion
that proves the templates produce a buildable Android shell. Notes on what
landed during verification:

- The `gradle.properties` template **omits** `org.gradle.java.home`. The
  reference doc pins it to `/Library/Java/JavaVirtualMachines/jdk-21.jdk/...`
  (a per-machine path) and warns Java 25+ breaks Gradle's Kotlin compiler.
  The chunk-3c templates rely on the developer's `JAVA_HOME` pointing at
  Java 21. Chunk 8 should consider auto-detecting Java 21 via
  `/usr/libexec/java_home -v 21` (macOS) or equivalent, and writing the line
  into `gradle.properties` at scaffold time so the project remains
  hermetic.
- The "Initial Version Pins" block in `rfcs/rfc-6-tasks.md` is stale for
  Android. The verification staging substituted the working values from the
  reference doc (`agp = "8.13.2"`, `kotlin = "2.3.0"`,
  `compose_bom = "2026.01.01"`, `ktor = "3.4.0"`, `koin = "4.1.1"`,
  `gradle = "8.13"` is fine). Chunk 4 should bump the Android defaults to
  match.
- `__ANDROID_NDK_VERSION__` was substituted from the locally-installed NDK
  (`$ANDROID_HOME/ndk/<version>/`) at verification time. Chunk 8 needs to
  decide whether the engine pins or detects this; pinning to a version that
  isn't installed yields a confusing "NDK not found" error from
  `rust-android-gradle`.

## Self-check

This manifest must list every file in `templates/vectis/android/`. CI can
enforce this trivially -- restrict the awk match to backtick-wrapped tokens
that look like file names so cap names (`http`, `kv`, ...) and placeholder
identifiers (`__APP_NAME__`, ...) from other tables don't pollute the
comparison:

```bash
diff \
  <(command ls -1 templates/vectis/android | grep -v '^MANIFEST.md$' | sort) \
  <(awk -F'`' '/^\| `[A-Za-z][A-Za-z._-]*`/ { print $2 }' templates/vectis/android/MANIFEST.md \
      | grep -E '\.[A-Za-z]+$|^Makefile$|^gitignore$' | sort -u)
```

Run the diff after adding or renaming a template file.
