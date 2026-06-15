# Vectis build — Android shell (write + verify)

Loaded by [../../build.md](../../build.md) when `android` is in the platform set (carried from `project.yaml.platforms` via `proposal.md ## Platforms`). The composition validation gate ([../composition.md](../composition.md)) MUST have passed first.

Compose patterns, Crux Android shell anatomy, Kotlin token templates, and design-system integration depth live in [`../../../references/android/`](../../../references/android/).

## Mode detection

Inspect `${ANDROID_SHELL_DIR}/app/src/main/java/<package>/Core.kt`:

- Missing → **create mode**: scaffold with `specify tool run vectis -- scaffold android <APP_NAME> [--caps <csv>] [--android-package <package>]`, then enter pre-flight (see § Verify below), then update mode.
- Present → **update mode**: diff core types against existing Kotlin code and apply targeted edits.

Spawn the writer sub-agent with `mode: create|update` and `skip_verification: true`; the dedicated verify sub-agent (§ Verify) runs afterward.

## Writer steps

1. **Read inputs.** `app.rs`, the regenerated `composition.yaml`, sibling `tokens.yaml` / `assets.yaml` when present, the `## Android Shell Requirements` section of `spec.md`, and the `## Android Shell Details` section of `design.md`. Extract App name, ViewModel / Effect / Event / Route variants, and the capability set.
2. **Build an inventory** of existing Kotlin code: effect handlers, ViewModel cases, screen composables, event dispatches, adapter clients (Ktor for HTTP / SSE, SharedPreferences for KV), DI modules (Koin when multiple non-Render effects are used).
3. **Diff Rust core types vs Kotlin inventory** by category (Effect → ViewModel → view-fields → Event → Route) and emit a summary edit plan.
4. **Apply changes.** Expand or strip CAP blocks in `Core.kt`, `AndroidManifest.xml`, and Gradle build files. Add or remove screen composables for each ViewModel variant under `Android/app/src/main/java/com/vectis/<app>/ui/screens/`. Update the root `when` over the `ViewModel` enum. Dispatch new `Event`s through `Core.update(...)`. Emit one named composable per `component: <slug>` directive in `composition.yaml` under `Android/app/src/main/java/com/vectis/<app>/ui/components/` (PascalCased — `tab-bar` → `TabBarComponent.kt`), with props inferred from variation across instances. When `CATALOG_PATH` exists, every `confirmed` catalog entry referenced by a `component:` directive in `composition.yaml` produces a shared composable file. Per-screen composables reference the shared component instead of inlining the layout. When the catalog is absent, component files are still emitted for any `component:` directives in `composition.yaml` (backward-compatible behaviour), but the catalog is the authoritative driver for which slugs to factor. **Retroactive factoring (B7).** When a component newly `confirmed` this build is referenced by baseline screens *outside the current slice's domains* (composition.md Step 6a attached the `component: <slug>` directive to those prior screens via `delta.modified`), generate the shared `<Slug>Component.kt` **and** refactor those prior screens' generated composables to consume it in place of the inlined layout — the writer runs in `update` mode against the live shell tree, so editing prior-slice composables is in scope. The refactor is behaviour-preserving because the skeletons are structurally identical by construction; the verify loop below catches any regression. Idempotent: a prior screen already consuming the shared composable needs no further edit. Patterns: [`android/shell-pattern.md`](../../../references/android/shell-pattern.md), [`android/view-patterns.md`](../../../references/android/view-patterns.md).
5. **Refresh generated UI surfaces.** Regenerate shell-local theme code under `Android/app/src/main/java/com/vectis/<app>/ui/theme/` (Material 3 fallback when `tokens.yaml` is absent — full templates: [`android/token-templates.md`](../../../references/android/token-templates.md)), and drawable / mipmap resources under `Android/app/src/main/res/`. Design-system integration depth: [`android/design-system-integration.md`](../../../references/android/design-system-integration.md).
   - **Materialize gate.** `specify slice build --phase prepare` runs `vectis materialize assets` for in-scope ids with missing `sources.android` pins; operators may also run `specify tool run vectis -- materialize assets` manually after editing canonical masters under `design-system/assets/`. Committed trees under `design-system/assets/exports/android/` are the copy source — never read canonical `source:` SVG/PNG at write time.
   - **Copy from `exports/android/`.** Resolve the effective `assets.yaml` (slice-local `.specify/slices/<name>/assets.yaml` when present, else `design-system/assets.yaml`). For each composition-referenced asset id:
     - `kind: symbol` — emit `Icons.Default.<glyph>` (or extended icon set) at the call site; **no** `res/` copy.
     - `role: app-icon` — copy the `assets/exports/android/app-icon/` tree (`mipmap-*/`, `mipmap-anydpi-v26/`, `drawable-*/ic_launcher_foreground.png`, `values/ic_launcher_background.xml`) into matching `res/` locations.
     - `role: icon` or `decorative` + `kind: vector` — copy `assets/exports/android/drawable/<id_snake>.xml` into `res/drawable/<id_snake>.xml`.
     - `role: illustration` + `kind: vector` — copy each `assets/exports/android/drawable-<density>/<id_snake>.png` into `res/drawable-<density>/<id_snake>.png`.
     - `kind: raster` (operator-pinned per-density masters) — copy pinned bucket files into matching `res/drawable-<density>/` paths per `sources.android`.
   - **Render by `kind`.** `vector` / `raster` ids emit `painterResource(R.drawable.<id_snake>)` from copied drawables; never substitute Material Icons for non-`symbol` entries.
6. **Update build configuration** (`libs.versions.toml`, `build.gradle.kts`, manifest permissions, `network_security_config.xml`) to match the changed capability set. Remove any legacy `:vectis-design` Gradle module references — there is no shared Compose module; the writer emits shell-local theme + drawable code exclusively. Replace any stale `import com.vectis.design.*` with `import com.vectis.<app>.ui.theme.*`.
7. **UniFFI bridging contract.** The `Application` class MUST set `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` before any UniFFI class is loaded — without this the app fails with `UnsatisfiedLinkError` on launch. Imports for generated FFI types follow `import com.vectis.<app>.*` (not `com.vectis.design.*`). Rethrow `CancellationException` from coroutines — never swallow it.

## Hard rules

Full set at [`hard-rules-android.md`](../../../references/hard-rules-android.md). Highlights:

- Java 21 only — Java 25+ environments hit `IllegalArgumentException` in AGP; pin `org.gradle.java.home` in `gradle.properties`.
- Always include `@Preview` blocks for new composables.
- Coroutine cancellation MUST rethrow `CancellationException`.

## Verify (max 3 iterations)

Spawn this loop in its own sub-agent with `ANDROID_SHELL_DIR`. The sub-agent returns `status`, `iterations_used`, and any unresolved errors. The verify sub-agent is the **sole source of truth** for Android shell checkboxes in `tasks.md` — never mark an Android task complete or report success unless `make build`, `gradlew :shared:cargoBuild`, and `gradlew :app:assembleDebug` have actually run and passed in the verify loop.

### Pre-flight (fail fast on misconfiguration)

Run these before entering the loop. If any check fails, report the missing prerequisite and mark Android verification as **pending** rather than entering the build loop.

```bash
test -f "${ANDROID_SHELL_DIR}/local.properties"
grep -q "sdk.dir" "${ANDROID_SHELL_DIR}/local.properties"
grep -q "org.gradle.java.home" "${ANDROID_SHELL_DIR}/gradle.properties"  # Must point to Java 21.
rustup target list --installed | grep android
```

### Gradle wrapper bootstrap

Before any `./gradlew` invocation, verify `gradlew` exists and is executable and `gradle/wrapper/gradle-wrapper.jar` is present. If the wrapper is missing, bootstrap from a minimal init project:

```bash
tmp_dir=$(mktemp -d)
cd "$tmp_dir" && gradle wrapper && cd -
cp "$tmp_dir/gradlew" "$tmp_dir/gradlew.bat" "$ANDROID_SHELL_DIR/"
cp -r "$tmp_dir/gradle" "$ANDROID_SHELL_DIR/"
chmod +x "$ANDROID_SHELL_DIR/gradlew"
rm -rf "$tmp_dir"
```

If `gradle` itself is not installed, report the prerequisite (`brew install gradle`) and mark Android verification as pending.

### Build loop

```bash
cd "$ANDROID_SHELL_DIR" && make build                       # 1. Type generation + cross-compile.
cd "$ANDROID_SHELL_DIR" && ./gradlew :shared:cargoBuild     # 2. Rust library build.
cd "$ANDROID_SHELL_DIR" && ./gradlew :app:assembleDebug     # 3. APK build.
```

If a step fails, fix the issue and re-run. Repeat until all three checks pass or 3 iterations are exhausted. Stop early on identical-output regressions. If still failing after 3 iterations: **stop** and escalate. Java 25+ environments hit `IllegalArgumentException`; the fix is pinning `org.gradle.java.home` to Java 21 in `gradle.properties`.

## Worked examples

- [`examples/android/01-simple-counter.md`](../../../references/examples/android/01-simple-counter.md) — minimal Core.kt + Application.kt + root composable.
- [`examples/android/02-http-counter.md`](../../../references/examples/android/02-http-counter.md) — Ktor HTTP capability, coroutine scope, suspending effect handlers.
